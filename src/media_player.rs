//! Local-file playback with the Windows Media Foundation engine, never a browser.
//! Engine I/O and decoding are asynchronous. Only the active preview owns a player.
use crate::project_media::{MediaKind, Playable};
use std::path::Path;

/// Pass a Unicode filesystem path to MF directly. Percent-encoded file URLs
/// are treated as literal paths by some Windows source handlers (notably WAV).
pub fn source_path(path: &Path) -> Result<String, String> {
    let path = path.to_str().ok_or("Invalid Unicode media path")?;
    let path = if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
    };
    if path.contains('\0')
        || !(path.starts_with(r"\\")
            || (path.as_bytes().get(1) == Some(&b':')
                && path.as_bytes().get(2).is_some_and(|c| b"/\\".contains(c))))
    {
        return Err("An absolute Windows file path is required".into());
    }
    Ok(path)
}

#[cfg(windows)]
pub use native::Player;
#[cfg(not(windows))]
pub struct Player;
#[cfg(not(windows))]
impl Player {
    pub fn new(_: &egui::Context, _: &Playable) -> Result<Self, String> {
        Err("Windows media playback is unavailable on this platform".into())
    }
    pub fn show(&mut self, _: &mut egui::Ui) {}
}

pub fn time_label(seconds: f64) -> String {
    let seconds = if seconds.is_finite() {
        seconds.max(0.0) as u64
    } else {
        0
    };
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU32, Ordering},
        },
        time::{Duration, Instant},
    };
    use windows::{
        Win32::{
            Graphics::{Dxgi::Common::DXGI_FORMAT_B8G8R8X8_UNORM, Imaging::*},
            Media::MediaFoundation::*,
            System::Com::*,
        },
        core::{BSTR, Interface, implement},
    };

    struct Events {
        active: AtomicBool,
        error: AtomicU32,
        event: AtomicU32,
    }
    #[implement(IMFMediaEngineNotify)]
    struct Notify {
        ctx: egui::Context,
        events: Arc<Events>,
    }
    impl IMFMediaEngineNotify_Impl for Notify_Impl {
        fn EventNotify(&self, event: u32, param1: usize, _: u32) -> windows::core::Result<()> {
            self.events.event.store(event, Ordering::Relaxed);
            if event == MF_MEDIA_ENGINE_EVENT_NOTIFYSTABLESTATE.0 as u32 {
                // This event carries a borrowed OS event handle, not application data.
                unsafe {
                    windows_sys::Win32::System::Threading::SetEvent(param1 as _);
                }
                return Ok(());
            }
            if self.events.active.load(Ordering::Acquire) {
                if event == MF_MEDIA_ENGINE_EVENT_ERROR.0 as u32 {
                    self.events.error.store(param1 as u32, Ordering::Release);
                }
                // TIMEUPDATE is throttled by the engine; paused players have no timer.
                self.ctx.request_repaint_of(egui::ViewportId::ROOT);
            }
            Ok(())
        }
    }
    struct Library(windows_sys::Win32::Foundation::HMODULE);
    impl Drop for Library {
        fn drop(&mut self) {
            unsafe {
                windows_sys::Win32::Foundation::FreeLibrary(self.0);
            }
        }
    }
    type Startup = unsafe extern "system" fn(u32, u32) -> windows::core::HRESULT;
    type Shutdown = unsafe extern "system" fn() -> windows::core::HRESULT;
    type Attributes =
        unsafe extern "system" fn(*mut *mut std::ffi::c_void, u32) -> windows::core::HRESULT;
    struct Runtime {
        com: bool,
        mf: bool,
        shutdown: Shutdown,
        attributes: Attributes,
        _library: Library,
    }
    impl Runtime {
        fn new() -> windows::core::Result<Self> {
            use windows_sys::Win32::System::LibraryLoader::*;
            // MF can be absent on Windows N. Load only when opening a media preview,
            // from System32 only, so the terminal still starts without media components.
            let library = unsafe {
                LoadLibraryExW(
                    windows_sys::core::w!("mfplat.dll"),
                    std::ptr::null_mut(),
                    LOAD_LIBRARY_SEARCH_SYSTEM32,
                )
            };
            if library.is_null() {
                return Err(windows::core::Error::from_thread());
            }
            let library = Library(library);
            let missing = || {
                windows::core::Error::new(
                    windows::Win32::Foundation::E_NOTIMPL,
                    "Windows Media Foundation is unavailable",
                )
            };
            // SAFETY: these are the documented mfplat.dll ABI exports, owned by library.
            let (startup, shutdown, attributes): (Startup, Shutdown, Attributes) = unsafe {
                (
                    std::mem::transmute::<unsafe extern "system" fn() -> isize, Startup>(
                        GetProcAddress(library.0, windows_sys::core::s!("MFStartup"))
                            .ok_or_else(missing)?,
                    ),
                    std::mem::transmute::<unsafe extern "system" fn() -> isize, Shutdown>(
                        GetProcAddress(library.0, windows_sys::core::s!("MFShutdown"))
                            .ok_or_else(missing)?,
                    ),
                    std::mem::transmute::<unsafe extern "system" fn() -> isize, Attributes>(
                        GetProcAddress(library.0, windows_sys::core::s!("MFCreateAttributes"))
                            .ok_or_else(missing)?,
                    ),
                )
            };
            let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            if result.is_err() && result != windows::Win32::Foundation::RPC_E_CHANGED_MODE {
                result.ok()?;
            }
            let mut runtime = Self {
                com: result.is_ok(),
                mf: false,
                shutdown,
                attributes,
                _library: library,
            };
            unsafe {
                startup(MF_VERSION, MFSTARTUP_FULL).ok()?;
            }
            runtime.mf = true;
            Ok(runtime)
        }
        fn attributes(&self) -> windows::core::Result<IMFAttributes> {
            let mut raw = std::ptr::null_mut();
            unsafe {
                (self.attributes)(&mut raw, 3).ok()?;
                Ok(IMFAttributes::from_raw(raw))
            }
        }
    }
    impl Drop for Runtime {
        fn drop(&mut self) {
            unsafe {
                if self.mf {
                    let _ = (self.shutdown)();
                }
                if self.com {
                    CoUninitialize();
                }
            }
        }
    }
    pub struct Player {
        engine: IMFMediaEngine,
        events: Arc<Events>,
        bitmap: Option<IWICBitmap>,
        texture: Option<egui::TextureHandle>,
        frame_pixels: Vec<u8>,
        frame_size: [u32; 2],
        copied: Instant,
        _runtime: Runtime,
        kind: MediaKind,
        error: Option<String>,
        seek_preview: Option<f64>,
    }
    impl Player {
        pub fn new(ctx: &egui::Context, media: &Playable) -> Result<Self, String> {
            Self::create(ctx, media).map_err(|e| e.to_string())
        }
        fn create(ctx: &egui::Context, media: &Playable) -> windows::core::Result<Self> {
            let source = source_path(&media.path).map_err(|e| {
                windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, e)
            })?;
            let runtime = Runtime::new()?;
            let events = Arc::new(Events {
                active: AtomicBool::new(true),
                error: AtomicU32::new(0),
                event: AtomicU32::new(0),
            });
            let callback: IMFMediaEngineNotify = Notify {
                ctx: ctx.clone(),
                events: events.clone(),
            }
            .into();
            // SAFETY: COM/MF are initialized on this UI thread; interfaces remain owned here.
            let engine = unsafe {
                let attributes = runtime.attributes()?;
                attributes.SetUnknown(&MF_MEDIA_ENGINE_CALLBACK, &callback)?;
                attributes.SetUINT32(
                    &MF_MEDIA_ENGINE_VIDEO_OUTPUT_FORMAT,
                    DXGI_FORMAT_B8G8R8X8_UNORM.0 as u32,
                )?;
                let factory: IMFMediaEngineClassFactory =
                    CoCreateInstance(&CLSID_MFMediaEngineClassFactory, None, CLSCTX_INPROC_SERVER)?;
                factory.CreateInstance(0, &attributes)?
            };
            let player = Self {
                engine,
                events,
                bitmap: None,
                texture: None,
                frame_pixels: vec![],
                frame_size: [0, 0],
                copied: Instant::now() - Duration::from_secs(1),
                _runtime: runtime,
                kind: media.kind,
                error: None,
                seek_preview: None,
            };
            unsafe {
                player.engine.SetAutoPlay(false)?;
                player.engine.SetPreload(MF_MEDIA_ENGINE_PRELOAD_METADATA)?;
                player.engine.SetSource(&BSTR::from(source))?;
                player.engine.Load()?;
            }
            Ok(player)
        }
        pub fn diagnostics(&self) -> String {
            unsafe {
                format!(
                    "event={} error={} ready={} network={} duration={:?} detail={:?}",
                    self.events.event.load(Ordering::Relaxed),
                    self.events.error.load(Ordering::Relaxed),
                    self.engine.GetReadyState(),
                    self.engine.GetNetworkState(),
                    self.duration(),
                    self.engine.GetError().map(|e| e.GetExtendedErrorCode())
                )
            }
        }
        pub fn position(&self) -> f64 {
            unsafe { self.engine.GetCurrentTime() }
        }
        pub fn duration(&self) -> f64 {
            unsafe { self.engine.GetDuration() }
        }
        pub fn paused(&self) -> bool {
            unsafe { self.engine.IsPaused().as_bool() || self.engine.IsEnded().as_bool() }
        }
        pub fn play(&self) -> windows::core::Result<()> {
            unsafe { self.engine.Play() }
        }
        pub fn pause(&self) -> windows::core::Result<()> {
            unsafe { self.engine.Pause() }
        }
        pub fn seek(&self, seconds: f64) -> windows::core::Result<()> {
            unsafe { self.engine.SetCurrentTime(seconds) }
        }
        pub fn volume(&self, volume: f64) -> windows::core::Result<()> {
            unsafe { self.engine.SetVolume(volume.clamp(0.0, 1.0)) }
        }
        pub fn failed(&self) -> bool {
            self.events.error.load(Ordering::Acquire) != 0
        }
        pub fn has_frame(&self) -> bool {
            self.texture.is_some()
        }
        fn video_frame(&mut self, ctx: &egui::Context) -> windows::core::Result<()> {
            if self.copied.elapsed() < Duration::from_micros(33_333) {
                return Ok(());
            }
            self.copied = Instant::now();
            let mut timestamp = 0;
            // Retain S_FALSE: the projection's Result would otherwise hide "no new frame".
            let tick = unsafe {
                (self.engine.vtable().OnVideoStreamTick)(self.engine.as_raw(), &mut timestamp)
            };
            if tick != windows::core::HRESULT(0) {
                return Ok(());
            }
            let (mut width, mut height) = (0, 0);
            unsafe {
                self.engine
                    .GetNativeVideoSize(Some(&mut width), Some(&mut height))?;
            }
            if width == 0 || height == 0 {
                return Ok(());
            }
            if width > 16384
                || height > 16384
                || u64::from(width) * u64::from(height) > 64 * 1024 * 1024
            {
                return Err(windows::core::Error::new(
                    windows::Win32::Foundation::E_INVALIDARG,
                    "Video dimensions exceed preview limits",
                ));
            }
            let scale = (1920.0 / width.max(height) as f64).min(1.0);
            let size = [
                (width as f64 * scale).round().max(1.0) as u32,
                (height as f64 * scale).round().max(1.0) as u32,
            ];
            if self.frame_size != size {
                let factory: IWICImagingFactory = unsafe {
                    CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?
                };
                self.bitmap = Some(unsafe {
                    factory.CreateBitmap(
                        size[0],
                        size[1],
                        &GUID_WICPixelFormat32bppBGR,
                        WICBitmapCacheOnLoad,
                    )?
                });
                self.frame_pixels
                    .resize(size[0] as usize * size[1] as usize * 4, 0);
                self.frame_size = size;
            }
            let bitmap = self.bitmap.as_ref().unwrap();
            let destination = windows::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: size[0] as i32,
                bottom: size[1] as i32,
            };
            unsafe {
                self.engine
                    .TransferVideoFrame(bitmap, None, &destination, None)?;
                bitmap.CopyPixels(std::ptr::null(), size[0] * 4, &mut self.frame_pixels)?;
            }
            let pixels = self
                .frame_pixels
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| egui::Color32::from_rgb(p[2], p[1], p[0]))
                .collect();
            let image = egui::ColorImage::new([size[0] as usize, size[1] as usize], pixels);
            if let Some(texture) = &mut self.texture {
                texture.set(image, egui::TextureOptions::LINEAR);
            } else {
                self.texture = Some(ctx.load_texture(
                    "media-video-frame",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
            Ok(())
        }
        pub fn show(&mut self, ui: &mut egui::Ui) {
            let locale = crate::i18n::locale(ui.ctx());
            if self.failed() || self.error.is_some() {
                ui.colored_label(crate::chrome::WAITING, locale.text("无法播放此文件，编码不受支持、文件损坏或系统媒体组件不可用。", "Cannot play this file. Its codec may be unsupported, the file may be damaged, or Windows media components may be unavailable."));
                if let Some(error) = &self.error {
                    ui.weak(error);
                }
                return;
            }
            let duration = self.duration();
            let ready = duration.is_finite() && duration > 0.0;
            let mut action = Ok(());
            ui.horizontal_wrapped(|ui| {
                let paused = self.paused();
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new(locale.text(
                            if paused { "播放" } else { "暂停" },
                            if paused { "Play" } else { "Pause" },
                        )),
                    )
                    .clicked()
                {
                    if unsafe { self.engine.IsEnded().as_bool() } {
                        action = self.seek(0.0);
                    }
                    if action.is_ok() {
                        action = if paused { self.play() } else { self.pause() };
                    }
                }
                ui.label(format!(
                    "{} / {}",
                    time_label(self.position()),
                    time_label(duration)
                ));
                let mut position = self
                    .seek_preview
                    .unwrap_or_else(|| self.position().max(0.0));
                let seek = ui.add_enabled(
                    ready,
                    egui::Slider::new(&mut position, 0.0..=if ready { duration } else { 1.0 })
                        .show_value(false),
                );
                if seek.changed() {
                    self.seek_preview = Some(position);
                }
                if seek.drag_stopped() || (seek.changed() && !seek.dragged()) {
                    action = self.seek(position);
                    self.seek_preview = None;
                }
                ui.label(locale.text("音量", "Volume"));
                let mut volume = unsafe { self.engine.GetVolume() };
                if ui
                    .add(egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false))
                    .changed()
                {
                    action = self.volume(volume);
                }
            });
            if let Err(error) = action {
                let _ = self.pause();
                self.error = Some(error.to_string());
            }
            if !ready {
                ui.weak(locale.text("正在读取媒体信息…", "Loading media information…"));
            }
            if self.kind == MediaKind::Audio {
                ui.vertical_centered(|ui| {
                    ui.add_space(48.0);
                    crate::file_icons::icon("audio.wav", false).show(ui, 48.0);
                    ui.weak(locale.text("音频预览", "Audio preview"));
                });
            } else {
                if let Err(error) = self.video_frame(ui.ctx()) {
                    let _ = self.pause();
                    self.error = Some(error.to_string());
                }
                let available = ui.available_size().max(egui::vec2(1.0, 1.0));
                if let Some(texture) = &self.texture {
                    let scale = (available.x / self.frame_size[0] as f32)
                        .min(available.y / self.frame_size[1] as f32);
                    let size =
                        egui::vec2(self.frame_size[0] as f32, self.frame_size[1] as f32) * scale;
                    let (area, _) = ui.allocate_exact_size(available, egui::Sense::hover());
                    let rect = egui::Rect::from_center_size(area.center(), size);
                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
            if !self.paused() {
                ui.ctx().request_repaint_after(Duration::from_millis(
                    if self.kind == MediaKind::Video {
                        34
                    } else {
                        250
                    },
                ));
            }
        }
    }
    impl Drop for Player {
        fn drop(&mut self) {
            self.events.active.store(false, Ordering::Release);
            unsafe {
                let _ = self.engine.Shutdown();
            }
            // COM references must be released before the runtime guard uninitializes.
        }
    }
}
