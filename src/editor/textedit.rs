use std::{
    cell::RefCell,
    collections::HashMap,
    ops::Range,
    rc::Rc,
    time::{Duration, Instant, SystemTime},
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
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{Weave, indexmap::IndexMap},
    v0::{InnerNodeContent, TapestryWeave},
};

use crate::{
    editor::shared::{
        NodeIndex, SharedState, get_node_color, get_token_color, render_node_metadata_tooltip,
        render_token_metadata_tooltip,
    },
    listing_margin,
    settings::{Settings, shortcuts::Shortcuts},
};

#[derive(Debug)]
pub struct TextEditorView {
    text: String,
    bytes: Vec<u8>,
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
}

// TODO: Implement a context menu on the TextEdit
// Currently stuck on lacking APIs in egui; see https://github.com/emilk/egui/issues/4393

// TODO: Implement Ctrl+F in TextEdit

type Snippet = (usize, Ulid, Color32, Option<usize>);

const SUBSTITUTION_CHAR: char = '␚'; //Must be 1 UTF-8 byte in length
const SUBSTITUTION_BYTE: u8 = "␚".as_bytes()[0];

impl Default for TextEditorView {
    fn default() -> Self {
        Self {
            text: String::with_capacity(262144),
            bytes: Vec::with_capacity(262144),
            buffer: Vec::with_capacity(262144),
            snippets: Rc::new(RefCell::new(Vec::with_capacity(65536))),
            node_snippets: HashMap::with_capacity(65536),
            rects: Vec::with_capacity(65536),
            last_seen_cursor_node: NodeIndex::None,
            last_seen_hovered_node: NodeIndex::None,
            last_text_edit_cursor: None,
            last_text_edit_hover: None,
            last_text_edit_highlighting_hover: HighlightingHover::None,
            last_text_edit_highlighting_hover_update: Instant::now(),
        }
    }
}

impl TextEditorView {
    /*pub fn reset(&mut self) {
        self.text.clear();
        self.bytes.clear();
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
    }*/
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        self.update(weave, settings, ui.visuals().widgets.inactive.text_color());

        let snippets = self.snippets.clone();
        let hover = self.last_text_edit_highlighting_hover;

        let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
            let layout_job = LayoutJob {
                sections: calculate_highlighting(
                    ui,
                    &snippets.borrow(),
                    buf.as_str().len(),
                    ui.visuals().widgets.inactive.text_color(),
                    hover,
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

                        render_rects(ui, &mut self.rects);

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
                            if !textedit.response.changed() {
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

                        calculate_boundaries(
                            ui,
                            &self.snippets.borrow(),
                            top_left,
                            &textedit.galley,
                            match self.last_text_edit_highlighting_hover {
                                HighlightingHover::Position((node, _)) => Some(node),
                                HighlightingHover::Node(node) => Some(node),
                                HighlightingHover::None => None,
                            },
                            &mut self.rects,
                        );

                        if textedit.response.changed() {
                            self.update_weave(weave);
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

                        let hover_position = textedit.response.hover_pos().map(|p| p - top_left);

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
                    });
            });
    }
    fn update(&mut self, weave: &mut TapestryWeave, settings: &Settings, default_color: Color32) {
        let mut snippets = self.snippets.borrow_mut();
        self.text.clear();
        self.bytes.clear();
        snippets.clear();
        self.node_snippets.clear();

        let active: Vec<u128> = weave.weave.get_active_thread().iter().copied().collect();

        let mut offset = 0;

        for node in active
            .into_iter()
            .rev()
            .filter_map(|id| weave.weave.get_node(&id))
        {
            let color = get_node_color(node, settings).unwrap_or(default_color);

            match &node.contents.content {
                InnerNodeContent::Snippet(snippet) => {
                    self.bytes.extend_from_slice(snippet);
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
                        let color = get_token_color(Some(color), token_metadata, settings)
                            .unwrap_or(default_color);

                        self.bytes.extend_from_slice(token);
                        snippets.push((token.len(), Ulid(node.id), color, Some(token_index)));
                        token_indices.push(offset..offset + token.len());
                        token_index += token.len();
                        offset += token.len();
                    }

                    self.node_snippets.insert(Ulid(node.id), token_indices);
                }
            }
        }

        for chunk in self.bytes.utf8_chunks() {
            self.text.push_str(chunk.valid());

            for _ in chunk.invalid() {
                self.text.push(SUBSTITUTION_CHAR);
            }
        }
    }
    fn calculate_cursor(
        &mut self,
        weave: &mut TapestryWeave,
        char_position: Option<usize>,
    ) -> Option<(Ulid, usize)> {
        let mut cursor_node = None;

        if let Some(char_index) = char_position {
            let index = self.text.byte_index_from_char_index(char_index);

            let mut offset = 0;

            for (length, node, _, _) in self.snippets.borrow().iter() {
                offset += length;
                if offset >= index {
                    cursor_node = Some((*node, index));
                    if offset > index {
                        break;
                    }
                }
            }
        } else if let Some(active) = weave.get_active_thread().next().map(|node| Ulid(node.id)) {
            cursor_node = Some((active, self.text.len()));
        } else {
            cursor_node = None;
        }

        cursor_node
    }
    fn update_weave(&mut self, weave: &mut TapestryWeave) {
        self.buffer.clear();
        self.buffer.extend_from_slice(self.text.as_bytes());

        for (index, byte) in self.buffer.iter_mut().take(self.bytes.len()).enumerate() {
            if *byte == SUBSTITUTION_BYTE {
                *byte = self.bytes[index];
            }
        }

        weave.set_active_content(&self.buffer, IndexMap::default(), |timestamp| {
            if let Some(timestamp) = timestamp {
                Ulid::from_datetime(SystemTime::UNIX_EPOCH + Duration::from_millis(timestamp))
            } else {
                Ulid::new()
            }
        });
    }
}

