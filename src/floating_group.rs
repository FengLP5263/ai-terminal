//! Interactive, non-activating member list owned by the floating status window.
use crate::{
    chrome,
    floating::{Action, HoverHint, HoverState, dispatch, group_viewport_id},
    floating_hint::{anchor, origin_for, window},
    sidebar::{self, Icons},
};
use egui::{Color32, Pos2, Rect, Sense, Vec2};
use std::{
    sync::{Arc, Mutex, mpsc::SyncSender},
    time::Instant,
};
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::*,
};

const MEMBER_GAP: f32 = 6.0;
const FIRST_MEMBER_GAP: f32 = 12.0;

fn content_height(count: usize) -> f32 {
    44.0 + count as f32 * 48.0
        + count.saturating_sub(1) as f32 * MEMBER_GAP
        + if count > 1 {
            FIRST_MEMBER_GAP - MEMBER_GAP
        } else {
            0.0
        }
}

pub(crate) fn show(
    ctx: &egui::Context,
    hint: HoverHint,
    zoom: f32,
    state: Arc<Mutex<HoverState>>,
    sender: SyncSender<Action>,
    icons: Icons,
) {
    let Some(group) = hint.group else {
        return;
    };
    let Some(owner) = window(ctx, "ai-terminal status", "floating-native-hwnd") else {
        return;
    };
    let Some((bar, work, scale)) = anchor(owner, zoom) else {
        return;
    };
    let size = Vec2::new(
        320.0f32.min((work[2] - work[0]) as f32 / scale),
        content_height(group.items.len())
            .min(380.0)
            .min((work[3] - work[1]) as f32 / scale),
    );
    let dimensions = [
        (size.x * scale).ceil() as i32,
        (size.y * scale).ceil() as i32,
    ];
    let place = origin_for(
        bar,
        bar[0] + (hint.anchor.x * scale).round() as i32,
        dimensions,
        work,
        (4.0 * scale).round() as i32,
    );
    let ratio = zoom / ctx.zoom_factor();
    let mut builder = egui::ViewportBuilder::default()
        .with_title("ai-terminal group")
        .with_icon(crate::appearance::window_icon())
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_inner_size(size * ratio)
        .with_taskbar(false)
        .with_active(false)
        .with_always_on_top();
    if !ctx.input(|i| i.raw.viewports.contains_key(&group_viewport_id())) {
        builder = builder.with_position([
            place[0] as f32 / (scale / ratio),
            place[1] as f32 / (scale / ratio),
        ]);
    }
    let owner = owner as usize;
    ctx.show_viewport_deferred(group_viewport_id(), builder, move |ui, _| {
        let owner = owner as HWND;
        let current = state.lock().unwrap().hint.clone();
        let Some(current) = current else {
            return;
        };
        let Some(group) = current.group.filter(|g| g.id == group.id) else {
            return;
        };
        let ratio = zoom / ui.ctx().zoom_factor();
        let rect = Rect::from_min_size(Pos2::ZERO, size * ratio);
        let pointer_inside = ui.input(|i| i.pointer.hover_pos().is_some_and(|p| rect.contains(p)));
        let mut hover = state.lock().unwrap();
        if hover.popup(pointer_inside, Instant::now()) {
            ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
        }
        drop(hover);
        let selected = body(ui, &group, size, ratio, &icons);
        if let Some(id) = selected {
            state.lock().unwrap().clear();
            dispatch(&sender, ui.ctx(), Action::Activate(id));
        } else if ui.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape)) {
            state.lock().unwrap().clear();
            ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
        }
        if let Some(hwnd) = window(ui.ctx(), "ai-terminal group", "floating-group-hwnd")
            && let Some((bar, work, scale)) = anchor(owner, zoom)
        {
            let dimensions = [
                (size.x * scale).ceil() as i32,
                (size.y * scale).ceil() as i32,
            ];
            let place = origin_for(
                bar,
                bar[0] + (current.anchor.x * scale).round() as i32,
                dimensions,
                work,
                (4.0 * scale).round() as i32,
            );
            unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let required = (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW) as isize;
                let desired = (style | required) & !(WS_EX_TRANSPARENT as isize);
                if desired != style {
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired);
                }
                if GetWindow(hwnd, GW_OWNER) != owner {
                    SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, owner as isize);
                }
                let mut rect = RECT::default();
                if GetWindowRect(hwnd, &mut rect) != 0
                    && [
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                    ] != [place[0], place[1], dimensions[0], dimensions[1]]
                {
                    SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        place[0],
                        place[1],
                        dimensions[0],
                        dimensions[1],
                        SWP_NOACTIVATE,
                    );
                    ui.ctx().request_repaint();
                }
            }
        }
    });
}

