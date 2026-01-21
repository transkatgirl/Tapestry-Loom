use std::{
    cell::RefCell,
    collections::HashMap,
    ops::Range,
    rc::Rc,
    time::{Duration, Instant},
};

use eframe::{
    egui::{
        Color32, Frame, Galley, Mesh, Pos2, Rect, ScrollArea, TextBuffer, TextEdit, TextFormat,
        TextStyle, Tooltip, Ui, Vec2,
        text::{CCursor, CCursorRange, LayoutJob, LayoutSection, TextWrapping},
    },
    epaint::{MarginF32, Vertex, WHITE_UV},
};
use egui_notify::Toasts;
use flagset::FlagSet;
use tapestry_weave::{ulid::Ulid, universal_weave::indexmap::IndexMap, v0::InnerNodeContent};

use crate::{
    editor::shared::{
        NodeIndex, SharedState, get_node_color, get_token_color, render_node_metadata_tooltip,
        render_token_tooltip, weave::WeaveWrapper,
    },
    settings::{Settings, shortcuts::Shortcuts},
};

#[derive(Debug)]
pub struct TextEditorView {
    text: String,
    bytes: Rc<RefCell<Vec<u8>>>,
    buffer: Vec<u8>,
    snippets: Rc<RefCell<Vec<Snippet>>>,
    node_snippets: HashMap<Ulid, Vec<Range<usize>>>,
    rects: Vec<(Rect, Color32)>,
    last_seen_cursor_node: NodeIndex,
    last_seen_hovered_node: NodeIndex,
    last_text_edit_cursor: Option<CCursor>,
    last_text_edit_hover: Option<Vec2>,
    last_text_edit_highlighting_hover: HighlightingHover,
    last_text_edit_highlighting_hover_update: Instant,
    last_text_edit_rect: Rect,
    text_edit_last_changed: bool,
    should_update_rects: bool,
}

// TODO: Implement a context menu on the TextEdit
// Currently stuck on lacking APIs in egui; see https://github.com/emilk/egui/issues/4393

// TODO: Implement Ctrl+F in TextEdit

type Snippet = (usize, Ulid, Color32, Option<usize>);

const SUBSTITUTION_CHAR: char = '\u{1A}'; //Must be 1 UTF-8 byte in length
const SUBSTITUTION_BYTE: u8 = "\u{1A}".as_bytes()[0];

impl Default for TextEditorView {
    fn default() -> Self {
        debug_assert_eq!(SUBSTITUTION_CHAR.to_string().len(), 1);
        debug_assert_eq!(
            SUBSTITUTION_CHAR.to_string().as_bytes()[0],
            SUBSTITUTION_BYTE
        );

        Self {
            text: String::with_capacity(131072),
            bytes: Rc::new(RefCell::new(Vec::with_capacity(131072))),
            buffer: Vec::with_capacity(131072),
            snippets: Rc::new(RefCell::new(Vec::with_capacity(16384))),
            node_snippets: HashMap::with_capacity(16384),
            rects: Vec::with_capacity(16384),
            last_seen_cursor_node: NodeIndex::None,
            last_seen_hovered_node: NodeIndex::None,
            last_text_edit_cursor: None,
            last_text_edit_hover: None,
            last_text_edit_highlighting_hover: HighlightingHover::None,
            last_text_edit_highlighting_hover_update: Instant::now(),
            last_text_edit_rect: Rect {
                min: Pos2::ZERO,
                max: Pos2::ZERO,
            },
            text_edit_last_changed: false,
            should_update_rects: false,
        }
    }
}

