//! Native local-file playback checks. Generated fixtures only; no Agent, network or user data.
#[cfg(not(windows))]
fn main() {
    panic!("Native media probe requires Windows");
}

#[cfg(windows)]
fn main() -> eframe::Result {
    use ai_terminal::{
        appearance,
        media_player::Player,
        project_media::{MediaKind, Playable},
    };
    use std::{
        path::PathBuf,
        time::{Duration, Instant},
    };
    struct Probe {
        output: PathBuf,
        player: Option<Player>,
        shots: usize,
        audio: &'static str,
        began: Instant,
        case: usize,
        stage: usize,
        changed: Instant,
        position: f64,
        checks: Vec<serde_json::Value>,
        done: bool,
    }
    impl Probe {
        fn check(&mut self, name: &str, ok: bool) {
            self.checks
                .push(serde_json::json!({"case":self.case,"check":name,"ok":ok}));
        }
    }
    impl eframe::App for Probe {
        fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
            let ctx = ui.ctx().clone();
            for event in ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot {
                    image, user_data, ..
                } = event
                {
                    let name = user_data
                        .data
                        .as_ref()
                        .and_then(|d| d.downcast_ref::<String>())
                        .cloned()
                        .unwrap();
                    save_png(&image, &self.output.join(name));
                    self.shots += 1;
                }
            }
            if self.began.elapsed() > Duration::from_secs(60) {
                self.check("global-deadline", false);
                self.done = true;
            }
            ui.heading("Native media preview · generated fixtures");
            ui.label(if self.case == 0 {
                "Audio / 音频 😀.wav"
            } else if self.case == 1 {
                "Video / 视频 #1.mp4"
            } else {
                "Unsupported media fixture"
            });
            if !self.done {
                if self.stage == 0 {
                    let (file, kind) = match self.case {
                        0 => (self.audio, MediaKind::Audio),
                        1 => ("视频 #1.mp4", MediaKind::Video),
                        _ => ("invalid.mp4", MediaKind::Video),
                    };
                    match Player::new(
                        &ctx,
                        &Playable {
                            path: self.output.join(file).canonicalize().unwrap(),
                            kind,
                        },
                    ) {
                        Ok(player) => self.player = Some(player),
                        Err(error) => {
                            self.check("create", false);
                            std::fs::write(self.output.join("error.txt"), error).unwrap();
                            self.done = true;
                        }
                    }
                    self.changed = Instant::now();
                    self.stage = 1;
                }
                if let Some(player) = &mut self.player {
                    player.show(ui);
                }
                let elapsed = self.changed.elapsed();
                let Some(player) = self.player.as_ref() else {
                    return;
                };
                if elapsed > Duration::from_secs(12) {
                    std::fs::write(self.output.join("diagnostics.txt"), player.diagnostics())
                        .unwrap();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        "timeout.png".to_owned(),
                    )));
                    self.check("stage-timeout", false);
                    self.done = true;
                } else if self.case == 2 {
                    if player.failed() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                            egui::UserData::new("unsupported.png".to_owned()),
                        ));
                        self.check("unsupported-reported", true);
                        self.done = true;
                    }
                } else {
                    match self.stage {
                        1 if player.duration().is_finite() && player.duration() > 1.0 => {
                            self.check("does-not-autoplay", player.paused());
                            let player = self.player.as_ref().unwrap();
                            player.volume(0.0).unwrap(); // Tests remain silent.
                            player.play().unwrap();
                            self.stage = 2;
                            self.changed = Instant::now();
                        }
                        2 if elapsed > Duration::from_millis(900) => {
                            self.position = player.position();
                            let moving = self.position > 0.2;
                            player.pause().unwrap();
                            self.check("position-advances", moving);
                            self.stage = 3;
                            self.changed = Instant::now();
                        }
                        3 if elapsed > Duration::from_millis(500) => {
                            let paused =
                                player.paused() && (player.position() - self.position).abs() < 0.2;
                            player.seek(2.0).unwrap();
                            self.check("pause-holds-position", paused);
                            self.stage = 4;
                            self.changed = Instant::now();
                        }
                        4 if elapsed > Duration::from_millis(600) => {
                            let sought = (player.position() - 2.0).abs() < 0.3;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                                egui::UserData::new(format!("case-{}.png", self.case)),
                            ));
                            self.check("seek", sought);
                            if self.case == 1 {
                                self.check(
                                    "decoded-video-frame",
                                    self.player.as_ref().unwrap().has_frame(),
                                );
                            }
                            self.stage = 5;
                            self.changed = Instant::now();
                        }
                        5 if self.shots > self.case => {
                            player.seek((player.duration() - 0.3).max(0.0)).unwrap();
                            player.play().unwrap();
                            self.stage = 6;
                            self.changed = Instant::now();
                        }
                        6 if elapsed > Duration::from_millis(1100) => {
                            self.check("end-of-file-pauses", player.paused());
                            self.player = None;
                            let file = self.output.join(if self.case == 0 {
                                self.audio
                            } else {
                                "视频 #1.mp4"
                            });
                            let renamed = file.with_extension("released");
                            let released = std::fs::rename(&file, &renamed).is_ok();
                            if released {
                                std::fs::rename(renamed, &file).unwrap();
                            }
                            self.check("close-releases-file", released);
                            self.case += 1;
                            self.stage = 0;
                        }
                        _ => {}
                    }
                }
            }
            if self.done
                && self.shots < 3
                && let Some(player) = &mut self.player
            {
                player.show(ui);
            }
            if self.done && (self.shots >= 3 || self.checks.iter().any(|c| c["ok"] == false)) {
                self.player = None;
                let ok = self.checks.iter().all(|c| c["ok"] == true) && self.case == 2;
                std::fs::write(self.output.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"ok":ok,"checks":self.checks,"synthetic_media":true,"network":false,"model_requests":false})).unwrap()).unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(Duration::from_millis(40));
        }
    }
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("media_probe <unique-fixture-directory>"),
    );
    std::fs::write(output.join("invalid.mp4"), b"unsupported media fixture").unwrap();
    eframe::run_native(
        "ai-terminal native media probe",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_inner_size([1050.0, 720.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            ai_terminal::window_frame::apply_dark(cc);
            Ok(Box::new(Probe {
                output,
                player: None,
                shots: 0,
                audio: if std::env::args().any(|a| a == "mp3") {
                    "音频 😀.mp3"
                } else {
                    "音频 😀.wav"
                },
                began: Instant::now(),
                case: 0,
                stage: 0,
                changed: Instant::now(),
                position: 0.0,
                checks: vec![],
                done: false,
            }))
        }),
    )
}

#[cfg(windows)]
fn save_png(image: &egui::ColorImage, path: &std::path::Path) {
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(std::fs::File::create(path).unwrap()),
        image.width() as u32,
        image.height() as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
}
