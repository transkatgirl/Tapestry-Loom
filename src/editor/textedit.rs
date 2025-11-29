use std::{
    cell::RefCell,
    ops::Range,
    rc::Rc,
    time::{Duration, SystemTime},
};

use eframe::{
    egui::{
        Color32, FontId, Galley, Mesh, Pos2, Rect, ScrollArea, TextBuffer, TextEdit, TextFormat,
        TextStyle, Ui, Vec2,
        text::{CCursor, LayoutJob, LayoutSection, TextWrapping},
    },
    epaint::{Vertex, WHITE_UV},
};
use egui_notify::Toasts;
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{Weave, indexmap::IndexMap},
    v0::{InnerNodeContent, TapestryWeave},
};

use crate::{
    editor::shared::{SharedState, get_node_color, get_token_color},
    settings::Settings,
};

#[derive(Debug)]
pub struct TextEditorView {
    text: String,
    bytes: Vec<u8>,
    buffer: Vec<u8>,
    snippets: Rc<RefCell<Vec<Snippet>>>,
    last_seen_cursor_node: Option<Ulid>,
    last_seen_hovered_node: Option<Ulid>,
    last_text_edit_cursor: Option<CCursor>,
    last_text_edit_hover: Option<Vec2>,
    last_text_edit_highlighting_hover: HighlightingHover,
}

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
            last_seen_cursor_node: None,
            last_seen_hovered_node: None,
            last_text_edit_cursor: None,
            last_text_edit_hover: None,
            last_text_edit_highlighting_hover: HighlightingHover::None,
        }
    }
}

impl TextEditorView {
    pub fn reset(&mut self) {
        self.text.clear();
        self.bytes.clear();
        self.buffer.clear();
        self.last_seen_cursor_node = None;
        self.last_seen_hovered_node = None;
        self.last_text_edit_cursor = None;
        self.last_text_edit_hover = None;
        self.last_text_edit_highlighting_hover = HighlightingHover::None;
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
        self.update(weave, settings, ui.visuals().widgets.inactive.text_color());

        if self.last_seen_cursor_node != state.get_cursor_node() {
            // TODO
            self.last_seen_cursor_node = state.get_cursor_node();
        }

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
                let textedit = TextEdit::multiline(&mut self.text)
                    .frame(false)
                    .text_color(ui.visuals().widgets.inactive.text_color())
                    .min_size(ui.available_size())
                    .desired_width(ui.available_size().x)
                    .code_editor()
                    .layouter(&mut layouter)
                    .show(ui);

                let top_left = textedit.text_clip_rect.left_top();

                render_boundaries(ui, &self.snippets.borrow(), top_left, &textedit.galley);

                if textedit.response.changed() {
                    self.update_weave(weave);
                    self.last_text_edit_cursor = None;
                } else {
                    let position = textedit.cursor_range.map(|c| c.sorted_cursors()[0]);
                    if position != self.last_text_edit_cursor {
                        if position.is_some() {
                            if let Some((node, _)) =
                                self.calculate_cursor(weave, position.map(|p| p.index))
                            {
                                state.set_cursor_node(Some(node));
                            } else {
                                state.set_cursor_node(None);
                            }
                        }
                        self.last_text_edit_cursor = position;
                    }
                }

                let hover_position = textedit.response.hover_pos().map(|p| p - top_left);

                if hover_position != self.last_text_edit_hover {
                    if let Some(hover_position) =
                        hover_position.map(|p| textedit.galley.cursor_from_pos(p).index)
                    {
                        self.last_text_edit_highlighting_hover = self
                            .calculate_cursor(weave, Some(hover_position))
                            .map(|(id, i)| HighlightingHover::Position((id, i)))
                            .unwrap_or(HighlightingHover::None);
                    } else {
                        self.last_text_edit_highlighting_hover = HighlightingHover::None;
                    }

                    self.last_text_edit_hover = hover_position;
                }

                if let HighlightingHover::Position((hover_node, hover_index)) =
                    self.last_text_edit_highlighting_hover
                {
                    // TODO: Display node metadata on hover
                    state.set_hovered_node(Some(hover_node));
                    self.last_seen_hovered_node = Some(hover_node);
                } else if self.last_seen_hovered_node != state.get_hovered_node() {
                    self.last_text_edit_highlighting_hover = state
                        .get_hovered_node()
                        .map(HighlightingHover::Node)
                        .unwrap_or(HighlightingHover::None);
                    self.last_seen_hovered_node = state.get_hovered_node();
                }
            });
    }
    fn update(&mut self, weave: &mut TapestryWeave, settings: &Settings, default_color: Color32) {
        let mut snippets = self.snippets.borrow_mut();
        self.text.clear();
        self.bytes.clear();
        snippets.clear();

        let active: Vec<u128> = weave.weave.get_active_thread().iter().copied().collect();

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
                }
                InnerNodeContent::Tokens(tokens) => {
                    let mut token_index = 0;

                    for (token, token_metadata) in tokens {
                        let color = get_token_color(Some(color), token_metadata, settings)
                            .unwrap_or(default_color);

                        self.bytes.extend_from_slice(token);
                        snippets.push((token.len(), Ulid(node.id), color, Some(token_index)));
                        token_index += token.len();
                    }
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
                // TODO: Improve handling of multi-token nodes

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

// TODO: Render token boundaries underneath text / cursors / highlighting
fn render_boundaries(ui: &Ui, snippets: &[Snippet], top_left: Pos2, galley: &Galley) {
    if snippets.len() < 2 {
        return;
    }

    // TODO: Only show token boundaries on hover

    let mut offset = 0;
    let mut snippet_index = 0;
    let mut snippet_offset = 0;

    let boundary_color = ui.style().visuals.widgets.inactive.weak_bg_fill;
    let boundary_width = ui.style().visuals.widgets.hovered.fg_stroke.width;

    let mut mesh = Mesh::default();
    mesh.reserve_triangles(snippets.len() * 2 * 2);
    mesh.reserve_vertices(snippets.len() * 4 * 2);

    let mut vertices = 0;

    let mut draw_rect = |rect: Rect, color| {
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
        vertices += 4;
    };

    let mut draw_row_index = |pos: Pos2, size: Vec2, len: usize, index: usize| {
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

        draw_rect(rect, boundary_color)
    };

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
                if offset > 0 {
                    draw_row_index(row_position, row.size, row_chars, i);
                }
                snippet_offset += snippets[snippet_index].0;
                snippet_index += 1;
            }

            offset += char_len;
        }

        if row.ends_with_newline {
            if snippet_index >= snippets.len() {
                break;
            } else if offset >= snippet_offset {
                if offset > 0 {
                    draw_row_index(row_position, row.size, row_chars, row_chars);
                }
                snippet_offset += snippets[snippet_index].0;
                snippet_index += 1;
            }

            offset += 1;
        }
    }

    ui.painter().add(mesh);
}

#[derive(Debug, Clone, Copy)]
enum HighlightingHover {
    Position((Ulid, usize)),
    Node(Ulid),
    None,
}