impl TextEditorView {
    /*pub fn reset(&mut self) {
        self.text.clear();
        self.bytes.borrow_mut().clear();
        self.buffer.clear();
        self.snippets.borrow_mut().clear();
        self.node_snippets.clear();
        self.rects.clear();
        self.last_seen_cursor_node = NodeIndex::None;
        self.last_seen_hovered_node = NodeIndex::None;
        self.last_text_edit_cursor = None;
        self.last_text_edit_hover = None;
        self.last_text_edit_highlighting_hover = HighlightingHover::None;
        self.last_text_edit_highlighting_hover_update = Instant::now();
        self.text_edit_last_changed = false;
    }*/
    pub fn update(
        &mut self,
        _weave: &mut WeaveWrapper,
        _settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        if state.has_weave_changed || state.has_theme_changed {
            self.text.clear();
            self.should_update_rects = true;
        }
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        let available_height = ui.available_height();

        if self.text.is_empty() {
            self.update_contents(weave, settings, ui.visuals().widgets.inactive.text_color());
        }

        let snippets = self.snippets.clone();
        let hover = self.last_text_edit_highlighting_hover;
        let bytes = self.bytes.clone();

        let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
            let mut font_id = TextStyle::Monospace.resolve(ui.style());
            font_id.size *= 1.1;

            let layout_job = LayoutJob {
                sections: calculate_highlighting(
                    ui,
                    &snippets.borrow(),
                    buf.as_str().len(),
                    ui.visuals().widgets.inactive.text_color(),
                    hover,
                    &bytes.borrow(),
                    buf.as_str(),
                ),
                text: buf.as_str().to_string(),
                wrap: TextWrapping {
                    max_width: wrap_width,
                    ..Default::default()
                },
                break_on_newline: true,
                ..Default::default()
            };
            ui.fonts_mut(|f| f.layout_job(layout_job))
        };

        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(MarginF32::same(ui.style().spacing.menu_spacing / 2.0))
                    .show(ui, |ui| {
                        let mut font_id = TextStyle::Monospace.resolve(ui.style());
                        font_id.size *= 1.1;
                        ui.style_mut().override_font_id = Some(font_id);

                        render_rects(ui, &self.rects);

                        let mut textedit = TextEdit::multiline(&mut self.text)
                            .frame(false)
                            .text_color(ui.visuals().widgets.inactive.text_color())
                            .min_size(ui.available_size())
                            .desired_width(ui.available_size().x)
                            .code_editor()
                            .layouter(&mut layouter)
                            .show(ui);

                        if self.last_seen_cursor_node != state.get_cursor_node() {
                            // TODO: Rewrite this to properly change the cursor position
                            if !(textedit.response.changed() || self.text_edit_last_changed) {
                                let index = self.text.chars().count();
                                textedit.state.cursor.set_char_range(Some(CCursorRange {
                                    primary: CCursor {
                                        index,
                                        prefer_next_row: true,
                                    },
                                    secondary: CCursor {
                                        index,
                                        prefer_next_row: true,
                                    },
                                    h_pos: None,
                                }));
                                textedit.state.store(ui.ctx(), textedit.response.id);
                            }
                            self.last_seen_cursor_node = state.get_cursor_node();
                        }

                        let top_left = textedit.text_clip_rect.left_top();

                        let hover_position = textedit.response.hover_pos().map(|p| p - top_left);
                        let is_cursor_within_bounds =
                            ui.rect_contains_pointer(textedit.response.rect);

                        if self.rects.is_empty()
                            || self.should_update_rects
                            || textedit.response.changed()
                            || textedit.response.rect != self.last_text_edit_rect
                            || (state.get_changed_node().is_some()
                                && settings.interface.auto_scroll
                                && !is_cursor_within_bounds)
                        {
                            self.rects.clear();
                            calculate_boundaries_and_update_scroll(
                                ui,
                                &self.snippets.borrow(),
                                top_left,
                                &textedit.galley,
                                if settings.interface.auto_scroll && !is_cursor_within_bounds {
                                    state.get_changed_node()
                                } else {
                                    None
                                },
                                available_height,
                                &mut self.rects,
                            );
                            /*absolute_snippet_positions(
                                &self.snippets.borrow(),
                                top_left,
                                &textedit.galley,
                                |snippet, bounds, mut start| {
                                    /*ui.painter().rect_stroke(
                                        bounds,
                                        0.0,
                                        (1.0, eframe::egui::Color32::WHITE),
                                        eframe::egui::StrokeKind::Inside,
                                    );*/

                                    start.max.x += 1.0;

                                    ui.painter().rect_stroke(
                                        start,
                                        0.0,
                                        (1.0, eframe::egui::Color32::GREEN),
                                        eframe::egui::StrokeKind::Inside,
                                    );
                                },
                            );*/
                            self.last_text_edit_rect = textedit.response.rect;
                            self.should_update_rects = false;
                            if !self.rects.is_empty() {
                                ui.ctx().request_repaint();
                            }
                        }

