//! Owned CF_HDROP memory -> production path encoding -> real ConPTY; no system clipboard writes.
#[allow(dead_code)]
#[rustfmt::skip]
#[path = "../vendor/egui_term/src/paste.rs"]
mod clipboard;
use ai_terminal::appearance;
use egui_term::{BackendCommand, BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

fn paths() -> String {
    use windows_sys::Win32::{
        Foundation::GlobalFree,
        System::Memory::{GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock},
    };
    let units = [
        r"C:\Projects\demo\notes review.md",
        r"C:\Projects\demo\图😀.png",
        r"\\server\share\folder",
    ]
    .into_iter()
    .flat_map(|s| s.encode_utf16().chain([0]))
    .chain([0])
    .collect::<Vec<_>>();
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, 20 + units.len() * 2);
        assert!(!memory.is_null());
        let ptr = GlobalLock(memory).cast::<u8>();
        assert!(!ptr.is_null());
        ptr.cast::<u32>().write(20);
        ptr.add(16).cast::<i32>().write(1);
        std::ptr::copy_nonoverlapping(units.as_ptr(), ptr.add(20).cast::<u16>(), units.len());
        GlobalUnlock(memory);
        let value = clipboard::drop_files(memory);
        GlobalFree(memory);
        value.unwrap()
    }
}
struct Probe {
    renderer: egui_term::TextRendererSession,
    terminal: TerminalBackend,
    output: PathBuf,
    paths: String,
    began: Instant,
    pasted: Option<Instant>,
    pending: bool,
    captured: bool,
    entered: bool,
}
impl eframe::App for Probe {
    #[cfg(feature = "renderer-glow")]
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.renderer.release();
    }
    #[cfg(not(feature = "renderer-glow"))]
    fn on_exit(&mut self) {
        self.renderer.release();
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        assert!(
            self.began.elapsed() < Duration::from_secs(25),
            "file paste probe timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(80));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let mut encoder = png::Encoder::new(
                    std::fs::File::create(self.output.join("unsent-file-paths.png")).unwrap(),
                    image.width() as u32,
                    image.height() as u32,
                );
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(
                        &image
                            .pixels
                            .iter()
                            .flat_map(|p| p.to_array())
                            .collect::<Vec<_>>(),
                    )
                    .unwrap();
                self.captured = true;
            }
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let text = self.terminal.plain_text();
        if self.pasted.is_none() && text.contains("FILE_PASTE_READY") {
            let payload = clipboard::bytes("", false, true, Some(&self.paths));
            assert!(!payload.contains(&b'\r') && !payload.contains(&b'\n'));
            self.terminal
                .process_command(BackendCommand::Write(payload));
            self.pasted = Some(Instant::now());
        }
        if self
            .pasted
            .is_some_and(|t| t.elapsed() > Duration::from_millis(700))
            && !self.pending
        {
            assert!(
                !text.contains("RECEIVED_HEX:"),
                "paste submitted the input without Enter"
            );
            self.pending = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        if self.captured && !self.entered {
            // Enter is sent only to this owned ReadLine fixture after the unsent capture.
            self.terminal
                .process_command(BackendCommand::Write(vec![b'\r']));
            self.entered = true;
        }
        if self.entered && text.contains("RECEIVED_HEX:") {
            let expected = self
                .paths
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join("-");
            let compact = text
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>();
            if compact.contains(&format!("RECEIVED_HEX:{expected}")) {
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"system_clipboard_written":false,"synthetic_hdrop":true,"real_conpty":true,"model_request":false,"paste_submitted_input":false,"owned_fixture_enter_after_capture":true,"pty_pid":self.terminal.pty_id(),"paths":self.paths})).unwrap()).unwrap();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("File paste / owned Windows file-list fixture / no model request");
            let view = TerminalView::new(ui, &mut self.terminal)
                .set_focus(true)
                .set_font(appearance::current_terminal_font(ui.ctx(), 14.0))
                .set_size(ui.available_size());
            ui.add(view);
        });
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("unique output directory required"),
    );
    assert!(!output.exists());
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "ai-terminal file paste verification",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(false)
                .with_inner_size([1000.0, 500.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            let (tx, _) = mpsc::channel();
            let terminal=TerminalBackend::new(993,cc.egui_ctx.clone(),tx,BackendSettings {shell:"powershell.exe".into(),args:vec!["-NoLogo".into(),"-NoProfile".into(),"-NonInteractive".into(),"-Command".into(),"[Console]::InputEncoding=[Text.UTF8Encoding]::new($false); [Console]::WriteLine('FILE_PASTE_READY'); $line=[Console]::ReadLine(); [Console]::WriteLine('RECEIVED_HEX:'+ [BitConverter]::ToString([Text.Encoding]::UTF8.GetBytes($line))); Start-Sleep -Seconds 25".into()],..Default::default()}).unwrap();
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                terminal,
                output,
                paths: paths(),
                began: Instant::now(),
                pasted: None,
                pending: false,
                captured: false,
                entered: false,
            }))
        }),
    )
}
