use alacritty_terminal::index::Point as TerminalGridPoint;
use alacritty_terminal::term::cell;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor};
use egui::epaint::RectShape;
use egui::Modifiers;
use egui::MouseWheelUnit;
use egui::Shape;
use egui::Widget;
use egui::{CornerRadius, Key};
use egui::{Id, PointerButton};
use egui::{Painter, Pos2, Rect, Response, Stroke, Vec2};
use std::sync::{Arc, OnceLock};
use unicode_segmentation::UnicodeSegmentation;

use crate::backend::BackendCommand;
use crate::backend::TerminalBackend;
use crate::backend::{LinkAction, MouseButton, SelectionType};
use crate::bindings::Binding;
use crate::bindings::{BindingAction, BindingsLayout, InputKind};
use crate::font::TerminalFont;
use crate::theme::TerminalTheme;
use crate::types::Size;

const EGUI_TERM_WIDGET_ID_PREFIX: &str = "egui_term::instance::";

#[derive(Debug, Clone)]
enum InputAction {
    BackendCall(BackendCommand),
    WriteToClipboard(String),
    Ignore,
}

#[derive(Clone, Default)]
pub struct TerminalViewState {
    preedit: String,
    input_enabled: bool,
    is_dragged: bool,
    scroll_pixels: f32,
    current_mouse_position_on_grid: TerminalGridPoint,
}

pub struct TerminalView<'a> {
    widget_id: Id,
    has_focus: bool,
    focus_requested: bool,
    size: Vec2,
    backend: &'a mut TerminalBackend,
    font: TerminalFont,
    theme: TerminalTheme,
    bindings_layout: Arc<BindingsLayout>,
}

impl Widget for TerminalView<'_> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let (_, rect) = ui.allocate_space(self.size);
        let layout = ui.interact(rect, self.widget_id, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        let widget_id = self.widget_id;
        let mut state = ui.memory(|m| {
            m.data
                .get_temp::<TerminalViewState>(widget_id)
                .unwrap_or_default()
        });

        self.focus(&layout, &mut state)
            .resize(&layout)
            .process_input(&layout, &mut state)
            .show(&mut state, &layout, &painter);

        ui.memory_mut(|m| m.data.insert_temp(widget_id, state));
        layout
    }
}

impl<'a> TerminalView<'a> {
    pub fn new(ui: &mut egui::Ui, backend: &'a mut TerminalBackend) -> Self {
        let widget_id =
            ui.make_persistent_id(format!("{}{}", EGUI_TERM_WIDGET_ID_PREFIX, backend.id()));

        Self {
            widget_id,
            has_focus: false,
            focus_requested: false,
            size: ui.available_size(),
            backend,
            font: TerminalFont::default(),
            theme: TerminalTheme::default(),
            bindings_layout: {
                static DEFAULT: OnceLock<Arc<BindingsLayout>> = OnceLock::new();
                DEFAULT
                    .get_or_init(|| Arc::new(BindingsLayout::new()))
                    .clone()
            },
        }
    }

    #[inline]
    pub fn set_theme(mut self, theme: TerminalTheme) -> Self {
        self.theme = theme;
        self
    }

    #[inline]
    pub fn set_font(mut self, font: TerminalFont) -> Self {
        self.font = font;
        self
    }

    #[inline]
    pub fn set_focus(mut self, has_focus: bool) -> Self {
        self.has_focus = has_focus;
        self
    }

    #[inline]
    pub fn request_focus(mut self, requested: bool) -> Self {
        self.focus_requested = requested;
        self
    }

    #[inline]
    pub fn set_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    #[inline]
    pub fn add_bindings(mut self, bindings: Vec<(Binding<InputKind>, BindingAction)>) -> Self {
        Arc::make_mut(&mut self.bindings_layout).add_bindings(bindings);
        self
    }

    fn focus(self, layout: &Response, state: &mut TerminalViewState) -> Self {
        if self.has_focus {
            // request_focus also interrupts native IME. Never repeat it every repaint.
            if !layout.has_focus()
                && (self.focus_requested
                    || !state.input_enabled
                    || layout.clicked()
                    || layout.ctx.memory(|memory| memory.focused().is_none()))
            {
                layout.request_focus();
            }
            if layout.has_focus() {
                layout.ctx.memory_mut(|memory| {
                    memory.set_focus_lock_filter(
                        layout.id,
                        egui::EventFilter {
                            tab: true,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: true,
                        },
                    )
                });
            }
        } else {
            layout.surrender_focus();
            state.preedit.clear();
        }
        state.input_enabled = self.has_focus;
        self
    }