                        if textedit.response.changed() {
                            self.update_weave(state, weave);
                            self.last_text_edit_cursor = None;
                            self.last_text_edit_highlighting_hover_update = Instant::now();
                        } else {
                            let position = textedit.cursor_range.map(|c| c.sorted_cursors()[0]);
                            if position != self.last_text_edit_cursor {
                                if position.is_some() {
                                    if let Some((cursor_node, raw_cursor_index)) =
                                        self.calculate_cursor(weave, position.map(|p| p.index))
                                    {
                                        if let Some(cursor_index) = calculate_cursor_index(
                                            cursor_node,
                                            raw_cursor_index,
                                            &self.node_snippets,
                                        ) {
                                            state.set_cursor_node(NodeIndex::WithinNode(
                                                cursor_node,
                                                cursor_index,
                                            ));
                                            self.last_seen_cursor_node =
                                                NodeIndex::WithinNode(cursor_node, cursor_index);
                                        } else {
                                            state.set_cursor_node(NodeIndex::Node(cursor_node));
                                            self.last_seen_cursor_node =
                                                NodeIndex::Node(cursor_node);
                                        }
                                    } else {
                                        state.set_cursor_node(NodeIndex::None);
                                        self.last_seen_cursor_node = NodeIndex::None;
                                    }
                                }
                                self.last_text_edit_cursor = position;
                            }
                        }

                        if hover_position != self.last_text_edit_hover {
                            if let Some(hover_position) = hover_position
                                && cursor_is_within_galley(
                                    &textedit.galley,
                                    hover_position.to_pos2(),
                                )
                            {
                                let hover_index =
                                    textedit.galley.cursor_from_pos(hover_position).index;
                                let highlighting_hover = self
                                    .calculate_cursor(weave, Some(hover_index))
                                    .map(|(id, i)| HighlightingHover::Position((id, i)))
                                    .unwrap_or(HighlightingHover::None);

                                if highlighting_hover != self.last_text_edit_highlighting_hover {
                                    self.last_text_edit_highlighting_hover = highlighting_hover;
                                    self.last_text_edit_highlighting_hover_update = Instant::now();
                                }
                            } else {
                                self.last_text_edit_highlighting_hover = HighlightingHover::None;
                            }

                            self.last_text_edit_hover = hover_position;
                        }

                        ui.style_mut().override_font_id = None;

