use std::{cell::RefCell, collections::HashMap, ops::Range, rc::Rc};

use eframe::{
    egui::{
        Color32, Frame, Galley, Id, Mesh, Pos2, Rect, ScrollArea, Sense, TextBuffer, TextEdit,
        TextFormat, TextStyle, Ui,
        text::{CCursor, CCursorRange, LayoutJob, LayoutSection, TextWrapping},
    },
    epaint::{MarginF32, Vertex, WHITE_UV},
};
use egui_notify::Toasts;
use flagset::FlagSet;
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::indexmap::{IndexMap, IndexSet},
    v0::{InnerNodeContent, NodeContent, TapestryNode, deserialize_counterfactual_logprobs},
};

use crate::{
    editor::shared::{
        NodeIndex, SharedState, get_node_color, get_token_color, render_node_metadata_tooltip,
        render_token_counterfactual_tooltip, render_token_tooltip, weave::WeaveWrapper,
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
    last_text_edit_cursor: Option<CCursor>,
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
            last_text_edit_cursor: None,
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
        self.last_text_edit_cursor = None;
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
        /*let contains_cursor = ui
        .clip_rect()
        .contains(ui.ctx().pointer_hover_pos().unwrap_or_default());*/

        if self.text.is_empty() {
            self.update_contents(weave, settings, ui.visuals().widgets.inactive.text_color());
        }

        let snippets = self.snippets.clone();
        let hover = state.get_hovered_node();
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
                                &mut self.rects,
                            );

                            self.last_text_edit_rect = textedit.response.rect;
                            self.should_update_rects = false;
                            if !self.rects.is_empty() {
                                ui.ctx().request_repaint();
                            }
                        }

                        let mut index: usize = 0;
                        let mut last_node = Ulid(0);
                        let mut byte_index: usize = 0;
                        let mut token_index: usize = 0;

                        absolute_snippet_row_positions(
                            &self.snippets.borrow(),
                            top_left,
                            &textedit.galley,
                            |snippet, bounds, _start, increment| {
                                let response = ui.interact(
                                    bounds,
                                    Id::new((snippet.1, index)),
                                    Sense::hover(),
                                );

                                if last_node != snippet.1 {
                                    last_node = snippet.1;
                                    token_index = 0;
                                }

                                if response.contains_pointer() {
                                    if let Some(within_index) = snippet.3 {
                                        state.set_hovered_node(NodeIndex::WithinNode(
                                            snippet.1,
                                            within_index,
                                        ));
                                    } else {
                                        state.set_hovered_node(NodeIndex::Node(snippet.1));
                                    }
                                }

                                response.on_hover_ui(|ui| {
                                    if let Some(within_index) = snippet.3 {
                                        state.set_hovered_node(NodeIndex::WithinNode(
                                            snippet.1,
                                            within_index,
                                        ));
                                    } else {
                                        state.set_hovered_node(NodeIndex::Node(snippet.1));
                                    }

                                    render_tooltip(ui, weave, snippet.1, token_index);
                                });

                                /*ui.painter().rect_filled(
                                    bounds,
                                    0.0,
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 50),
                                );*/

                                /*ui.painter().rect_stroke(
                                    bounds,
                                    0.0,
                                    (1.0, eframe::egui::Color32::WHITE),
                                    eframe::egui::StrokeKind::Inside,
                                );*/

                                if increment {
                                    token_index += 1;
                                    byte_index += snippet.0;
                                }

                                index += 1;

                                /*start.max.x += 1.0;

                                ui.painter().rect_stroke(
                                    start,
                                    0.0,
                                    (1.0, eframe::egui::Color32::GREEN),
                                    eframe::egui::StrokeKind::Inside,
                                );*/
                            },
                        );

                        if textedit.response.changed() {
                            self.update_weave(state, weave);
                            self.last_text_edit_cursor = None;
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

                        ui.style_mut().override_font_id = None;

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
    hover: NodeIndex,
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
    let mut last_node = Ulid(0);
    let mut node_index = 0;

    let hover_bg = ui.style().visuals.widgets.hovered.weak_bg_fill;

    for (snippet_length, node, color, token_index) in snippets {
        let byte_range = index..(index + snippet_length);

        if *node != last_node {
            last_node = *node;
            node_index = 0;
        }

        let node_range = node_index..(node_index + snippet_length);

        index += snippet_length;
        node_index += snippet_length;

        if index > length
            || snippet_buffer[byte_range.clone()] != text_buffer.as_bytes()[byte_range.clone()]
        {
            index -= snippet_length;
            break;
        }

        let mut format = TextFormat::simple(font_id.clone(), *color);

        match hover {
            NodeIndex::WithinNode(hover_node, hover_position) => {
                if hover_node == *node {
                    format.background = hover_bg;

                    if node_range.contains(&hover_position) && token_index.is_some() {
                        format.underline = ui.style().visuals.widgets.hovered.bg_stroke;
                    }
                }
            }
            NodeIndex::Node(hover_node) => {
                if hover_node == *node {
                    format.background = hover_bg;
                }
            }
            NodeIndex::None => {}
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

fn absolute_snippet_positions(
    snippets: &[Snippet],
    top_left: Pos2,
    galley: &Galley,
    mut callback: impl FnMut(&Snippet, Rect, Rect),
) {
    if snippets.is_empty() {
        return;
    }

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

    absolute_galley_character_positions(
        top_left,
        galley,
        |offset, _row, char_start_pos, char_end_pos| {
            min_x = min_x.min(char_start_pos.x);
            let pos_x = if snippet_offset > offset {
                char_end_pos.x
            } else {
                char_start_pos.x
            };
            max_x = max_x.max(pos_x);

            while snippet_offset <= offset {
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

                snippet_index += 1;

                if snippet_index < snippets.len() {
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
                    return true;
                }
            }

            false
        },
    );
}

fn absolute_snippet_row_positions(
    snippets: &[Snippet],
    top_left: Pos2,
    galley: &Galley,
    mut callback: impl FnMut(&Snippet, Rect, Rect, bool),
) {
    if snippets.is_empty() {
        return;
    }

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
    let mut max_y = top_left.y;
    let mut min_x = top_left.x;
    let mut max_x = top_left.x;
    let mut last_row = 0;

    absolute_galley_character_positions(
        top_left,
        galley,
        |offset, row, char_start_pos, char_end_pos| {
            if row > last_row {
                callback(
                    &snippets[snippet_index],
                    Rect {
                        min: Pos2 { x: min_x, y: min_y },
                        max: Pos2 { x: max_x, y: max_y },
                    },
                    Rect {
                        min: start_pos,
                        max: Pos2 {
                            x: start_pos.x,
                            y: start_pos.y + start_height,
                        },
                    },
                    snippet_offset <= offset,
                );
            }

            if row > last_row {
                start_pos = char_start_pos;
                start_height = char_end_pos.y - char_start_pos.y;
                min_y = char_start_pos.y;
                max_y = char_end_pos.y;
                min_x = char_start_pos.x;
                max_x = if snippets[snippet_index].0 > 0 {
                    char_end_pos.x
                } else {
                    char_start_pos.x
                };
            } else {
                min_x = min_x.min(char_start_pos.x);
                let pos_x = if snippet_offset > offset {
                    char_end_pos.x
                } else {
                    char_start_pos.x
                };
                max_x = max_x.max(pos_x);
                max_y = max_y.max(char_end_pos.y);
            }

            while snippet_offset <= offset {
                if row <= last_row {
                    callback(
                        &snippets[snippet_index],
                        Rect {
                            min: Pos2 { x: min_x, y: min_y },
                            max: Pos2 { x: max_x, y: max_y },
                        },
                        Rect {
                            min: start_pos,
                            max: Pos2 {
                                x: start_pos.x,
                                y: start_pos.y + start_height,
                            },
                        },
                        true,
                    );
                    last_row = row;
                }

                snippet_index += 1;

                if snippet_index < snippets.len() {
                    snippet_offset += snippets[snippet_index].0;
                    start_pos = char_start_pos;
                    start_height = char_end_pos.y - char_start_pos.y;
                    min_y = char_start_pos.y;
                    max_y = char_end_pos.y;
                    min_x = char_start_pos.x;
                    max_x = if snippets[snippet_index].0 > 0 {
                        char_end_pos.x
                    } else {
                        char_start_pos.x
                    };
                } else {
                    return true;
                }
            }

            if row > last_row {
                last_row = row;
            }

            false
        },
    );
}

fn absolute_galley_character_positions(
    top_left: Pos2,
    galley: &Galley,
    mut callback: impl FnMut(usize, usize, Pos2, Pos2) -> bool,
) {
    let mut offset = 0;

    let first_row_rect = galley.rows.first().map(|row| row.rect());

    if callback(
        0,
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
    ) {
        return;
    }

    'outer: for (i, row) in galley.rows.iter().enumerate() {
        let row_rect = row.rect();

        let row_start_pos = Pos2 {
            x: top_left.x + row_rect.min.x,
            y: top_left.y + row_rect.min.y,
        };

        let row_end_pos = Pos2 {
            x: top_left.x + row_rect.max.x,
            y: top_left.y + row_rect.max.y,
        };

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

            if callback(offset, i, char_start_pos, char_end_pos) {
                break 'outer;
            }

            offset += char_len;
        }

        if row.ends_with_newline || i == galley.rows.len() - 1 {
            let (char_start_pos, char_end_pos) = (
                Pos2 {
                    x: row_end_pos.x,
                    y: row_start_pos.y,
                },
                row_end_pos,
            );

            if callback(offset, i, char_start_pos, char_end_pos) {
                break 'outer;
            }

            offset += 1;
        }
    }
}

fn calculate_boundaries_and_update_scroll(
    ui: &mut Ui,
    snippets: &[Snippet],
    top_left: Pos2,
    galley: &Galley,
    changed: Option<Ulid>,
    output: &mut Vec<(Rect, Color32)>,
) {
    if snippets.len() < 2 {
        return;
    }

    let mut last_node = None;
    let mut scroll_to = None;

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
                    scroll_to = Some(bounds);
                }

                last_node = Some(snippet.1);
            } else if last_node.is_some()
                && changed == last_node
                && let Some(scroll_to) = &mut scroll_to
            {
                scroll_to.extend_with(bounds.min);
                scroll_to.extend_with(bounds.max);
            }
        },
    );

    if let Some(rect) = scroll_to {
        ui.scroll_to_rect(rect, None);
    }
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