    fn resize(self, layout: &Response) -> Self {
        self.backend.process_command(BackendCommand::Resize(
            Size::from(layout.rect.size()),
            self.font.font_measure(&layout.ctx),
            layout.ctx.pixels_per_point(),
        ));
        self.backend.sync();

        self
    }

    fn process_input(self, layout: &Response, state: &mut TerminalViewState) -> Self {
        if !layout.has_focus() || !self.has_focus || !layout.ctx.input(|i| i.focused) {
            return self;
        }

        let modifiers = layout.ctx.input(|i| i.modifiers);
        let events = layout.ctx.input(|i| i.events.clone());
        let ime_composing = !state.preedit.is_empty()
            || events
                .iter()
                .any(|event| matches!(event,
                    egui::Event::Ime(egui::ImeEvent::Preedit { text, .. } | egui::ImeEvent::Commit(text)) if !text.is_empty()));
        for event in events {
            let mut input_actions = vec![];

            match event {
                egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                    state.preedit.clear();
                    input_actions.push(InputAction::BackendCall(BackendCommand::Write(
                        text.into_bytes(),
                    )));
                }
                egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => {
                    state.preedit = text;
                }
                #[allow(deprecated)]
                egui::Event::Ime(egui::ImeEvent::Disabled) => state.preedit.clear(),
                egui::Event::Key { .. } | egui::Event::Text(_) if ime_composing => {}
                egui::Event::Text(_)
                | egui::Event::Key { .. }
                | egui::Event::Copy
                | egui::Event::Paste(_) => input_actions.push(process_keyboard_event(
                    event,
                    self.backend,
                    &self.bindings_layout,
                    modifiers,
                )),
                egui::Event::MouseWheel { unit, delta, .. } if layout.contains_pointer() => {
                    input_actions.push(process_mouse_wheel(
                        state,
                        self.font.font_type().size,
                        unit,
                        delta,
                    ))
                }
                egui::Event::PointerButton {
                    button,
                    pressed,
                    modifiers,
                    pos,
                    ..
                } if layout.rect.contains(pos) || state.is_dragged => {
                    input_actions.push(process_button_click(
                        state,
                        layout,
                        self.backend,
                        &self.bindings_layout,
                        button,
                        pos,
                        &modifiers,
                        pressed,
                    ))
                }
                egui::Event::PointerMoved(pos) if layout.rect.contains(pos) || state.is_dragged => {
                    input_actions = process_mouse_move(state, layout, self.backend, pos, &modifiers)
                }
                _ => {}
            };

            for action in input_actions {
                match action {
                    InputAction::BackendCall(cmd) => {
                        self.backend.process_command(cmd);
                    }
                    InputAction::WriteToClipboard(data) => {
                        layout.ctx.copy_text(data);
                    }
                    InputAction::Ignore => {}
                }
            }
        }

