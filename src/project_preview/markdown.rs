//! Native document layout around the CommonMark prose renderer.
use super::*;
use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, TagEnd};
use std::{cell::RefCell, rc::Rc};

pub(super) fn show(
    ui: &mut egui::Ui,
    cache: &mut CommonMarkCache,
    document: Arc<Document>,
    anchor: Option<&str>,
) -> Vec<String> {
    let md = document.markdown.as_ref().unwrap();
    let clicked = Rc::new(RefCell::new(vec![]));
    let handler_clicked = clicked.clone();
    let handler_document = document.clone();
    let anchor = anchor.map(str::to_owned);
    let html = move |ui: &mut egui::Ui, raw: &str| {
        block(
            ui,
            raw,
            handler_document.markdown.as_ref().unwrap(),
            &handler_clicked,
            anchor.as_deref(),
            false,
            0,
        )
    };
    CommonMarkViewer::new()
        .explicit_image_uri_scheme(true)
        .render_html_fn(Some(&html))
        .show(ui, cache, &md.text);
    let mut result = clicked.borrow_mut().drain(..).collect::<Vec<_>>();
    result.extend(
        md.links
            .iter()
            .filter(|link| cache.get_link_hook(link) == Some(true))
            .cloned(),
    );
    result
}
fn block(
    ui: &mut egui::Ui,
    raw: &str,
    md: &Markdown,
    clicked: &RefCell<Vec<String>>,
    anchor: Option<&str>,
    center: bool,
    depth: usize,
) {
    if depth > 16 {
        return;
    }
    let Some(key) = raw
        .trim()
        .strip_prefix(&format!("<{}-", md.code_marker))
        .and_then(|s| s.strip_suffix(" />"))
    else {
        return;
    };
    if let Some(heading) = key
        .strip_prefix("heading-")
        .and_then(|s| s.parse::<usize>().ok())
        .and_then(|i| md.headings.get(i))
    {
        ui.vertical(|ui| {
            ui.add_space(if heading.level <= 2 { 16.0 } else { 10.0 });
            let base = egui::TextStyle::Body.resolve(ui.style()).size;
            let scale = match heading.level {
                1 => 2.0,
                2 => 1.55,
                3 => 1.25,
                _ => 1.1,
            };
            let response = inline(
                ui,
                &heading.text,
                md,
                clicked,
                center,
                Some(base * scale),
                true,
            );
            if anchor == Some(heading.anchor.as_str()) {
                response.scroll_to_me_animation(
                    Some(egui::Align::TOP),
                    egui::style::ScrollAnimation::none(),
                );
            }
            if heading.level <= 2 {
                ui.add_space(5.0);
                ui.separator();
            }
            ui.add_space(8.0);
        });
    } else if let Some(table) = key
        .strip_prefix("table-")
        .and_then(|s| s.parse::<usize>().ok())
        .and_then(|i| md.tables.get(i))
    {
        table_ui(ui, key, table, md, clicked);
    } else if let Some(body) = key
        .strip_prefix("center-")
        .and_then(|s| s.parse::<usize>().ok())
        .and_then(|i| md.centered.get(i))
    {
        centered(ui, body, md, clicked, anchor, depth + 1);
    } else if let Ok(index) = key.parse::<usize>()
        && let Some(code) = md.code.get(index)
        && let Some(source) = md.code_sources.get(index)
    {
        super::code_block(ui, code, source, egui::Id::new((&md.code_marker, index)));
    }
}
fn centered(
    ui: &mut egui::Ui,
    body: &str,
    md: &Markdown,
    clicked: &RefCell<Vec<String>>,
    anchor: Option<&str>,
    depth: usize,
) {
    ui.vertical(|ui| {
        ui.set_min_width(ui.available_width());
        let mut start = None;
        for (event, span) in Parser::new_ext(body, Options::ENABLE_STRIKETHROUGH).into_offset_iter()
        {
            match event {
                Event::Start(Tag::Paragraph) => start = Some(span.start),
                Event::End(TagEnd::Paragraph) => {
                    if let Some(begin) = start.take() {
                        inline(ui, &body[begin..span.end], md, clicked, true, None, false);
                        ui.add_space(12.0);
                    }
                }
                Event::Html(raw) => block(ui, &raw, md, clicked, anchor, true, depth),
                Event::Rule => {
                    ui.separator();
                }
                _ => {}
            }
        }
    });
}
fn table_ui(
    ui: &mut egui::Ui,
    id: &str,
    table: &project_media::MarkdownTable,
    md: &Markdown,
    clicked: &RefCell<Vec<String>>,
) {
    let count = table.alignment.len();
    if count == 0 {
        return;
    }
    let available = ui.available_width().max(1.0);
    let width = available.max(count as f32 * 100.0);
    let weights: Vec<_> = (0..count)
        .map(|col| {
            table
                .rows
                .iter()
                .filter_map(|row| row.get(col))
                .map(|cell| {
                    let mut image = false;
                    Parser::new(cell)
                        .filter_map(|event| match event {
                            Event::Start(Tag::Image { .. }) => {
                                image = true;
                                None
                            }
                            Event::End(TagEnd::Image) => {
                                image = false;
                                None
                            }
                            Event::Text(s) | Event::Code(s) if !image => Some(s.chars().count()),
                            _ => None,
                        })
                        .sum::<usize>()
                })
                .max()
                .unwrap_or(0)
                .clamp(5, 60) as f32
        })
        .collect();
    let total: f32 = weights.iter().sum();
    let widths: Vec<_> = weights
        .iter()
        .map(|weight| 54.0 + (width - 54.0 * count as f32) * weight / total)
        .collect();
    ui.vertical(|ui| {
        ui.add_space(8.0);
        egui::ScrollArea::horizontal()
            .id_salt(id)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (row_index, row) in table.rows.iter().enumerate() {
                        let mut height = 36.0_f32;
                        for (column, cell_width) in widths.iter().enumerate() {
                            let mut measure = ui.new_child(
                                egui::UiBuilder::new()
                                    .id_salt((id, row_index, column, "measure"))
                                    .invisible()
                                    .sizing_pass(),
                            );
                            measure.set_width((cell_width - 20.0).max(1.0));
                            height = height.max(
                                inline(
                                    &mut measure,
                                    row.get(column).map_or("", String::as_str),
                                    md,
                                    &RefCell::new(vec![]),
                                    false,
                                    None,
                                    row_index == 0,
                                )
                                .rect
                                .height()
                                    + 16.0,
                            );
                        }
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                        let fill = if row_index % 2 == 0 {
                            Color32::from_rgb(19, 22, 27)
                        } else {
                            Color32::from_rgb(13, 16, 20)
                        };
                        ui.painter().rect_filled(rect, 0.0, fill);
                        let stroke = egui::Stroke::new(1.0, Color32::from_gray(55));
                        ui.painter()
                            .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
                        let mut x = rect.left();
                        for (column, cell_width) in widths.iter().enumerate() {
                            let max_rect = egui::Rect::from_min_size(
                                egui::pos2(x + 10.0, rect.top() + 8.0),
                                egui::vec2((cell_width - 20.0).max(1.0), height - 16.0),
                            );
                            let mut cell_ui = ui.new_child(
                                egui::UiBuilder::new()
                                    .id_salt((id, row_index, column))
                                    .max_rect(max_rect)
                                    .layout(egui::Layout::top_down(egui::Align::Min)),
                            );
                            inline(
                                &mut cell_ui,
                                row.get(column).map_or("", String::as_str),
                                md,
                                clicked,
                                table.alignment[column] == Alignment::Center || row_index == 0,
                                None,
                                row_index == 0,
                            );
                            x += cell_width;
                            if column + 1 < count {
                                ui.painter().vline(x, rect.y_range(), stroke);
                            }
                        }
                    }
                });
            });
        ui.add_space(12.0);
    });
}