fn body(
    ui: &mut egui::Ui,
    group: &crate::floating::HoverGroup,
    size: Vec2,
    ratio: f32,
    icons: &Icons,
) -> Option<String> {
    let rect = Rect::from_min_size(Pos2::ZERO, size * ratio);
    let mut selected = None;
    ui.scope(|ui| {
        ui.set_clip_rect(rect);
        ui.painter().rect_filled(rect, 0, Color32::from_gray(28));
        for font in ui.style_mut().text_styles.values_mut() {
            font.size *= ratio;
        }
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let header = Rect::from_min_size(
            Pos2::new(12.0, 6.0) * ratio,
            Vec2::new(size.x - 24.0, 32.0) * ratio,
        );
        crate::ui_text::paint(
            ui,
            ui.painter(),
            header.left_top(),
            &format!("{} × {}", group.name, group.items.len()),
            13.0 * ratio,
            chrome::SECONDARY,
            header.width(),
        );
        let content = Rect::from_min_max(
            Pos2::new(6.0, 38.0) * ratio,
            rect.max - Vec2::new(6.0, 6.0) * ratio,
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
            egui::ScrollArea::vertical()
                .id_salt(("group-members", &group.id))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (index, item) in group.items.iter().enumerate() {
                        let (row, response) = ui.allocate_exact_size(
                            Vec2::new(content.width(), 48.0 * ratio),
                            Sense::click(),
                        );
                        let active = group.active.as_ref() == Some(&item.id);
                        let fill = chrome::session_fill(item.color, active, response.hovered());
                        ui.painter().rect_filled(row, (5.0 * ratio) as u8, fill);
                        let avatar = Rect::from_center_size(
                            Pos2::new(row.left() + 23.0 * ratio, row.center().y),
                            Vec2::splat(32.0 * ratio),
                        );
                        icons.paint(ui.painter(), avatar, item.agent);
                        sidebar::paint_status(ui, avatar, item.activity, item.unread, fill);
                        let left = row.left() + 46.0 * ratio;
                        crate::ui_text::paint(
                            ui,
                            ui.painter(),
                            Pos2::new(left, row.top() + 6.0 * ratio),
                            &item.name,
                            14.0 * ratio,
                            chrome::TEXT,
                            row.right() - left - 8.0 * ratio,
                        );
                        crate::ui_text::paint(
                            ui,
                            ui.painter(),
                            Pos2::new(left, row.top() + 27.0 * ratio),
                            &format!(
                                "{} · {}",
                                item.agent.label(),
                                if item.agent.is_shell()
                                    && item.activity == crate::status::Activity::Unknown
                                {
                                    group.locale.text("未启动", "Not started")
                                } else {
                                    crate::i18n::activity(group.locale, item.activity)
                                }
                            ),
                            11.0 * ratio,
                            chrome::SECONDARY,
                            row.right() - left - 8.0 * ratio,
                        );
                        if response.clicked() {
                            selected = Some(item.id.clone());
                        }
                        if index + 1 < group.items.len() {
                            let gap = if index == 0 {
                                FIRST_MEMBER_GAP
                            } else {
                                MEMBER_GAP
                            };
                            let (divider, _) = ui.allocate_exact_size(
                                Vec2::new(content.width(), gap * ratio),
                                Sense::hover(),
                            );
                            if index == 0 {
                                ui.painter().hline(
                                    divider.left() + 8.0 * ratio..=divider.right() - 8.0 * ratio,
                                    divider.center().y,
                                    egui::Stroke::new(ratio, chrome::BORDER),
                                );
                            }
                        }
                    }
                });
        });
    });
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        appearance,
        floating::{HoverGroup, Item},
        i18n::Locale,
        model::{Agent, SessionColor},
        status::Activity,
    };

    #[test]
    fn member_cards_keep_normal_hovered_and_active_colors_and_individual_activation() {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        let icons = Icons::new(&ctx);
        let mut group = HoverGroup {
            id: "team".into(),
            name: "项目队伍".into(),
            items: [Some(SessionColor::Green), Some(SessionColor::Blue), None]
                .into_iter()
                .enumerate()
                .map(|(n, color)| Item {
                    id: n.to_string(),
                    name: format!("Member {n}"),
                    agent: Agent::Shell,
                    color,
                    activity: Activity::Completed,
                    unread: true,
                })
                .collect(),
            active: Some("0".into()),
            locale: Locale::ZhCn,
        };
        for scale in [1.0, 1.42, 2.0] {
            for locale in [Locale::ZhCn, Locale::En] {
                group.locale = locale;
                let size = Vec2::new(320.0, content_height(group.items.len()));
                let frame = |events| {
                    let mut selected = None;
                    let output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size * scale)),
                            events,
                            ..Default::default()
                        },
                        |ui| selected = body(ui, &group, size, scale, &icons),
                    );
                    (output, selected)
                };
                frame(vec![egui::Event::PointerGone]);
                let (output, _) = frame(vec![]);
                assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                    egui::Shape::Text(t) if t.galley.job.text == "项目队伍 × 3")));
                let card = |output: &egui::FullOutput, color, active, hovered| {
                    output
                        .shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Rect(r)
                                if r.fill == chrome::session_fill(color, active, hovered)
                                    && (r.rect.height() - 48.0 * scale).abs() < 0.1 =>
                            {
                                Some(r.rect)
                            }
                            _ => None,
                        })
                        .expect("every member retains its sidebar background, even when inactive")
                };
                card(&output, Some(SessionColor::Green), true, false);
                let blue = card(&output, Some(SessionColor::Blue), false, false);
                card(&output, None, false, false);
                let pos = blue.center();
                frame(vec![egui::Event::PointerMoved(pos)]);
                let (hovered, _) = frame(vec![]);
                assert_eq!(card(&hovered, Some(SessionColor::Blue), false, true), blue);
                let button = |pressed| egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                };
                frame(vec![button(true)]);
                assert_eq!(frame(vec![button(false)]).1.as_deref(), Some("1"));
            }
        }
    }
}