        self
    }

    fn show(self, state: &mut TerminalViewState, layout: &Response, painter: &Painter) {
        let (_, baseline) = self.font.metrics(&layout.ctx);
        let scale = layout.ctx.pixels_per_point();
        let snap = |value: f32| (value * scale).round() / scale;
        let content = self.backend.sync();
        let origin = Pos2::new(snap(layout.rect.min.x), snap(layout.rect.min.y));
        let cell_height = content.terminal_size.cell_height;
        let cell_width = content.terminal_size.cell_width;
        let color = |value: Color| {
            let index = match value {
                Color::Named(name) => Some(name as usize),
                Color::Indexed(index) => Some(index as usize),
                Color::Spec(_) => None,
            };
            index
                .and_then(|i| content.colors[i])
                .map(|rgb| egui::Color32::from_rgb(rgb.r, rgb.g, rgb.b))
                .unwrap_or_else(|| self.theme.get_color(value))
        };
        let global_bg = color(Color::Named(NamedColor::Background));
        let cursor_color = content.colors[NamedColor::Cursor]
            .map(|rgb| egui::Color32::from_rgb(rgb.r, rgb.g, rgb.b))
            .unwrap_or_else(|| color(Color::Named(NamedColor::Foreground)));
        let mut backgrounds = vec![Shape::Rect(RectShape::filled(
            layout.rect,
            CornerRadius::ZERO,
            global_bg,
        ))];
        let mut foregrounds = Vec::new();
        let mut cursors = Vec::new();
        let window_focused = layout.ctx.input(|i| i.focused);
        let active = layout.has_focus() && window_focused;
        let cursor_shape = if active || content.cursor_shape == CursorShape::Hidden {
            content.cursor_shape
        } else {
            CursorShape::HollowBlock
        };
        let mut skip_glyph_until = None;
        for (cell_index, indexed) in content.cells.iter().enumerate() {
            let flags = indexed.cell.flags;
            let wide = flags.contains(cell::Flags::WIDE_CHAR);
            let spacer = flags
                .intersects(cell::Flags::WIDE_CHAR_SPACER | cell::Flags::LEADING_WIDE_CHAR_SPACER);
            let x = origin.x + cell_width * indexed.point.column.0 as f32;
            let y = origin.y
                + cell_height * (indexed.point.line.0 + content.display_offset as i32) as f32;
            let width = if wide { cell_width * 2.0 } else { cell_width };
            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, cell_height));
            if !rect.intersects(layout.rect) {
                continue;
            }
            let selected = content
                .selectable_range
                .is_some_and(|r| r.contains(indexed.point));
            let hovered = content.hovered_hyperlink.as_ref().is_some_and(|range| {
                range.contains(&indexed.point)
                    && range.contains(&state.current_mouse_position_on_grid)
            });
            let mut fg = color(indexed.cell.fg);
            let mut bg = color(indexed.cell.bg);
            if flags.contains(cell::Flags::DIM) {
                fg = fg.linear_multiply(0.7);
            }
            if flags.contains(cell::Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }
            if selected {
                std::mem::swap(&mut fg, &mut bg);
            }
            // Paint every background before any glyph: adjacent cells must not erase text.
            if global_bg != bg {
                backgrounds.push(Shape::Rect(RectShape::filled(rect, CornerRadius::ZERO, bg)));
            }
            if spacer {
                continue;
            }
            if content.cursor_point == indexed.point {
                match cursor_shape {
                    CursorShape::Block => {
                        backgrounds.push(Shape::Rect(RectShape::filled(
                            rect,
                            CornerRadius::ZERO,
                            cursor_color,
                        )));
                        fg = global_bg;
                        bg = cursor_color;
                    }
                    CursorShape::Beam => cursors.push(Shape::Rect(RectShape::filled(
                        Rect::from_min_size(rect.min, Vec2::new(1.5, cell_height)),
                        CornerRadius::ZERO,
                        cursor_color,
                    ))),
                    CursorShape::Underline => cursors.push(Shape::Rect(RectShape::filled(
                        Rect::from_min_size(Pos2::new(x, rect.max.y - 1.5), Vec2::new(width, 1.5)),
                        CornerRadius::ZERO,
                        cursor_color,
                    ))),
                    CursorShape::HollowBlock => cursors.push(Shape::Rect(RectShape::stroke(
                        rect.shrink(0.5),
                        CornerRadius::ZERO,
                        Stroke::new(1.0, cursor_color),
                        egui::StrokeKind::Inside,
                    ))),
                    CursorShape::Hidden => {}
                }
            }
            if !flags.contains(cell::Flags::HIDDEN)
                && indexed.cell.c != '\t'
                && !skip_glyph_until.is_some_and(|end| cell_index <= end)
            {
                let (text, end, columns) = glyph_cluster(&content.cells, cell_index);
                if end > cell_index {
                    skip_glyph_until = Some(end);
                }
                let glyph_rect = Rect::from_min_size(
                    rect.min,
                    egui::vec2(columns as f32 * cell_width, cell_height),
                );
                if text != " " {
                    let builtin = indexed
                        .cell
                        .zerowidth()
                        .is_none_or(|chars| chars.is_empty())
                        .then(|| grid_glyph(indexed.cell.c, rect, fg))
                        .flatten();
                    if let Some(shapes) = builtin {
                        foregrounds.extend(shapes);
                    } else {
                        #[cfg(windows)]
                        let native = self.font.native.as_ref().and_then(|font| {
                            crate::native_text::shape(
                                &layout.ctx,
                                font,
                                self.font.font_type().size,
                                &text,
                                glyph_rect,
                                baseline,
                                flags.contains(cell::Flags::BOLD),
                                flags.contains(cell::Flags::ITALIC),
                                fg,
                                bg,
                            )
                        });
                        #[cfg(not(windows))]
                        let native: Option<Shape> = None;
                        if let Some(native) = native {
                            foregrounds.push(native);
                        } else {
                            let font = self.font.styled_font_at_scale(
                                flags.contains(cell::Flags::BOLD),
                                flags.contains(cell::Flags::ITALIC),
                                painter.ctx().pixels_per_point(),
                            );
                            let galley =
                                painter.layout_no_wrap(text, font, egui::Color32::PLACEHOLDER);
                            let glyph_baseline = galley
                                .rows
                                .first()
                                .and_then(|row| {
                                    row.glyphs.first().map(|glyph| row.pos.y + glyph.pos.y)
                                })
                                .unwrap_or(0.0);
                            let inset = if wide {
                                ((width - galley.size().x) / 2.0).max(0.0)
                            } else {
                                0.0
                            };
                            let pos =
                                Pos2::new(snap(x + inset), snap(y + baseline - glyph_baseline));
                            foregrounds.push(Shape::Text(
                                egui::epaint::TextShape::new(pos, galley, fg)
                                    .with_override_text_color(fg),
                            ));
                        }
                    }
                }
            }

            if hovered || flags.intersects(cell::Flags::ALL_UNDERLINES) {
                let line = rect.max.y - 1.5;
                foregrounds.push(Shape::line_segment(
                    [Pos2::new(x, line), Pos2::new(x + width, line)],
                    Stroke::new(1.0, fg),
                ));
            }
            if flags.contains(cell::Flags::STRIKEOUT) {
                foregrounds.push(Shape::line_segment(
                    [
                        Pos2::new(x, rect.center().y),
                        Pos2::new(x + width, rect.center().y),
                    ],
                    Stroke::new(1.0, fg),
                ));
            }
        }
        painter.extend(backgrounds);
        painter.extend(foregrounds);
        painter.extend(cursors);
        if layout.has_focus() && self.has_focus {
            let cursor_pos = Pos2::new(
                origin.x + content.cursor_point.column.0 as f32 * cell_width,
                origin.y
                    + (content.cursor_point.line.0 + content.display_offset as i32) as f32
                        * cell_height,
            );
            layout.ctx.output_mut(|o| {
                o.ime = Some(egui::output::IMEOutput {
                    rect: layout.rect,
                    cursor_rect: Rect::from_min_size(cursor_pos, Vec2::new(1.0, cell_height))
                        .intersect(layout.rect),
                    should_interrupt_composition: false,
                });
            });
            if !state.preedit.is_empty() {
                let galley = painter.layout_no_wrap(
                    state.preedit.clone(),
                    self.font
                        .styled_font_at_scale(false, false, painter.ctx().pixels_per_point()),
                    egui::Color32::WHITE,
                );
                let rect = Rect::from_min_size(
                    cursor_pos,
                    galley.size().max(Vec2::new(cell_width, cell_height)),
                );
                painter.rect_filled(rect, CornerRadius::ZERO, global_bg);
                painter.galley(cursor_pos, galley, egui::Color32::WHITE);
                painter.line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    Stroke::new(1.0, cursor_color),
                );
            }
        }
    }
}