fn render_tooltip(ui: &mut Ui, weave: &mut WeaveWrapper, node: Ulid, index: usize) {
    if let Some(node) = weave.get_node(&node) {
        match &node.contents.content {
            InnerNodeContent::Snippet(_) => {
                render_node_metadata_tooltip(ui, node);
            }
            InnerNodeContent::Tokens(tokens) => {
                if let Some((token, token_metadata)) = tokens.get(index) {
                    let (has_counterfactual, counterfactual_choice) =
                        render_token_counterfactual_tooltip(ui, token_metadata);

                    if has_counterfactual {
                        ui.separator();
                    }

                    render_token_tooltip(ui, token, token_metadata);
                    ui.separator();

                    render_node_metadata_tooltip(ui, node);

                    if let Some(counterfactual_index) = counterfactual_choice
                        && let Some(value) = token_metadata.get("counterfactual")
                        && let Some(counterfactual) = deserialize_counterfactual_logprobs(value)
                        && let Some(counterfactual_token) =
                            counterfactual.get(counterfactual_index).cloned()
                    {
                        let metadata = node.contents.metadata.clone();
                        let model = node.contents.model.clone();

                        let node = Ulid(node.id);
                        if let Some(split) = weave.split_out_token(&node, index) {
                            let active = weave
                                .get_active_thread_u128()
                                .collect::<Vec<_>>()
                                .contains(&node.0);

                            weave.add_node(TapestryNode {
                                id: Ulid::new().0,
                                from: Some(split.0.0),
                                to: IndexSet::default(),
                                active,
                                bookmarked: false,
                                contents: NodeContent {
                                    content: InnerNodeContent::Tokens(vec![counterfactual_token]),
                                    metadata,
                                    model,
                                },
                            });
                        }
                    }
                } else {
                    render_node_metadata_tooltip(ui, node);
                }
            }
        }
    }
}