fn calculate_highlighting(
    ui: &Ui,
    snippets: &[Snippet],
    length: usize,
    default_color: Color32,
    hover: HighlightingHover,
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
        index += snippet_length;

        if index > length {
            index -= snippet_length;
            break;
        }

        let mut format = TextFormat::simple(font_id.clone(), *color);

        let byte_range = (index - snippet_length)..index;

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
            byte_range,
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

fn calculate_boundaries(
    ui: &Ui,
    snippets: &[Snippet],
    top_left: Pos2,
    galley: &Galley,
    _hover: Option<Ulid>,
    output: &mut Vec<(Rect, Color32)>,
) {
    if snippets.len() < 2 {
        return;
    }

    let mut offset = 0;
    let mut snippet_index = 0;
    let mut snippet_offset = 0;

    let boundary_color = ui.style().visuals.widgets.inactive.bg_fill;
    let boundary_color_strong = ui.style().visuals.widgets.inactive.fg_stroke.color;
    let boundary_width = ui.style().visuals.widgets.hovered.fg_stroke.width;

    let mut draw_row_index = |pos: Pos2, size: Vec2, len: usize, index: usize, is_token: bool| {
        let x = pos.x + ((size.x / len as f32) * index as f32);

        let rect = Rect {
            min: Pos2 {
                x: (x - (boundary_width / 2.0)),
                y: pos.y,
            },
            max: Pos2 {
                x: (x + (boundary_width / 2.0)),
                y: pos.y + size.y,
            },
        };

        output.push((
            rect,
            if is_token {
                boundary_color_strong
            } else {
                boundary_color
            },
        ))
    };

    let mut last_node = None;

    for row in &galley.rows {
        if snippet_index > snippets.len() {
            break;
        }

        let row_chars = row.char_count_excluding_newline();

        let row_position = Pos2 {
            x: row.pos.x + top_left.x,
            y: row.pos.y + top_left.y,
        };

        for (i, char) in row.glyphs.iter().enumerate() {
            let char_len = char.chr.len_utf8();

            if snippet_index >= snippets.len() {
                break;
            } else if offset >= snippet_offset {
                if last_node != Some(snippets[snippet_index].1) {
                    if offset > 0 {
                        draw_row_index(row_position, row.size, row_chars, i, false);
                    }
                    last_node = Some(snippets[snippet_index].1);
                } /*else if hover == Some(snippets[snippet_index].1) {
                draw_row_index(row_position, row.size, row_chars, i, true);
                }*/

                snippet_offset += snippets[snippet_index].0;
                snippet_index += 1;
            }

            offset += char_len;
        }

        if row.ends_with_newline {
            if snippet_index >= snippets.len() {
                break;
            } else if offset >= snippet_offset {
                if last_node != Some(snippets[snippet_index].1) {
                    if offset > 0 {
                        draw_row_index(row_position, row.size, row_chars, row_chars, false);
                    }
                    last_node = Some(snippets[snippet_index].1);
                } /*else if hover == Some(snippets[snippet_index].1) {
                draw_row_index(row_position, row.size, row_chars, row_chars, true);
                }*/

                snippet_offset += snippets[snippet_index].0;
                snippet_index += 1;
            }

            offset += 1;
        }
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

fn render_rects(ui: &Ui, rects: &mut Vec<(Rect, Color32)>) {
    if rects.is_empty() {
        return;
    }

    let mut mesh = Mesh::default();
    mesh.reserve_triangles(rects.len() * 2 * 2);
    mesh.reserve_vertices(rects.len() * 4 * 2);

    let mut vertices = 0;

    for (rect, color) in rects.drain(..) {
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
    weave: &TapestryWeave,
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

                if let Some((_, token_metadata)) = token_offset.and_then(|index| tokens.get(index))
                {
                    ui.separator();
                    render_token_metadata_tooltip(ui, token_metadata);
                }
            }
        }
    }
}