                        if let HighlightingHover::Position((hover_node, hover_index)) =
                            self.last_text_edit_highlighting_hover
                        {
                            let since_last_update = self
                                .last_text_edit_highlighting_hover_update
                                .elapsed()
                                .as_secs_f32();
                            let show_tooltip =
                                since_last_update >= ui.style().interaction.tooltip_delay;

                            if !show_tooltip {
                                ui.ctx().request_repaint_after(Duration::from_secs_f32(
                                    (ui.style().interaction.tooltip_delay - since_last_update)
                                        + (1.0 / 15.0),
                                ));
                            }

                            let mut tooltip = Tooltip::for_widget(&textedit.response).at_pointer();
                            tooltip.popup = tooltip.popup.open(show_tooltip);
                            tooltip.show(|ui| {
                                render_tooltip(
                                    ui,
                                    weave,
                                    &self.node_snippets,
                                    hover_node,
                                    hover_index,
                                );
                            });

                            if let Some(corrected_hover_index) =
                                calculate_cursor_index(hover_node, hover_index, &self.node_snippets)
                            {
                                state.set_hovered_node(NodeIndex::WithinNode(
                                    hover_node,
                                    corrected_hover_index,
                                ));
                                self.last_seen_hovered_node =
                                    NodeIndex::WithinNode(hover_node, corrected_hover_index);
                            } else {
                                state.set_hovered_node(NodeIndex::Node(hover_node));
                                self.last_seen_hovered_node = NodeIndex::Node(hover_node);
                            }
                        } else if self.last_seen_hovered_node != state.get_hovered_node() {
                            self.last_text_edit_highlighting_hover = state
                                .get_hovered_node()
                                .into_node()
                                .map(HighlightingHover::Node)
                                .unwrap_or(HighlightingHover::None);
                            self.last_seen_hovered_node = state.get_hovered_node();
                        }

                        self.text_edit_last_changed = textedit.response.changed();
                    });
            });
    }
    fn update_contents(
        &mut self,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        default_color: Color32,
    ) {
        let mut bytes = self.bytes.borrow_mut();
        self.text.clear();
        bytes.clear();
        let mut snippets = self.snippets.borrow_mut();
        snippets.clear();
        self.node_snippets.clear();

        let active: Vec<u128> = weave.get_active_thread_u128().collect();

        let mut offset = 0;

        for node in active
            .into_iter()
            .rev()
            .filter_map(|id| weave.get_node_u128(&id))
        {
            let color = get_node_color(node, settings).unwrap_or(default_color);

            match &node.contents.content {
                InnerNodeContent::Snippet(snippet) => {
                    bytes.extend_from_slice(snippet);
                    snippets.push((snippet.len(), Ulid(node.id), color, None));
                    #[allow(clippy::single_range_in_vec_init)]
                    self.node_snippets
                        .insert(Ulid(node.id), vec![offset..offset + snippet.len()]);
                    offset += snippet.len();
                }
                InnerNodeContent::Tokens(tokens) => {
                    let mut token_index = 0;
                    let mut token_indices = Vec::with_capacity(tokens.len());

                    for (token, token_metadata) in tokens {
                        let color = get_token_color(color, token_metadata, settings)
                            .unwrap_or(default_color);

                        bytes.extend_from_slice(token);
                        snippets.push((token.len(), Ulid(node.id), color, Some(token_index)));
                        token_indices.push(offset..offset + token.len());
                        token_index += token.len();
                        offset += token.len();
                    }

                    self.node_snippets.insert(Ulid(node.id), token_indices);
                }
            }
        }

        for chunk in bytes.utf8_chunks() {
            self.text.push_str(chunk.valid());

            for _ in chunk.invalid() {
                self.text.push(SUBSTITUTION_CHAR);
            }
        }
    }
    fn calculate_cursor(
        &mut self,
        weave: &mut WeaveWrapper,
        char_position: Option<usize>,
    ) -> Option<(Ulid, usize)> {
        let mut cursor_node = None;
        let snippets = self.snippets.borrow();

        if let Some(char_index) = char_position {
            let index = self.text.byte_index_from_char_index(char_index);

            let mut offset = 0;

            for (length, node, _, _) in snippets.iter() {
                offset += length;
                if offset >= index {
                    cursor_node = Some((*node, index));
                    if offset > index {
                        break;
                    }
                }
            }
        } else if let Some(active) = weave.get_active_thread_first() {
            cursor_node = Some((active, self.text.len()));
        } else {
            cursor_node = None;
        }

        cursor_node
    }
    fn update_weave(&mut self, state: &mut SharedState, weave: &mut WeaveWrapper) {
        self.buffer.clear();
        self.buffer.extend_from_slice(self.text.as_bytes());

        let bytes = self.bytes.borrow();

        for (index, byte) in self.buffer.iter_mut().take(bytes.len()).enumerate() {
            if *byte == SUBSTITUTION_BYTE {
                *byte = bytes[index];
            }
        }

        weave.set_active_content(&self.buffer, IndexMap::default());
        state.set_cursor_node(NodeIndex::None);
    }
}