/// Preserve emulator coordinates while joining emoji modifiers, flags and ZWJ sequences for shaping.
fn glyph_cluster(cells: &[crate::backend::RenderCell], index: usize) -> (String, usize, usize) {
    let first = &cells[index];
    let mut text = first.cell.c.to_string();
    if let Some(zero) = first.cell.zerowidth() {
        text.extend(zero);
    }
    let mut end = index;
    let first_width = if first.cell.flags.contains(cell::Flags::WIDE_CHAR) {
        2
    } else {
        1
    };
    let mut columns = first_width;
    if !first.cell.c.is_ascii() {
        loop {
            let mut next = end + 1;
            while next < cells.len()
                && cells[next]
                    .cell
                    .flags
                    .contains(cell::Flags::WIDE_CHAR_SPACER)
            {
                next += 1;
            }
            let Some(candidate) = cells.get(next) else {
                break;
            };
            if candidate.point.line != first.point.line
                || candidate.point.column.0 != first.point.column.0 + columns
                || candidate.cell.c == ' '
                || candidate.cell.fg != first.cell.fg
                || candidate.cell.bg != first.cell.bg
                || candidate.cell.flags != first.cell.flags
            {
                break;
            }
            let mut joined = text.clone();
            joined.push(candidate.cell.c);
            if let Some(zero) = candidate.cell.zerowidth() {
                joined.extend(zero);
            }
            if joined.graphemes(true).count() != 1 {
                break;
            }
            text = joined;
            end = next;
            columns += if candidate.cell.flags.contains(cell::Flags::WIDE_CHAR) {
                2
            } else {
                1
            };
            if text.len() > 128 {
                break;
            }
        }
    }
    let presentation = text.contains('\u{fe0f}');
    let blank_after = cells.get(index + 1).is_some_and(|c| {
        c.point.line == first.point.line
            && c.point.column.0 == first.point.column.0 + 1
            && c.cell.c == ' '
    });
    let visual = if presentation && first_width == 1 && blank_after {
        2
    } else {
        columns.min(2)
    };
    (text, end, visual)
}

