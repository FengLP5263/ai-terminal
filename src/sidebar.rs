use crate::{appearance, model::Agent, status::Activity};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::{
    io::{BufReader, Cursor},
    time::Duration,
};

pub struct Entry<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub directory: &'a std::path::Path,
    pub agent: Agent,
    pub activity: Activity,
    pub unread: bool,
    pub running: bool,
    pub resumable: bool,
    pub hint: String,
}

#[derive(Default, Debug)]
pub struct Actions {
    pub create: bool,
    pub toggle: bool,
    pub select: Option<String>,
    pub start: Option<String>,
    pub rename: Option<String>,
    pub close: Option<String>,
    pub reorder: Option<(String, String, bool)>,
    pub width: f32,
}

#[derive(Clone)]
struct DragSession(String);

pub struct Icons {
    codex: egui::TextureHandle,
    claude: egui::TextureHandle,
}
impl Icons {
    pub fn new(ctx: &egui::Context) -> Self {
        fn load(ctx: &egui::Context, name: &str, bytes: &[u8]) -> egui::TextureHandle {
            let mut decoder = png::Decoder::new(BufReader::new(Cursor::new(bytes)));
            decoder
                .set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
            let mut reader = decoder.read_info().expect("embedded icon header");
            let mut pixels = vec![0; reader.output_buffer_size().expect("small icon")];
            let info = reader
                .next_frame(&mut pixels)
                .expect("embedded icon pixels");
            let size = [info.width as usize, info.height as usize];
            let pixels = &pixels[..info.buffer_size()];
            let image = match info.color_type {
                png::ColorType::Rgba => egui::ColorImage::from_rgba_unmultiplied(size, pixels),
                png::ColorType::Rgb => egui::ColorImage::from_rgb(size, pixels),
                _ => panic!("embedded icon must be RGB/RGBA"),
            };
            ctx.load_texture(name, image, egui::TextureOptions::LINEAR)
        }
        Self {
            codex: load(
                ctx,
                "agent-codex",
                include_bytes!("../assets/icons/openai.png"),
            ),
            claude: load(
                ctx,
                "agent-claude",
                include_bytes!("../assets/icons/claude-color.png"),
            ),
        }
    }
    fn paint(&self, painter: &egui::Painter, rect: Rect, agent: Agent) {
        painter.circle_filled(rect.center(), rect.width() / 2.0, Color32::from_gray(30));
        let icon = rect.shrink(5.0);
        match agent {
            Agent::Codex | Agent::Claude => {
                let texture = if agent == Agent::Codex {
                    &self.codex
                } else {
                    &self.claude
                };
                painter.image(
                    texture.id(),
                    icon,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Agent::Shell => {
                let c = icon.center();
                let s = Stroke::new(1.8, Color32::from_gray(205));
                painter.add(egui::Shape::line(
                    vec![
                        c + egui::vec2(-6.0, -5.0),
                        c + egui::vec2(-1.0, 0.0),
                        c + egui::vec2(-6.0, 5.0),
                    ],
                    s,
                ));
                painter.line_segment([c + egui::vec2(1.0, 5.0), c + egui::vec2(7.0, 5.0)], s);
            }
        }
    }
}

pub fn show(
    ui: &mut egui::Ui,
    entries: &[Entry<'_>],
    active: Option<&str>,
    collapsed: bool,
    width: f32,
    icons: &Icons,
) -> Actions {
    let mut actions = Actions::default();
    let panel = egui::Panel::left(if collapsed {
        "session-rail"
    } else {
        "sessions"
    })
    .default_size(width)
    .size_range(if collapsed {
        52.0..=52.0
    } else {
        190.0..=420.0
    })
    .resizable(!collapsed)
    .frame(
        egui::Frame::new()
            .fill(appearance::SIDEBAR_BG)
            .inner_margin(8),
    );
    let response = panel.show(ui, |ui| {
        ui.horizontal(|ui| {
            if collapsed {
                actions.toggle = ui
                    .add_sized([34.0, 28.0], egui::Button::new("›").frame(false))
                    .on_hover_text("展开会话面板")
                    .clicked();
            } else {
                ui.label(egui::RichText::new("ai-terminal").size(17.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    actions.toggle = ui.small_button("‹").on_hover_text("收起会话面板").clicked();
                    actions.create = ui
                        .small_button("＋")
                        .on_hover_text("新建会话 · Ctrl+Shift+N")
                        .clicked();
                });
            }
        });
        ui.add_space(10.0);
        if collapsed {
            actions.create = ui
                .add_sized([34.0, 28.0], egui::Button::new("＋").frame(false))
                .on_hover_text("新建会话")
                .clicked();
            ui.add_space(5.0);
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in entries {
                    let height = if collapsed { 42.0 } else { 60.0 };
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), height),
                        Sense::hover(),
                    );
                    let response = ui.interact(
                        rect,
                        egui::Id::new(("session-row", entry.id)),
                        Sense::click_and_drag(),
                    );
                    let selected = active == Some(entry.id);
                    let fill = if selected {
                        Color32::from_gray(48)
                    } else if response.hovered() {
                        Color32::from_gray(31)
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(rect, 5, fill);
                    if selected {
                        ui.painter().rect_stroke(
                            rect.shrink(0.5),
                            5,
                            Stroke::new(1.0, Color32::from_gray(75)),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let avatar = Rect::from_center_size(
                        Pos2::new(
                            rect.left() + if collapsed { 18.0 } else { 24.0 },
                            rect.center().y,
                        ),
                        Vec2::splat(30.0),
                    );
                    icons.paint(ui.painter(), avatar, entry.agent);
                    let indicator = avatar.right_bottom() - egui::vec2(2.0, 2.0);
                    if entry.activity == Activity::Working {
                        ui.painter().circle_filled(
                            indicator,
                            6.0,
                            fill.gamma_multiply(1.0).to_opaque(),
                        );
                        let time = ui.input(|i| i.time as f32);
                        let points = (0..13)
                            .map(|i| {
                                let a = time * 4.0 + i as f32 / 12.0 * 4.6;
                                indicator + egui::vec2(a.cos(), a.sin()) * 4.0
                            })
                            .collect();
                        ui.painter().add(egui::Shape::line(
                            points,
                            Stroke::new(1.7, Color32::from_rgb(136, 180, 246)),
                        ));
                        ui.ctx().request_repaint_after(Duration::from_millis(80));
                    } else if entry.activity == Activity::Waiting || entry.unread {
                        ui.painter()
                            .circle_filled(indicator, 5.0, appearance::SIDEBAR_BG);
                        ui.painter().circle_filled(
                            indicator,
                            3.5,
                            if entry.activity == Activity::Waiting {
                                Color32::from_rgb(235, 184, 82)
                            } else {
                                Color32::from_rgb(88, 191, 141)
                            },
                        );
                    }
                    let mut edit = false;
                    if !collapsed {
                        let left = rect.left() + 46.0;
                        let text_rect = Rect::from_min_max(
                            Pos2::new(left, rect.top() + 8.0),
                            Pos2::new(rect.right() - 28.0, rect.bottom()),
                        );
                        let painter = ui.painter().with_clip_rect(text_rect);
                        painter.text(
                            text_rect.left_top(),
                            egui::Align2::LEFT_TOP,
                            entry.name,
                            egui::FontId::proportional(14.0),
                            Color32::from_gray(225),
                        );
                        let project = entry
                            .directory
                            .file_name()
                            .unwrap_or(entry.directory.as_os_str())
                            .to_string_lossy();
                        painter.text(
                            Pos2::new(left, rect.top() + 31.0),
                            egui::Align2::LEFT_TOP,
                            project,
                            egui::FontId::proportional(11.0),
                            Color32::from_gray(145),
                        );
                        if response.hovered() || selected {
                            let edit_rect = Rect::from_center_size(
                                Pos2::new(rect.right() - 15.0, rect.center().y),
                                Vec2::splat(24.0),
                            );
                            let edit_response = ui.interact(
                                edit_rect,
                                egui::Id::new(("edit-session", entry.id)),
                                Sense::click(),
                            );
                            if edit_response.hovered() {
                                ui.painter()
                                    .rect_filled(edit_rect, 3, Color32::from_gray(65));
                            }
                            let c = edit_rect.center();
                            let stroke = Stroke::new(1.5, Color32::from_gray(195));
                            ui.painter().line_segment(
                                [c + egui::vec2(-4.0, 3.0), c + egui::vec2(3.0, -4.0)],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [c + egui::vec2(-2.0, 5.0), c + egui::vec2(5.0, -2.0)],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [c + egui::vec2(3.0, -4.0), c + egui::vec2(5.0, -2.0)],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [c + egui::vec2(-4.0, 3.0), c + egui::vec2(-5.0, 6.0)],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [c + egui::vec2(-5.0, 6.0), c + egui::vec2(-2.0, 5.0)],
                                stroke,
                            );
                            edit = edit_response.on_hover_text("编辑会话名称").clicked();
                        }
                    }
                    if edit {
                        actions.rename = Some(entry.id.into());
                    } else if response.clicked() {
                        actions.select = Some(entry.id.into());
                    }
                    if response.double_clicked() && !entry.running {
                        actions.start = Some(entry.id.into());
                    }
                    response.dnd_set_drag_payload(DragSession(entry.id.into()));
                    let after = ui
                        .input(|i| i.pointer.interact_pos())
                        .is_some_and(|p| p.y > rect.center().y);
                    if let Some(payload) = response.dnd_hover_payload::<DragSession>()
                        && payload.0 != entry.id
                    {
                        let y = if after { rect.bottom() } else { rect.top() };
                        ui.painter().line_segment(
                            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                            Stroke::new(2.0, Color32::from_rgb(136, 180, 246)),
                        );
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    }
                    if let Some(payload) = response.dnd_release_payload::<DragSession>()
                        && payload.0 != entry.id
                    {
                        actions.reorder = Some((payload.0.clone(), entry.id.into(), after));
                    }
                    response.clone().on_hover_text(format!(
                        "{} · {}\n{}\n{}",
                        entry.name,
                        entry.agent.label(),
                        entry.directory.display(),
                        entry.hint
                    ));
                    response.context_menu(|ui| {
                        if ui.button("编辑名称").clicked() {
                            actions.rename = Some(entry.id.into());
                            ui.close();
                        }
                        if !entry.running
                            && ui
                                .button(if entry.resumable {
                                    "继续会话"
                                } else {
                                    "启动会话"
                                })
                                .clicked()
                        {
                            actions.start = Some(entry.id.into());
                            ui.close();
                        }
                        if ui.button("关闭会话").clicked() {
                            actions.close = Some(entry.id.into());
                            ui.close();
                        }
                    });
                    ui.add_space(4.0);
                }
            });
    });
    actions.width = response.response.rect.width();
    actions
}