fn calculate_highlighting(
    ui: &Ui,
    snippets: &[Snippet],
    length: usize,
    default_color: Color32,
    hover: HighlightingHover,
    snippet_buffer: &[u8],
    text_buffer: &str,
) -> Vec<LayoutSection> {
    let font_id = ui
        .style()
        .override_font_id
        .clone()
        .unwrap_or_else(|| TextStyle::Monospace.resolve(ui.style()));

    let mut sections = Vec::with_capacity(snippets.len() + 1);
    let mut index = 0;

    let hover_bg = ui.style().visuals.widgets.hovered.weak_bg_fill;

    for (snippet_length, node, color, token_index) in snippets {
        let byte_range = index..(index + snippet_length);

        index += snippet_length;

        if index > length
            || snippet_buffer[byte_range.clone()] != text_buffer.as_bytes()[byte_range.clone()]
        {
            index -= snippet_length;
            break;
        }

        let mut format = TextFormat::simple(font_id.clone(), *color);

        match hover {
            HighlightingHover::Position((hover_node, hover_position)) => {
                if hover_node == *node {
                    format.background = hover_bg;

                    if byte_range.contains(&hover_position) && token_index.is_some() {
                        format.underline = ui.style().visuals.widgets.hovered.bg_stroke;
                    }
                }
            }
            HighlightingHover::Node(hover_node) => {
                if hover_node == *node {
                    format.background = hover_bg;
                }
            }
            HighlightingHover::None => {}
        }

        sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: Range {
                start: text_buffer.floor_char_boundary(byte_range.start),
                end: text_buffer.floor_char_boundary(byte_range.end),
            },
            format,
        });
    }

    if index < length {
        sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: index..length,
            format: TextFormat::simple(font_id, default_color),
        });
    }

    sections
}

// TODO: Add bounding boxes per row, with each row's boxes being added in reverse order