fn process_keyboard_event(
    event: egui::Event,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    modifiers: Modifiers,
) -> InputAction {
    match event {
        egui::Event::Text(text) => process_text_event(&text, modifiers),
        egui::Event::Paste(text) => {
            let text = if backend
                .last_content()
                .terminal_mode
                .contains(TermMode::BRACKETED_PASTE)
            {
                format!("\x1b[200~{}\x1b[201~", text.replace('\x1b', ""))
            } else {
                text.replace("\r\n", "\r").replace('\n', "\r")
            };
            InputAction::BackendCall(BackendCommand::Write(text.into_bytes()))
        }
        egui::Event::Copy => {
            #[cfg(not(any(target_os = "ios", target_os = "macos")))]
            {
                let content = backend.selectable_content();
                if modifiers.contains(Modifiers::COMMAND | Modifiers::SHIFT) || !content.is_empty()
                {
                    InputAction::WriteToClipboard(content)
                } else {
                    InputAction::BackendCall(BackendCommand::Write(vec![0x03]))
                }
            }
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            {
                let content = backend.selectable_content();
                InputAction::WriteToClipboard(content)
            }
        }
        egui::Event::Key {
            key,
            pressed,
            modifiers,
            ..
        } => process_keyboard_key(backend, bindings_layout, key, modifiers, pressed),
        _ => InputAction::Ignore,
    }
}

fn process_text_event(text: &str, modifiers: Modifiers) -> InputAction {
    // Text events carry literal text, not key names. Ctrl/Command shortcuts are
    // handled by Key/Copy/Paste; Ctrl+Alt can be AltGr producing normal text.
    if (modifiers.ctrl || modifiers.command) && !modifiers.alt {
        InputAction::Ignore
    } else if modifiers.alt && !modifiers.ctrl {
        InputAction::BackendCall(BackendCommand::Write(format!("\x1b{text}").into_bytes()))
    } else {
        InputAction::BackendCall(BackendCommand::Write(text.as_bytes().to_vec()))
    }
}

fn process_keyboard_key(
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    key: Key,
    modifiers: Modifiers,
    pressed: bool,
) -> InputAction {
    if !pressed {
        return InputAction::Ignore;
    }

    let terminal_mode = backend.last_content().terminal_mode;
    let binding_action =
        bindings_layout.get_action(InputKind::KeyCode(key), modifiers, terminal_mode);

    match binding_action {
        BindingAction::Char(c) => {
            let mut buf = [0, 0, 0, 0];
            let str = c.encode_utf8(&mut buf);
            InputAction::BackendCall(BackendCommand::Write(str.as_bytes().to_vec()))
        }
        BindingAction::Esc(seq) => {
            InputAction::BackendCall(BackendCommand::Write(seq.as_bytes().to_vec()))
        }
        _ => InputAction::Ignore,
    }
}

