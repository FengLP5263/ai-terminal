use crate::{
    appearance, chrome,
    model::{Agent, SessionColor},
    status::Activity,
};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::{
    collections::VecDeque,
    io::{BufReader, Cursor},
    sync::{Arc, Mutex},
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
    pub settings: bool,
    pub toggle: bool,
    pub select: Option<String>,
    pub rename: Option<String>,
    pub color: Option<(String, Option<SessionColor>)>,
    pub close: Option<String>,
    pub reorder: Option<(String, String, bool)>,
    pub width: f32,
}

#[derive(Clone)]
struct DragSession {
    id: String,
    grab: Vec2,
}

/// Project a temporary list order without moving any live sessions before drop.
fn drag_slot(slot: usize, source: usize, destination: usize) -> usize {
    if slot == destination {
        source
    } else if source < destination && (source..destination).contains(&slot) {
        slot + 1
    } else if source > destination && (destination + 1..=source).contains(&slot) {
        slot - 1
    } else {
        slot
    }
}

#[derive(Clone)]
pub struct Icons {
    codex: Arc<Mutex<IconRaster>>,
    claude: Arc<Mutex<IconRaster>>,
    qoder: Arc<Mutex<IconRaster>>,
    kimi: Arc<Mutex<IconRaster>>,
    zcode: Arc<Mutex<IconRaster>>,
    trae: Arc<Mutex<IconRaster>>,
}
struct IconRaster {
    name: &'static str,
    source: egui::ColorImage,
    sizes: VecDeque<(usize, egui::TextureHandle)>,
}
impl IconRaster {
    fn texture(&mut self, ctx: &egui::Context, pixels: usize) -> egui::TextureId {
        if let Some((_, texture)) = self.sizes.iter().find(|(size, _)| *size == pixels) {
            return texture.id();
        }
        let image = minify_icon(&self.source, pixels);
        let texture = ctx.load_texture(
            format!("{}-{pixels}", self.name),
            image,
            egui::TextureOptions::LINEAR,
        );
        let id = texture.id();
        if self.sizes.len() == 12 {
            self.sizes.pop_front();
        }
        self.sizes.push_back((pixels, texture));
        id
    }
}

/// Area filtering preserves thin strokes when a 640px source becomes a 20px avatar.
/// Run once per physical size, sharing a bounded cache across all viewports.
fn minify_icon(source: &egui::ColorImage, size: usize) -> egui::ColorImage {
    let mut output = egui::ColorImage::filled([size, size], Color32::TRANSPARENT);
    let ratio_x = source.width() as f32 / size as f32;
    let ratio_y = source.height() as f32 / size as f32;
    for y in 0..size {
        let top = y as f32 * ratio_y;
        let bottom = (y + 1) as f32 * ratio_y;
        for x in 0..size {
            let left = x as f32 * ratio_x;
            let right = (x + 1) as f32 * ratio_x;
            let mut rgba = [0.0_f32; 4];
            for sy in top.floor() as usize..(bottom.ceil() as usize).min(source.height()) {
                let wy = (bottom.min((sy + 1) as f32) - top.max(sy as f32)).max(0.0);
                for sx in left.floor() as usize..(right.ceil() as usize).min(source.width()) {
                    let weight = wy * (right.min((sx + 1) as f32) - left.max(sx as f32)).max(0.0);
                    for (sum, value) in rgba.iter_mut().zip(source[(sx, sy)].to_array()) {
                        *sum += value as f32 * weight;
                    }
                }
            }
            let rgba = rgba.map(|sum| (sum / (ratio_x * ratio_y)).round() as u8);
            output[(x, y)] = Color32::from_rgba_premultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
        }
    }
    output
}