fn absolute_snippet_positions(
    snippets: &[Snippet],
    top_left: Pos2,
    galley: &Galley,
    mut callback: impl FnMut(&Snippet, Rect, Rect),
) {
    if snippets.is_empty() {
        return;
    }

    let mut offset = 0;
    let mut snippet_index = 0;
    let mut snippet_offset = snippets[0].0;

    let first_row_rect = galley.rows.first().map(|row| row.rect());

    let mut start_pos = first_row_rect
        .map(|row_rect| row_rect.min)
        .unwrap_or(top_left);
    let mut start_height = first_row_rect
        .map(|row_rect| row_rect.max.y - row_rect.min.y)
        .unwrap_or(0.0);
    let mut min_y = top_left.y;
    let mut min_x = top_left.x;
    let mut max_x = top_left.x;

    let mut increment_callback =
        |offset: usize, char_start_pos: Pos2, char_end_pos: Pos2| -> bool {
            min_x = min_x.min(char_start_pos.x);
            let pos_x = if snippet_offset > offset {
                char_end_pos.x
            } else {
                char_start_pos.x
            };
            max_x = max_x.max(pos_x);

            while snippet_offset <= offset && snippet_index < snippets.len() {
                callback(
                    &snippets[snippet_index],
                    Rect {
                        min: Pos2 { x: min_x, y: min_y },
                        max: Pos2 {
                            x: max_x,
                            y: char_end_pos.y,
                        },
                    },
                    Rect {
                        min: start_pos,
                        max: Pos2 {
                            x: start_pos.x,
                            y: start_pos.y + start_height,
                        },
                    },
                );
                if snippet_index < snippets.len() - 1 {
                    snippet_index += 1;
                    snippet_offset += snippets[snippet_index].0;
                    start_pos = char_start_pos;
                    start_height = char_end_pos.y - char_start_pos.y;
                    min_y = char_start_pos.y;
                    min_x = char_start_pos.x;
                    max_x = if snippets[snippet_index].0 > 0 {
                        char_end_pos.x
                    } else {
                        char_start_pos.x
                    };
                } else {
                    break;
                }
            }

            snippet_index >= snippets.len()
        };

    let mut should_break = increment_callback(
        0,
        first_row_rect
            .map(|row_rect| Pos2 {
                x: top_left.x + row_rect.min.x,
                y: top_left.y + row_rect.min.y,
            })
            .unwrap_or(top_left),
        first_row_rect
            .map(|row_rect| Pos2 {
                x: top_left.x + row_rect.min.x,
                y: top_left.y + row_rect.max.y,
            })
            .unwrap_or(top_left),
    );

    for row in &galley.rows {
        let row_rect = row.rect();

        let row_start_pos = Pos2 {
            x: top_left.x + row_rect.min.x,
            y: top_left.y + row_rect.min.y,
        };

        let row_end_pos = Pos2 {
            x: top_left.x + row_rect.max.x,
            y: top_left.y + row_rect.max.y,
        };

        if should_break {
            break;
        }

        for char in row.glyphs.iter() {
            let char_len = char.chr.len_utf8();

            let char_rect = char.logical_rect();

            let char_start_pos = Pos2 {
                x: row_start_pos.x + char_rect.min.x,
                y: row_start_pos.y + char_rect.min.y,
            };

            let char_end_pos = Pos2 {
                x: row_start_pos.x + char_rect.max.x,
                y: row_start_pos.y + char_rect.max.y,
            };

            should_break = increment_callback(offset, char_start_pos, char_end_pos);

            offset += char_len;
        }

        if row.ends_with_newline {
            let (char_start_pos, char_end_pos) = (
                Pos2 {
                    x: row_end_pos.x,
                    y: row_start_pos.y,
                },
                row_end_pos,
            );

            should_break = increment_callback(offset, char_start_pos, char_end_pos);

            offset += 1;
        }
    }

    if !should_break {
        let last_row_rect = galley.rows.last().unwrap().rect();

        let row_start_pos = Pos2 {
            x: top_left.x + last_row_rect.min.x,
            y: top_left.y + last_row_rect.min.y,
        };

        let row_end_pos = Pos2 {
            x: top_left.x + last_row_rect.max.x,
            y: top_left.y + last_row_rect.max.y,
        };

        let (char_start_pos, char_end_pos) = (
            row_start_pos,
            Pos2 {
                x: row_start_pos.x,
                y: row_end_pos.y,
            },
        );

        increment_callback(offset, char_start_pos, char_end_pos);
    }
}

#[allow(clippy::collapsible_if)]
fn calculate_boundaries_and_update_scroll(
    ui: &mut Ui,
    snippets: &[Snippet],
    top_left: Pos2,
    galley: &Galley,
    changed: Option<Ulid>,
    max_height: f32,
    output: &mut Vec<(Rect, Color32)>,
) {
    if snippets.len() < 2 {
        return;
    }

    let mut last_node = None;
    let mut scroll_to = None;
    let mut scroll_to_boundary = false;

    let boundary_color = ui.style().visuals.widgets.inactive.bg_fill;
    //let boundary_color_strong = ui.style().visuals.widgets.inactive.fg_stroke.color;
    let boundary_width = ui.style().visuals.widgets.hovered.fg_stroke.width;

    absolute_snippet_positions(
        snippets,
        top_left,
        galley,
        |snippet, bounds, mut boundary| {
            boundary.min.x -= boundary_width / 2.0;
            boundary.max.x += boundary_width / 2.0;

            if last_node != Some(snippet.1) {
                output.push((
                    boundary,
                    /*if is_token {
                        boundary_color_strong
                    } else {*/
                    boundary_color,
                    //},
                ));

                if (last_node.is_some() && changed == last_node) || changed == Some(snippet.1) {
                    (scroll_to, scroll_to_boundary) = if bounds.height() > max_height {
                        (Some(boundary), true)
                    } else {
                        (Some(bounds), false)
                    };
                }

                last_node = Some(snippet.1);
            } else if last_node.is_some()
                && changed == last_node
                && let Some(scroll_to) = &mut scroll_to
            {
                if scroll_to_boundary {
                    *scroll_to = boundary;
                } else {
                    scroll_to.extend_with(bounds.min);
                    scroll_to.extend_with(bounds.max);

                    if scroll_to.height() > max_height {
                        *scroll_to = boundary;
                        scroll_to_boundary = true;
                    }
                }
            }
        },
    );

    if let Some(rect) = scroll_to {
        ui.scroll_to_rect(rect, None);
    }
}