fn inline(
    ui: &mut egui::Ui,
    text: &str,
    md: &Markdown,
    clicked: &RefCell<Vec<String>>,
    center: bool,
    font_size: Option<f32>,
    strong: bool,
) -> egui::Response {
    if center {
        let available = ui.available_width();
        let mut measure = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("inline-measure")
                .invisible()
                .sizing_pass(),
        );
        let measured = inline(
            &mut measure,
            text,
            md,
            &RefCell::new(vec![]),
            false,
            font_size,
            strong,
        );
        let gutter = ((available - measured.rect.width()) / 2.0).max(0.0);
        return ui
            .horizontal(|ui| {
                ui.add_space(gutter);
                inline(ui, text, md, clicked, false, font_size, strong);
            })
            .response;
    }
    let align = if center {
        egui::Align::Center
    } else {
        egui::Align::Min
    };
    let layout = egui::Layout::left_to_right(egui::Align::Center)
        .with_main_wrap(true)
        .with_main_align(align);
    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 0.0), layout, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let mut bold = strong;
        let mut emphasis = false;
        let mut strike = false;
        let mut link: Option<(String, String)> = None;
        let mut image: Option<(String, String)> = None;
        let rich =
            |ui: &egui::Ui, value: &str, bold: bool, emphasis: bool, strike: bool, code: bool| {
                let mut rich = egui::RichText::new(value);
                if let Some(size) = font_size {
                    rich = rich.size(size);
                }
                if bold {
                    rich = rich.strong();
                }
                if emphasis {
                    rich = rich.italics();
                }
                if strike {
                    rich = rich.strikethrough();
                }
                if code {
                    rich = rich
                        .monospace()
                        .background_color(ui.visuals().code_bg_color);
                }
                rich
            };
        for event in Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH) {
            let is_code = matches!(&event, Event::Code(_));
            match event {
                Event::Start(Tag::Strong) => bold = true,
                Event::End(TagEnd::Strong) => bold = strong,
                Event::Start(Tag::Emphasis) => emphasis = true,
                Event::End(TagEnd::Emphasis) => emphasis = false,
                Event::Start(Tag::Strikethrough) => strike = true,
                Event::End(TagEnd::Strikethrough) => strike = false,
                Event::Start(Tag::Link { dest_url, .. }) => {
                    link = Some((dest_url.into_string(), String::new()))
                }
                Event::End(TagEnd::Link) => {
                    if let Some((url, title)) = link.take()
                        && !title.is_empty()
                    {
                        let response = ui.link(
                            rich(ui, &title, bold, emphasis, strike, false)
                                .color(Color32::from_rgb(100, 170, 245)),
                        );
                        if response.clicked() {
                            clicked.borrow_mut().push(url.clone());
                        }
                        response.on_hover_text(url);
                    }
                }
                Event::Start(Tag::Image { dest_url, .. }) => {
                    image = Some((dest_url.into_string(), String::new()))
                }
                Event::End(TagEnd::Image) => {
                    if let Some((url, alt)) = image.take() {
                        let response = if md.images.get(&url).is_some_and(Result::is_ok) {
                            ui.add(
                                egui::Image::from_uri(&url)
                                    .fit_to_original_size(1.0)
                                    .max_width(ui.available_width().max(1.0))
                                    .sense(egui::Sense::click()),
                            )
                        } else {
                            ui.add(
                                egui::Label::new(egui::RichText::new(&alt).weak())
                                    .wrap()
                                    .sense(egui::Sense::click()),
                            )
                        };
                        if response.clicked()
                            && let Some((target, _)) = &link
                        {
                            clicked.borrow_mut().push(target.clone());
                        }
                        response.on_hover_text(alt);
                    }
                }
                Event::Text(value) | Event::Code(value) => {
                    if let Some((_, alt)) = &mut image {
                        alt.push_str(&value);
                    } else if let Some((_, title)) = &mut link {
                        title.push_str(&value);
                    } else {
                        ui.add(
                            egui::Label::new(rich(ui, &value, bold, emphasis, strike, is_code))
                                .selectable(true)
                                .wrap(),
                        );
                    }
                }
                Event::SoftBreak => {
                    ui.label(" ");
                }
                Event::HardBreak => {
                    ui.label("\n");
                }
                _ => {}
            }
        }
    })
    .response
}
