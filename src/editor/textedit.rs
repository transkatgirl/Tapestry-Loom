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
    snippets: Vec<Snippet>,
    highlighting: Rc<RefCell<Vec<LayoutSection>>>,
    node_snippets: HashMap<Ulid, Vec<Range<usize>>>,
    rects: Vec<(Rect, Color32)>,
    last_seen_cursor_node: NodeIndex,
    last_seen_hovered_node: NodeIndex,
    last_text_edit_cursor: Option<CCursor>,
    last_text_edit_hover: Option<Vec2>,
    last_text_edit_highlighting_hover: HighlightingHover,
    last_text_edit_highlighting_hover_update: Instant,
    last_text_edit_rect: Rect,
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
            text: String::with_capacity(262144),
            bytes: Rc::new(RefCell::new(Vec::with_capacity(262144))),
            buffer: Vec::with_capacity(262144),
            snippets: Vec::with_capacity(65536),
            highlighting: Rc::new(RefCell::new(Vec::with_capacity(65536))),
            node_snippets: HashMap::with_capacity(65536),
            rects: Vec::with_capacity(65536),
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
    pub fn update(
        &mut self,
        ui: &Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        if state.has_weave_changed || self.text.is_empty() {
            self.update_contents(weave, settings, ui.visuals().widgets.inactive.text_color());
        }
        if state.has_weave_changed {
            self.rects.clear();
        }
        if state.has_weave_changed
            || self.highlighting.borrow().is_empty()
            || state.has_hover_node_changed
        {
            self.calculate_highlighting(ui);
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
        let highlighting = self.highlighting.clone();

        let bytes = self.bytes.clone();

        let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
            let layout_job = LayoutJob {
                sections: if buf.as_str().as_bytes() == *bytes.borrow() {
                    highlighting.borrow().clone()
                } else {
                    vec![LayoutSection {
                        leading_space: 0.0,
                        byte_range: 0..buf.as_str().len(),
                        format: TextFormat::simple(
                            ui.style()
                                .override_font_id
                                .clone()
                                .unwrap_or_else(|| TextStyle::Monospace.resolve(ui.style())),
                            ui.visuals().widgets.inactive.text_color(),
                        ),
                    }]
                },
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

                        if self.rects.is_empty()
                            || textedit.response.changed()
                            || textedit.response.rect != self.last_text_edit_rect
                            || state.get_changed_node().is_some()
                        {
                            self.rects.clear();
                            calculate_boundaries_and_update_scroll(
                                ui,
                                &self.snippets,
                                top_left,
                                &textedit.galley,
                                if settings.interface.auto_scroll {
                                    state.get_changed_node()
                                } else {
                                    None
                                },
                                &mut self.rects,
                            );
                            self.last_text_edit_rect = textedit.response.rect;
                        }

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
    fn update_contents(
        &mut self,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        default_color: Color32,
    ) {
        let mut bytes = self.bytes.borrow_mut();
        self.text.clear();
        bytes.clear();
        self.snippets.clear();
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
                    self.snippets
                        .push((snippet.len(), Ulid(node.id), color, None));
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

                        bytes.extend_from_slice(token);
                        self.snippets
                            .push((token.len(), Ulid(node.id), color, Some(token_index)));
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

        if let Some(char_index) = char_position {
            let index = self.text.byte_index_from_char_index(char_index);

            let mut offset = 0;

            for (length, node, _, _) in self.snippets.iter() {
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
    fn update_weave(&mut self, weave: &mut WeaveWrapper) {
        self.buffer.clear();
        self.buffer.extend_from_slice(self.text.as_bytes());

        let bytes = self.bytes.borrow();

        for (index, byte) in self.buffer.iter_mut().take(bytes.len()).enumerate() {
            if *byte == SUBSTITUTION_BYTE {
                *byte = bytes[index];
            }
        }

        weave.set_active_content(&self.buffer, IndexMap::default());
    }
    fn calculate_highlighting(&mut self, ui: &Ui) {
        let font_id = ui
            .style()
            .override_font_id
            .clone()
            .unwrap_or_else(|| TextStyle::Monospace.resolve(ui.style()));

        let mut sections = self.highlighting.borrow_mut();
        sections.clear();

        let mut index = 0;
        let hover_bg = ui.style().visuals.widgets.hovered.weak_bg_fill;

        for (snippet_length, node, color, token_index) in self.snippets.iter().copied() {
            let byte_range = index..(index + snippet_length);

            index += snippet_length;

            let mut format = TextFormat::simple(font_id.clone(), color);

            match self.last_text_edit_highlighting_hover {
                HighlightingHover::Position((hover_node, hover_position)) => {
                    if hover_node == node {
                        format.background = hover_bg;

                        if byte_range.contains(&hover_position) && token_index.is_some() {
                            format.underline = ui.style().visuals.widgets.hovered.bg_stroke;
                        }
                    }
                }
                HighlightingHover::Node(hover_node) => {
                    if hover_node == node {
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

    let mut offset = 0;
    let mut snippet_index = 0;
    let mut snippet_offset = 0;

    let boundary_color = ui.style().visuals.widgets.inactive.bg_fill;
    let boundary_color_strong = ui.style().visuals.widgets.inactive.fg_stroke.color;
    let boundary_width = ui.style().visuals.widgets.hovered.fg_stroke.width;

    let mut draw_row_boundary = |row_pos: Pos2, row_size: Vec2, x: f32, is_token: bool| {
        let x = row_pos.x + x;

        let rect = Rect {
            min: Pos2 {
                x: (x - (boundary_width / 2.0)),
                y: row_pos.y,
            },
            max: Pos2 {
                x: (x + (boundary_width / 2.0)),
                y: row_pos.y + row_size.y,
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

    let scroll_boundary_into_view = |row_pos: Pos2, row_size: Vec2, x: f32| {
        let x = row_pos.x + x;

        let rect = Rect {
            min: Pos2 { x, y: row_pos.y },
            max: Pos2 {
                x,
                y: row_pos.y + row_size.y,
            },
        };

        ui.scroll_to_rect(rect, None);
    };

    let mut last_node = None;

    for row in &galley.rows {
        if snippet_index > snippets.len() {
            break;
        }

        let row_position = Pos2 {
            x: row.pos.x + top_left.x,
            y: row.pos.y + top_left.y,
        };

        for char in row.glyphs.iter() {
            let char_len = char.chr.len_utf8();

            if snippet_index >= snippets.len() {
                break;
            } else {
                while offset >= snippet_offset {
                    if last_node != Some(snippets[snippet_index].1) {
                        if offset > 0 {
                            draw_row_boundary(row_position, row.size, char.pos.x, false);
                        }
                        last_node = Some(snippets[snippet_index].1);
                    } /*else if hover == Some(snippets[snippet_index].1) {
                    draw_row_boundary(row_position, row.size, char.pos.x, true);
                    }*/

                    if changed == Some(snippets[snippet_index].1) || changed == last_node {
                        scroll_boundary_into_view(row_position, row.size, char.pos.x);
                    }

                    snippet_offset += snippets[snippet_index].0;
                    snippet_index += 1;
                }
            }

            offset += char_len;
        }

        if row.ends_with_newline {
            if snippet_index >= snippets.len() {
                break;
            } else {
                while offset >= snippet_offset {
                    if last_node != Some(snippets[snippet_index].1) {
                        if offset > 0 {
                            draw_row_boundary(row_position, row.size, row.size.x, false);
                        }
                        last_node = Some(snippets[snippet_index].1);
                    } /*else if hover == Some(snippets[snippet_index].1) {
                    draw_row_boundary(row_position, row.size, row.size.x, true);
                    }*/

                    if changed == Some(snippets[snippet_index].1) || changed == last_node {
                        scroll_boundary_into_view(row_position, row.size, row.size.x);
                    }

                    snippet_offset += snippets[snippet_index].0;
                    snippet_index += 1;
                }
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