impl Icons {
    pub fn new(ctx: &egui::Context) -> Self {
        fn load(name: &'static str, bytes: &[u8]) -> Arc<Mutex<IconRaster>> {
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
            Arc::new(Mutex::new(IconRaster {
                name,
                source: image,
                sizes: VecDeque::new(),
            }))
        }
        let _ = ctx;
        Self {
            codex: load("agent-codex", include_bytes!("../assets/icons/openai.png")),
            claude: load(
                "agent-claude",
                include_bytes!("../assets/icons/claude-color.png"),
            ),
            qoder: load(
                "agent-qoder-cn",
                include_bytes!("../assets/icons/qoder.png"),
            ),
            kimi: load(
                "agent-kimi-code",
                include_bytes!("../assets/icons/kimi-color.png"),
            ),
            zcode: load(
                "agent-zcode",
                include_bytes!("../assets/icons/zcode-color.png"),
            ),
            trae: load(
                "agent-trae",
                include_bytes!("../assets/icons/trae-color.png"),
            ),
        }
    }
    pub fn paint(&self, painter: &egui::Painter, rect: Rect, agent: Agent) {
        let snap_square = |center: Pos2, side: f32| {
            let ppp = painter.pixels_per_point();
            let side = (side * ppp).round().max(1.0);
            let min = (center * ppp - Vec2::splat(side / 2.0)).round() / ppp;
            Rect::from_min_size(min, Vec2::splat(side / ppp))
        };
        let rect = snap_square(rect.center(), rect.width());
        painter.circle_filled(rect.center(), rect.width() / 2.0, Color32::from_gray(29));
        let icon = snap_square(rect.center(), chrome::AVATAR_ICON);
        match agent {
            Agent::Codex
            | Agent::Claude
            | Agent::QoderCn
            | Agent::Kimi
            | Agent::ZCode
            | Agent::Trae => {
                let texture = match agent {
                    Agent::Codex => &self.codex,
                    Agent::QoderCn => &self.qoder,
                    Agent::Kimi => &self.kimi,
                    Agent::ZCode => &self.zcode,
                    Agent::Trae => &self.trae,
                    _ => &self.claude,
                };
                painter.image(
                    texture.lock().unwrap().texture(
                        painter.ctx(),
                        (icon.width() * painter.pixels_per_point())
                            .round()
                            .clamp(1.0, 256.0) as usize,
                    ),
                    icon,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Agent::Shell | Agent::Cmd | Agent::Bash => chrome::paint_shell(painter, icon, agent),
        }
    }
}

/// Shared status indicator used by the sidebar and floating session rail.
pub fn paint_status(ui: &egui::Ui, avatar: Rect, activity: Activity, unread: bool, fill: Color32) {
    paint_status_on(ui, ui.painter(), avatar, activity, unread, fill);
}

fn paint_status_on(
    ui: &egui::Ui,
    painter: &egui::Painter,
    avatar: Rect,
    activity: Activity,
    unread: bool,
    fill: Color32,
) {
    let indicator = avatar.right_bottom() - egui::vec2(2.0, 2.0);
    let surface = fill;
    if activity == Activity::Working {
        painter.circle_filled(indicator, 5.5, surface);
        let time = ui.input(|i| i.time as f32);
        let points = (0..13)
            .map(|i| {
                let a = time * 4.0 + i as f32 / 12.0 * 4.6;
                indicator + egui::vec2(a.cos(), a.sin()) * 4.0
            })
            .collect();
        painter.add(egui::Shape::line(
            points,
            Stroke::new(1.75, chrome::WORKING),
        ));
        ui.ctx().request_repaint_after(Duration::from_millis(80));
    } else if activity == Activity::Unconfirmed {
        painter.circle_filled(indicator, 5.5, surface);
        painter.circle_stroke(indicator, 3.5, Stroke::new(1.25, chrome::SECONDARY));
    } else if activity == Activity::Waiting || unread {
        painter.circle_filled(indicator, 5.5, surface);
        painter.circle_filled(
            indicator,
            3.5,
            if activity == Activity::Failed {
                chrome::FAILED
            } else if activity == Activity::Waiting {
                chrome::WAITING
            } else {
                chrome::COMPLETED
            },
        );
    }
}

#[derive(Clone, Copy)]
struct CardStyle {
    selected: bool,
    hovered: bool,
    raised: bool,
}

fn paint_card(
    ui: &egui::Ui,
    painter: &egui::Painter,
    entry: &Entry<'_>,
    rect: Rect,
    collapsed: bool,
    icons: &Icons,
    style: CardStyle,
) {
    let fill = chrome::session_fill(entry.color, style.selected, style.hovered);
    if style.raised {
        painter.rect_filled(
            rect.translate(egui::vec2(0.0, 3.0)).expand(2.0),
            8,
            Color32::from_black_alpha(100),
        );
    }
    painter.rect_filled(rect, 6, fill);
    if style.selected || style.raised {
        painter.rect_stroke(
            rect.shrink(0.5),
            5,
            Stroke::new(1.0, chrome::session_border(entry.color)),
            egui::StrokeKind::Inside,
        );
    }
    let avatar = Rect::from_center_size(
        Pos2::new(
            if collapsed {
                rect.center().x
            } else {
                rect.left() + 24.0
            },
            rect.center().y,
        ),
        Vec2::splat(chrome::AVATAR),
    );
    icons.paint(painter, avatar, entry.agent);
    paint_status_on(ui, painter, avatar, entry.activity, entry.unread, fill);
    if !collapsed {
        let left = rect.left() + 52.0;
        let text_rect = Rect::from_min_max(
            Pos2::new(left, rect.center().y - 18.0),
            Pos2::new(rect.right() - 40.0, rect.bottom()),
        );
        let painter = painter.with_clip_rect(text_rect.intersect(painter.clip_rect()));
        crate::ui_text::paint(
            ui,
            &painter,
            text_rect.left_top(),
            entry.name,
            14.0,
            if style.selected {
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
            if style.selected {
                Color32::from_gray(180)
            } else {
                chrome::SECONDARY
            },
        );
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
    .exact_size(if collapsed {
        chrome::RAIL_ROW + 16.0
    } else {
        width.clamp(
            crate::model::UiSettings::MIN_SIDEBAR_WIDTH,
            crate::model::UiSettings::MAX_SIDEBAR_WIDTH,
        )
    })
    .resizable(false)
    .show_separator_line(true)
    .frame(
        egui::Frame::new()
            .fill(appearance::SIDEBAR_BG)
            .inner_margin(egui::Margin {
                left: 8,
                right: 1,
                top: 8,
                bottom: 8,
            }),
    );
    let response = panel.show(ui, |ui| {
        if collapsed {
            ui.spacing_mut().item_spacing.y = 4.0;
        }
        ui.horizontal(|ui| {
            let button = if collapsed {
                chrome::rail_button
            } else {
                chrome::icon_button
            };
            actions.settings = button(
                ui,
                egui::Id::new("settings-button"),
                chrome::ActionIcon::Settings,
                crate::i18n::tr(ui.ctx(), "设置", "Settings"),
            )
            .clicked();
            if !collapsed {
                let space = (ui.available_width()
                    - 2.0 * chrome::CONTROL
                    - 2.0 * ui.spacing().item_spacing.x)
                    .max(0.0);
                ui.add_space(space);
                actions.create = new_session_menu(ui);
                actions.toggle = chrome::icon_button(
                    ui,
                    egui::Id::new("sidebar-toggle"),
                    chrome::ActionIcon::Collapse,
                    crate::i18n::tr(ui.ctx(), "收起会话面板", "Collapse sidebar"),
                )
                .clicked();
            }
        });
        if collapsed {
            actions.create =
                new_session_menu_inner(ui, egui::Id::new(chrome::NEW_TERMINAL_ID), true);
            actions.toggle = chrome::rail_button(
                ui,
                egui::Id::new("sidebar-toggle"),
                chrome::ActionIcon::Expand,
                crate::i18n::tr(ui.ctx(), "展开会话面板", "Expand sidebar"),
            )
            .clicked();
        }
        ui.add_space(8.0);
        // Keep a permanent gutter without painting a scrollbar outside the panel.
        let pointer_inside = ui.rect_contains_pointer(ui.max_rect());
        let scroll_style = &mut ui.style_mut().spacing.scroll;
        scroll_style.floating = true;
        scroll_style.bar_width = 3.0;
        scroll_style.floating_width = 3.0;
        scroll_style.floating_allocated_width = 7.0;
        scroll_style.bar_inner_margin = 0.0;
        scroll_style.bar_outer_margin = 0.0;
        scroll_style.dormant_background_opacity = 0.0;
        scroll_style.active_background_opacity = 0.0;
        scroll_style.interact_background_opacity = 0.0;
        scroll_style.dormant_handle_opacity = 0.0;
        scroll_style.active_handle_opacity = if pointer_inside { 0.55 } else { 0.0 };
        scroll_style.interact_handle_opacity = if pointer_inside { 0.8 } else { 0.0 };
        // Keep the card inset even when the floating scrollbar is hidden.
        // The scrollbar itself remains next to the divider in this gutter.
        let row_right = ui.max_rect().right() - 7.0;
        egui::ScrollArea::vertical()
            .id_salt("session-list-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let drag = egui::DragAndDrop::payload::<DragSession>(ui.ctx());
                let pointer = ui.input(|i| i.pointer.interact_pos());
                let source = drag
                    .as_ref()
                    .and_then(|drag| entries.iter().position(|entry| entry.id == drag.id));
                let row_height = if collapsed {
                    chrome::RAIL_ROW
                } else {
                    chrome::ROW
                };
                let pitch =
                    row_height + ui.spacing().item_spacing.y + if collapsed { 0.0 } else { 4.0 };
                let list_top = ui.cursor().top();
                let viewport = ui.clip_rect().intersect(Rect::from_min_max(
                    Pos2::new(ui.cursor().left(), ui.clip_rect().top()),
                    Pos2::new(row_right, ui.clip_rect().bottom()),
                ));
                // Use fixed slots rather than moving card hitboxes to avoid reorder feedback.
                let destination = source.map(|source| {
                    match (drag.as_ref(), pointer.filter(|p| viewport.contains(*p))) {
                        (Some(drag), Some(pointer)) => {
                            (((pointer.y - drag.grab.y - list_top) / pitch)
                                .round()
                                .max(0.0) as usize)
                                .min(entries.len().saturating_sub(1))
                        }
                        _ => source,
                    }
                });
                for slot in 0..entries.len() {
                    let index = match (source, destination) {
                        (Some(source), Some(destination)) => drag_slot(slot, source, destination),
                        _ => slot,
                    };
                    let entry = &entries[index];
                    // A scrollbar can change available width from 51 to 44.
                    // Deriving square row height from it toggles overflow forever
                    // near the fit threshold. Keep the rail tile independent.
                    let size = if collapsed {
                        Vec2::splat(chrome::RAIL_ROW)
                    } else {
                        Vec2::new(
                            (row_right - ui.available_rect_before_wrap().left()).max(1.0),
                            chrome::ROW,
                        )
                    };
                    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                    let response = ui.interact(
                        rect,
                        egui::Id::new(("session-row", entry.id)),
                        Sense::click_and_drag(),
                    );
                    let selected = active == Some(entry.id);
                    let hovered = ui.rect_contains_pointer(rect);
                    let is_source = source == Some(index);
                    if is_source {
                        // The gap takes the same space as a real card; all other rows move aside.
                        ui.painter()
                            .rect_filled(rect.shrink(1.0), 6, Color32::from_gray(22));
                        ui.painter().rect_stroke(
                            rect.shrink(1.0),
                            5,
                            Stroke::new(1.0, chrome::BORDER),
                            egui::StrokeKind::Inside,
                        );
                    } else {
                        paint_card(
                            ui,
                            ui.painter(),
                            entry,
                            rect,
                            collapsed,
                            icons,
                            CardStyle {
                                selected,
                                hovered,
                                raised: false,
                            },
                        );
                    }
                    let mut edit = false;
                    if !collapsed && drag.is_none() && (hovered || selected) {
                        let edit_rect = Rect::from_center_size(
                            Pos2::new(rect.right() - 18.0, rect.center().y),
                            Vec2::splat(chrome::CONTROL),
                        );
                        edit = chrome::icon_at(
                            ui,
                            edit_rect,
                            egui::Id::new(("edit-session", entry.id)),
                            chrome::ActionIcon::Edit,
                            crate::i18n::tr(ui.ctx(), "编辑会话名称", "Edit session name"),
                        )
                        .clicked();
                    }
                    if edit {
                        actions.rename = Some(entry.id.into());
                    } else if response.clicked() {
                        actions.select = Some(entry.id.into());
                    }
                    if response.drag_started_by(egui::PointerButton::Primary) {
                        let press = ui
                            .input(|i| i.pointer.press_origin())
                            .unwrap_or(rect.center());
                        egui::DragAndDrop::set_payload(
                            ui.ctx(),
                            DragSession {
                                id: entry.id.into(),
                                grab: press - rect.min,
                            },
                        );
                        ui.ctx().request_repaint();
                    }
                    if drag.is_none() && !response.dragged() {
                        response.clone().on_hover_text(format!(
                            "{} · {}\n{}\n{}",
                            entry.name,
                            entry.agent.label(),
                            entry.directory.display(),
                            entry.hint
                        ));
                    }
                    response.context_menu(|ui| {
                        if ui
                            .button(crate::i18n::tr(ui.ctx(), "编辑名称", "Rename"))
                            .clicked()
                        {
                            actions.rename = Some(entry.id.into());
                            ui.close();
                        }
                        ui.menu_button(
                            crate::i18n::tr(ui.ctx(), "会话颜色", "Session color"),
                            |ui| {
                                egui::Grid::new(("session-palette", entry.id))
                                    .num_columns(4)
                                    .spacing(egui::vec2(6.0, 6.0))
                                    .show(ui, |ui| {
                                        for (index, color) in std::iter::once(None)
                                            .chain(SessionColor::ALL.into_iter().map(Some))
                                            .enumerate()
                                        {
                                            if color_swatch(
                                                ui,
                                                entry.id,
                                                color,
                                                entry.color == color,
                                            )
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
                            },
                        );
                        if ui
                            .button(crate::i18n::tr(ui.ctx(), "关闭会话", "Close session"))
                            .clicked()
                        {
                            actions.close = Some(entry.id.into());
                            ui.close();
                        }
                    });
                    if slot + 1 < entries.len() {
                        ui.painter().hline(
                            rect.left() + 4.0..=rect.right() - 4.0,
                            rect.bottom() + if collapsed { 2.0 } else { 5.0 },
                            Stroke::new(1.0 / ui.ctx().pixels_per_point(), Color32::from_gray(40)),
                        );
                    }
                    if !collapsed {
                        ui.add_space(4.0);
                    }
                }
                // A non-interactive foreground copy follows the original grab point, even
                // outside the rail. Only metadata is painted; no terminal data is copied.
                if let Some(drag) = egui::DragAndDrop::payload::<DragSession>(ui.ctx())
                    && let Some(entry) = entries.iter().find(|entry| entry.id == drag.id)
                    && let Some(pointer) = pointer
                {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
                        if viewport.contains(pointer)
                            && let (Some(source), Some(destination)) = (source, destination)
                            && source != destination
                        {
                            actions.reorder = Some((
                                drag.id.clone(),
                                entries[destination].id.into(),
                                destination > source,
                            ));
                        }
                        egui::DragAndDrop::clear_payload(ui.ctx());
                    } else {
                        // Only a held card near the viewport edge scrolls the long list.
                        // Ordinary hovering and idle frames never start a scroll loop.
                        if viewport.contains(pointer) {
                            let edge = 24.0_f32.min(viewport.height() / 4.0);
                            let direction =
                                if pointer.y < viewport.top() + edge && list_top < viewport.top() {
                                    1.0
                                } else if pointer.y > viewport.bottom() - edge
                                    && ui.min_rect().bottom() > viewport.bottom()
                                {
                                    -1.0
                                } else {
                                    0.0
                                };
                            if direction != 0.0 {
                                let distance = ui.input(|i| i.stable_dt.min(0.05)) * 260.0;
                                ui.scroll_with_delta_animation(
                                    egui::vec2(0.0, direction * distance),
                                    egui::style::ScrollAnimation::none(),
                                );
                                ui.ctx().request_repaint();
                            }
                        }
                        let size = if collapsed {
                            Vec2::splat(chrome::RAIL_ROW)
                        } else {
                            Vec2::new((row_right - ui.max_rect().left()).max(1.0), chrome::ROW)
                        };
                        let rect = Rect::from_min_size(pointer - drag.grab, size);
                        let painter = ui.ctx().layer_painter(egui::LayerId::new(
                            egui::Order::Tooltip,
                            egui::Id::new("session-drag-preview"),
                        ));
                        paint_card(
                            ui,
                            &painter,
                            entry,
                            rect,
                            collapsed,
                            icons,
                            CardStyle {
                                selected: active == Some(entry.id),
                                hovered: true,
                                raised: true,
                            },
                        );
                    }
                }
            });
    });
    actions.width = response.response.rect.width();
    if !collapsed {
        // The resize target lives outside the list, so it cannot steal scrollbar drags.
        let panel = response.response.rect;
        let grip = Rect::from_min_max(
            Pos2::new(panel.right(), panel.top()),
            Pos2::new(panel.right() + 6.0, panel.bottom()),
        );
        let resize = ui
            .interact(grip, egui::Id::new("sidebar-resize"), Sense::drag())
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if resize.dragged() {
            actions.width = (actions.width + resize.drag_delta().x).clamp(
                crate::model::UiSettings::MIN_SIDEBAR_WIDTH,
                crate::model::UiSettings::MAX_SIDEBAR_WIDTH,
            );
        }
    }
    actions
}

/// A new terminal is one menu choice; project and Agent selection belong in the shell.
pub fn new_session_menu(ui: &mut egui::Ui) -> Option<Agent> {
    new_session_menu_with_id(ui, egui::Id::new(chrome::NEW_TERMINAL_ID))
}

pub fn new_session_menu_with_id(ui: &mut egui::Ui, id: egui::Id) -> Option<Agent> {
    new_session_menu_inner(ui, id, false)
}

fn new_session_menu_inner(ui: &mut egui::Ui, id: egui::Id, rail: bool) -> Option<Agent> {
    let button = if rail {
        chrome::rail_button
    } else {
        chrome::icon_button
    };
    let trigger = button(
        ui,
        id,
        chrome::ActionIcon::Add,
        crate::i18n::tr(
            ui.ctx(),
            "新建终端 · Ctrl+Shift+N 打开 PowerShell",
            "New terminal · Ctrl+Shift+N opens PowerShell",
        ),
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
    let label =
        color
            .map(SessionColor::label)
            .unwrap_or(crate::i18n::tr(ui.ctx(), "默认", "Default"));
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

#[cfg(test)]
mod icon_tests {
    use super::*;
    #[test]
    fn small_avatars_retain_thin_source_strokes_and_transparency() {
        let mut source = egui::ColorImage::filled([64, 64], Color32::TRANSPARENT);
        for y in 0..64 {
            source[(0, y)] = Color32::WHITE;
        }
        let filtered = minify_icon(&source, 4);
        for y in 0..4 {
            assert!(
                (15..=17).contains(&filtered[(0, y)].a()),
                "thin stroke vanished"
            );
            assert_eq!(
                filtered[(0, y)].r(),
                filtered[(0, y)].a(),
                "premultiplied edge lost brightness"
            );
            assert_eq!(filtered[(1, y)], Color32::TRANSPARENT);
        }
    }
}