fn process_mouse_wheel(
    state: &mut TerminalViewState,
    font_size: f32,
    unit: MouseWheelUnit,
    delta: Vec2,
) -> InputAction {
    match unit {
        MouseWheelUnit::Line => {
            let lines = delta.y.signum() * delta.y.abs().ceil();
            InputAction::BackendCall(BackendCommand::Scroll(lines as i32))
        }
        MouseWheelUnit::Point => {
            state.scroll_pixels -= delta.y;
            let lines = (state.scroll_pixels / font_size).trunc();
            state.scroll_pixels %= font_size;
            if lines != 0.0 {
                InputAction::BackendCall(BackendCommand::Scroll(-lines as i32))
            } else {
                InputAction::Ignore
            }
        }
        MouseWheelUnit::Page => InputAction::Ignore,
    }
}

// Keep the upstream input-handler interface intact.
#[allow(clippy::too_many_arguments)]
fn process_button_click(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    button: PointerButton,
    position: Pos2,
    modifiers: &Modifiers,
    pressed: bool,
) -> InputAction {
    match button {
        PointerButton::Primary => process_left_button(
            state,
            layout,
            backend,
            bindings_layout,
            position,
            modifiers,
            pressed,
        ),
        _ => InputAction::Ignore,
    }
}

fn process_left_button(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    position: Pos2,
    modifiers: &Modifiers,
    pressed: bool,
) -> InputAction {
    let terminal_mode = backend.last_content().terminal_mode;
    if terminal_mode.intersects(TermMode::MOUSE_MODE) {
        InputAction::BackendCall(BackendCommand::MouseReport(
            MouseButton::LeftButton,
            *modifiers,
            state.current_mouse_position_on_grid,
            pressed,
        ))
    } else if pressed {
        process_left_button_pressed(state, layout, position)
    } else {
        process_left_button_released(state, layout, backend, bindings_layout, position, modifiers)
    }
}

fn process_left_button_pressed(
    state: &mut TerminalViewState,
    layout: &Response,
    position: Pos2,
) -> InputAction {
    state.is_dragged = true;
    InputAction::BackendCall(build_start_select_command(layout, position))
}

fn process_left_button_released(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    position: Pos2,
    modifiers: &Modifiers,
) -> InputAction {
    state.is_dragged = false;
    if layout.double_clicked() || layout.triple_clicked() {
        InputAction::BackendCall(build_start_select_command(layout, position))
    } else {
        let terminal_content = backend.last_content();
        let binding_action = bindings_layout.get_action(
            InputKind::Mouse(PointerButton::Primary),
            *modifiers,
            terminal_content.terminal_mode,
        );

        if binding_action == BindingAction::LinkOpen {
            InputAction::BackendCall(BackendCommand::ProcessLink(
                LinkAction::Open,
                state.current_mouse_position_on_grid,
            ))
        } else {
            InputAction::Ignore
        }
    }
}

fn build_start_select_command(layout: &Response, cursor_position: Pos2) -> BackendCommand {
    let selection_type = if layout.double_clicked() {
        SelectionType::Semantic
    } else if layout.triple_clicked() {
        SelectionType::Lines
    } else {
        SelectionType::Simple
    };

    BackendCommand::SelectStart(
        selection_type,
        cursor_position.x - layout.rect.min.x,
        cursor_position.y - layout.rect.min.y,
    )
}

fn process_mouse_move(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    position: Pos2,
    modifiers: &Modifiers,
) -> Vec<InputAction> {
    let terminal_content = backend.last_content();
    let cursor_x = position.x - layout.rect.min.x;
    let cursor_y = position.y - layout.rect.min.y;
    state.current_mouse_position_on_grid = TerminalBackend::selection_point(
        cursor_x,
        cursor_y,
        &terminal_content.terminal_size,
        terminal_content.display_offset,
    );

    let mut actions = vec![];
    // Handle command or selection update based on terminal mode and modifiers
    if state.is_dragged {
        let terminal_mode = terminal_content.terminal_mode;
        let cmd = if terminal_mode.contains(TermMode::MOUSE_MOTION) && modifiers.is_none() {
            InputAction::BackendCall(BackendCommand::MouseReport(
                MouseButton::LeftMove,
                *modifiers,
                state.current_mouse_position_on_grid,
                true,
            ))
        } else {
            InputAction::BackendCall(BackendCommand::SelectUpdate(cursor_x, cursor_y))
        };

        actions.push(cmd);
    }

    // Handle link hover if applicable
    if modifiers.command_only() {
        actions.push(InputAction::BackendCall(BackendCommand::ProcessLink(
            LinkAction::Hover,
            state.current_mouse_position_on_grid,
        )));
    }

    actions
}