fn cursor_is_within_galley(galley: &Galley, cursor: Pos2) -> bool {
    for row in &galley.rows {
        if row.rect().contains(cursor) {
            return true;
        }
    }

    false
}

fn render_rects(ui: &Ui, rects: &[(Rect, Color32)]) {
    if rects.is_empty() {
        return;
    }

    let mut mesh = Mesh::default();
    mesh.reserve_triangles(rects.len() * 2 * 2);
    mesh.reserve_vertices(rects.len() * 4 * 2);

    let mut vertices = 0;

    for (rect, color) in rects.iter().copied() {
        mesh.indices.extend_from_slice(&[
            vertices,
            vertices + 1,
            vertices + 2,
            vertices + 2,
            vertices + 1,
            vertices + 3,
        ]);
        mesh.vertices.extend_from_slice(&[
            Vertex {
                pos: rect.left_top(),
                uv: WHITE_UV,
                color,
            },
            Vertex {
                pos: rect.right_top(),
                uv: WHITE_UV,
                color,
            },
            Vertex {
                pos: rect.left_bottom(),
                uv: WHITE_UV,
                color,
            },
            Vertex {
                pos: rect.right_bottom(),
                uv: WHITE_UV,
                color,
            },
        ]);
        vertices += 4
    }

    ui.painter().add(mesh);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HighlightingHover {
    Position((Ulid, usize)),
    Node(Ulid),
    None,
}

fn calculate_cursor_index(
    node: Ulid,
    index: usize,
    node_snippets: &HashMap<Ulid, Vec<Range<usize>>>,
) -> Option<usize> {
    if let Some(ranges) = node_snippets.get(&node)
        && !ranges.is_empty()
    {
        Some((index - ranges.first().unwrap().start).min(ranges.last().unwrap().end))
    } else {
        None
    }
}

fn render_tooltip(
    ui: &mut Ui,
    weave: &WeaveWrapper,
    node_snippets: &HashMap<Ulid, Vec<Range<usize>>>,
    node: Ulid,
    index: usize,
) {
    if let Some(node) = weave.get_node(&node) {
        match &node.contents.content {
            InnerNodeContent::Snippet(_) => {
                render_node_metadata_tooltip(ui, node);
            }
            InnerNodeContent::Tokens(tokens) => {
                render_node_metadata_tooltip(ui, node);

                let mut token_offset: Option<usize> = if let Some(ranges) =
                    node_snippets.get(&Ulid(node.id))
                    && !ranges.is_empty()
                    && ranges.len() == tokens.len()
                {
                    ranges
                        .iter()
                        .enumerate()
                        .find(|(_, range)| range.contains(&index))
                        .map(|(i, _)| i)
                } else {
                    None
                };

                if tokens.len() == 1 && token_offset.is_none() {
                    token_offset = Some(0);
                }

                if let Some((token, token_metadata)) =
                    token_offset.and_then(|index| tokens.get(index))
                {
                    ui.separator();
                    render_token_tooltip(ui, token, token_metadata);
                }
            }
        }
    }
}
