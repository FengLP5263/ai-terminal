use crate::{
    appearance, chrome,
    model::{Agent, SessionColor},
    status::Activity,
};
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
    pub color: Option<SessionColor>,
    pub activity: Activity,
    pub unread: bool,
    pub hint: String,
}

#[derive(Default, Debug)]
pub struct Actions {
    pub create: Option<Agent>,
    pub toggle: bool,
    pub select: Option<String>,
    pub rename: Option<String>,
    pub color: Option<(String, Option<SessionColor>)>,
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
        painter.circle_filled(rect.center(), rect.width() / 2.0, Color32::from_gray(29));
        let icon = Rect::from_center_size(rect.center(), Vec2::splat(chrome::AVATAR_ICON));
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
            Agent::Shell | Agent::Cmd | Agent::Bash => chrome::paint_shell(painter, icon, agent),
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
                ui.add_space(2.0);
                actions.toggle = chrome::icon_button(
                    ui,
                    egui::Id::new("sidebar-toggle"),
                    chrome::ActionIcon::Expand,
                    "展开会话面板",
                )
                .clicked();
            } else {
                let title_width = (ui.available_width()
                    - 2.0 * chrome::CONTROL
                    - 2.0 * ui.spacing().item_spacing.x)
                    .max(1.0);
                let (rect, _) = ui
                    .allocate_exact_size(egui::vec2(title_width, chrome::CONTROL), Sense::hover());
                let title = ui.interact(rect, egui::Id::new("sidebar-title"), Sense::hover());
                title.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), "AI终端")
                });
                ui.painter().text(
                    egui::pos2(rect.left(), rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "AI终端",
                    egui::FontId::proportional(16.0),
                    chrome::TEXT,
                );
                actions.create = new_session_menu(ui);
                actions.toggle = chrome::icon_button(
                    ui,
                    egui::Id::new("sidebar-toggle"),
                    chrome::ActionIcon::Collapse,
                    "收起会话面板",
                )
                .clicked();
            }
        });
        ui.add_space(12.0);
        if collapsed {
            ui.horizontal(|ui| {
                ui.add_space(2.0);
                actions.create = new_session_menu(ui);
            });
            ui.add_space(8.0);
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, entry) in entries.iter().enumerate() {
                    let height = if collapsed {
                        chrome::RAIL_ROW
                    } else {
                        chrome::ROW
                    };
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
                    let hovered = ui.rect_contains_pointer(rect);
                    let fill = chrome::session_fill(entry.color, selected, hovered);
                    ui.painter().rect_filled(rect, 6, fill);
                    if selected {
                        ui.painter().rect_stroke(
                            rect.shrink(0.5),
                            5,
                            Stroke::new(1.0, chrome::session_border(entry.color)),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let avatar = Rect::from_center_size(
                        Pos2::new(
                            rect.left() + if collapsed { 18.0 } else { 24.0 },
                            rect.center().y,
                        ),
                        Vec2::splat(chrome::AVATAR),
                    );
                    icons.paint(ui.painter(), avatar, entry.agent);
                    let indicator = avatar.right_bottom() - egui::vec2(2.0, 2.0);
                    let surface = fill;
                    if entry.activity == Activity::Working {
                        ui.painter().circle_filled(indicator, 5.5, surface);
                        let time = ui.input(|i| i.time as f32);
                        let points = (0..13)
                            .map(|i| {
                                let a = time * 4.0 + i as f32 / 12.0 * 4.6;
                                indicator + egui::vec2(a.cos(), a.sin()) * 4.0
                            })
                            .collect();
                        ui.painter().add(egui::Shape::line(
                            points,
                            Stroke::new(1.5, Color32::from_rgb(136, 180, 246)),
                        ));
                        ui.ctx().request_repaint_after(Duration::from_millis(80));
                    } else if matches!(entry.activity, Activity::Waiting | Activity::Completed)
                        || entry.unread
                    {
                        ui.painter().circle_filled(indicator, 5.5, surface);
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
                        let left = rect.left() + 52.0;
                        let text_rect = Rect::from_min_max(
                            Pos2::new(left, rect.center().y - 18.0),
                            Pos2::new(rect.right() - 40.0, rect.bottom()),
                        );
                        let painter = ui.painter().with_clip_rect(text_rect);
                        crate::ui_text::paint(
                            ui,
                            &painter,
                            text_rect.left_top(),
                            entry.name,
                            14.0,
                            if selected {
                                Color32::from_gray(245)
                            } else {
                                chrome::TEXT
                            },
                            text_rect.width(),
                        );
                        let project = entry
                            .directory
                            .file_name()
                            .unwrap_or(entry.directory.as_os_str())
                            .to_string_lossy();
                        painter.text(
                            Pos2::new(left, rect.center().y + 4.0),
                            egui::Align2::LEFT_TOP,
                            project,
                            egui::FontId::proportional(12.0),
                            if selected {
                                Color32::from_gray(180)
                            } else {
                                chrome::SECONDARY
                            },
                        );
                        if hovered || selected {
                            let edit_rect = Rect::from_center_size(
                                Pos2::new(rect.right() - 18.0, rect.center().y),
                                Vec2::splat(chrome::CONTROL),
                            );
                            edit = chrome::icon_at(
                                ui,
                                edit_rect,
                                egui::Id::new(("edit-session", entry.id)),
                                chrome::ActionIcon::Edit,
                                "编辑会话名称",
                            )
                            .clicked();
                        }
                    }
                    if edit {
                        actions.rename = Some(entry.id.into());
                    } else if response.clicked() {
                        actions.select = Some(entry.id.into());
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
                        ui.menu_button("会话颜色", |ui| {
                            egui::Grid::new(("session-palette", entry.id))
                                .num_columns(4)
                                .spacing(egui::vec2(6.0, 6.0))
                                .show(ui, |ui| {
                                    for (index, color) in std::iter::once(None)
                                        .chain(SessionColor::ALL.into_iter().map(Some))
                                        .enumerate()
                                    {
                                        if color_swatch(ui, entry.id, color, entry.color == color)
                                            .clicked()
                                        {
                                            actions.color = Some((entry.id.into(), color));
                                            ui.close();
                                        }
                                        if index % 4 == 3 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                        if ui.button("关闭会话").clicked() {
                            actions.close = Some(entry.id.into());
                            ui.close();
                        }
                    });
                    if index + 1 < entries.len() {
                        ui.painter().hline(
                            rect.left() + 4.0..=rect.right() - 4.0,
                            rect.bottom() + 5.0,
                            Stroke::new(1.0 / ui.ctx().pixels_per_point(), Color32::from_gray(40)),
                        );
                    }
                    ui.add_space(4.0);
                }
            });
    });
    actions.width = response.response.rect.width();
    actions
}

/// A new terminal is one menu choice; project and Agent selection belong in the shell.
pub fn new_session_menu(ui: &mut egui::Ui) -> Option<Agent> {
    new_session_menu_with_id(ui, egui::Id::new(chrome::NEW_TERMINAL_ID))
}

pub fn new_session_menu_with_id(ui: &mut egui::Ui, id: egui::Id) -> Option<Agent> {
    let trigger = chrome::icon_button(
        ui,
        id,
        chrome::ActionIcon::Add,
        "新建终端 · Ctrl+Shift+N 打开 PowerShell",
    );
    let mut selected = None;
    egui::Popup::menu(&trigger).show(|ui| {
        ui.set_min_width(148.0);
        ui.spacing_mut().interact_size.y = chrome::CONTROL;
        for shell in [Agent::Shell, Agent::Cmd, Agent::Bash] {
            if ui.button(shell.label()).clicked() {
                selected = Some(shell);
                ui.close();
            }
        }
    });
    selected
}

fn color_swatch(
    ui: &mut egui::Ui,
    session: &str,
    color: Option<SessionColor>,
    selected: bool,
) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(chrome::CONTROL), Sense::hover());
    let response = ui.interact(
        rect,
        egui::Id::new(("session-color", session, color)),
        Sense::click(),
    );
    let label = color.map(SessionColor::label).unwrap_or("默认");
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if response.hovered() || response.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, 6, chrome::HOVER);
    }
    let fill = color
        .map(|c| {
            let [r, g, b] = c.rgb();
            Color32::from_rgb(r, g, b)
        })
        .unwrap_or(Color32::from_gray(90));
    ui.painter().circle_filled(rect.center(), 9.0, fill);
    if color.is_none() {
        ui.painter().line_segment(
            [
                rect.center() + egui::vec2(-5.0, 5.0),
                rect.center() + egui::vec2(5.0, -5.0),
            ],
            Stroke::new(1.5, Color32::from_gray(200)),
        );
    }
    if selected || response.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            6,
            Stroke::new(1.5, chrome::TEXT),
            egui::StrokeKind::Inside,
        );
    }
    response.on_hover_text(label)
}