// Terminal box rules and block art must meet the cell edges. Font fallback and
// glyph advances otherwise leave seams in TUI frames and Claude's block mascot.
fn grid_glyph(ch: char, rect: Rect, color: egui::Color32) -> Option<Vec<Shape>> {
    let mut shapes = Vec::new();
    let connections = match ch {
        '─' => Some((true, true, false, false)),
        '│' => Some((false, false, true, true)),
        '┌' | '╭' => Some((false, true, false, true)),
        '┐' | '╮' => Some((true, false, false, true)),
        '└' | '╰' => Some((false, true, true, false)),
        '┘' | '╯' => Some((true, false, true, false)),
        '├' => Some((false, true, true, true)),
        '┤' => Some((true, false, true, true)),
        '┬' => Some((true, true, false, true)),
        '┴' => Some((true, true, true, false)),
        '┼' => Some((true, true, true, true)),
        _ => None,
    };
    if let Some((left, right, up, down)) = connections {
        let middle = rect.center();
        for (enabled, end) in [
            (left, rect.left_center()),
            (right, rect.right_center()),
            (up, rect.center_top()),
            (down, rect.center_bottom()),
        ] {
            if enabled {
                shapes.push(Shape::line_segment([middle, end], Stroke::new(1.0, color)));
            }
        }
        return Some(shapes);
    }
    let fraction = |x0: f32, y0: f32, x1: f32, y1: f32| {
        Shape::Rect(RectShape::filled(
            Rect::from_min_max(
                egui::pos2(
                    rect.left() + x0 * rect.width(),
                    rect.top() + y0 * rect.height(),
                ),
                egui::pos2(
                    rect.left() + x1 * rect.width(),
                    rect.top() + y1 * rect.height(),
                ),
            ),
            CornerRadius::ZERO,
            color,
        ))
    };
    let code = ch as u32;
    match ch {
        '▀' => shapes.push(fraction(0.0, 0.0, 1.0, 0.5)),
        '▁'..='█' => shapes.push(fraction(0.0, 1.0 - (code - 0x2580) as f32 / 8.0, 1.0, 1.0)),
        '▉'..='▏' => shapes.push(fraction(0.0, 0.0, (0x2590 - code) as f32 / 8.0, 1.0)),
        '▐' => shapes.push(fraction(0.5, 0.0, 1.0, 1.0)),
        '▔' => shapes.push(fraction(0.0, 0.0, 1.0, 0.125)),
        '▕' => shapes.push(fraction(0.875, 0.0, 1.0, 1.0)),
        '▖'..='▟' => {
            let mask = match ch {
                '▖' => 4,
                '▗' => 8,
                '▘' => 1,
                '▙' => 13,
                '▚' => 9,
                '▛' => 7,
                '▜' => 11,
                '▝' => 2,
                '▞' => 6,
                '▟' => 14,
                _ => 0,
            };
            for (bit, x, y) in [(1, 0.0, 0.0), (2, 0.5, 0.0), (4, 0.0, 0.5), (8, 0.5, 0.5)] {
                if mask & bit != 0 {
                    shapes.push(fraction(x, y, x + 0.5, y + 0.5));
                }
            }
        }
        _ => return None,
    }
    Some(shapes)
}

#[cfg(test)]
mod rendering_tests {
    use super::*;
    #[test]
    fn terminal_rules_and_blocks_fill_their_cells() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(9.0, 24.0));
        let line = grid_glyph('─', rect, egui::Color32::WHITE).unwrap();
        assert!(line.iter().any(
            |s| matches!(s,Shape::LineSegment{points,..} if points.contains(&rect.left_center()))
        ));
        assert!(line.iter().any(
            |s| matches!(s,Shape::LineSegment{points,..} if points.contains(&rect.right_center()))
        ));
        let block = grid_glyph('█', rect, egui::Color32::WHITE).unwrap();
        assert!(matches!(&block[0],Shape::Rect(shape) if shape.rect==rect));
        assert!(grid_glyph('你', rect, egui::Color32::WHITE).is_none());
    }
}
