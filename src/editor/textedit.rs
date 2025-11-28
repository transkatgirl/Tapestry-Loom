use std::{
    cell::RefCell,
    ops::Range,
    rc::Rc,
    time::{Duration, SystemTime},
};

use eframe::egui::{
    Color32, FontId, Galley, Pos2, ScrollArea, TextBuffer, TextEdit, TextFormat, TextStyle, Ui,
    text::{CCursor, LayoutJob, LayoutSection, TextWrapping},
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
    snippets: Vec<Snippet>,
    highlighting: Rc<RefCell<Vec<LayoutSection>>>,
    last_seen_cursor_node: Option<Ulid>,
    last_seen_hovered_node: Option<Ulid>,
    last_text_edit_cursor: Option<CCursor>,
    last_text_edit_hover: Option<Pos2>,
    last_text_edit_highlighting_hover: HighlightingHover,
}

type Snippet = (usize, Ulid, usize);

const SUBSTITUTION_CHAR: char = '␚'; //Must be 1 UTF-8 byte in length
const SUBSTITUTION_BYTE: u8 = "␚".as_bytes()[0];

impl Default for TextEditorView {
    fn default() -> Self {
        Self {
            text: String::with_capacity(262144),
            bytes: Vec::with_capacity(262144),
            buffer: Vec::with_capacity(262144),
            snippets: Vec::with_capacity(65536),
            highlighting: Rc::new(RefCell::new(Vec::with_capacity(65536))),
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
        self.snippets.clear();
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
        self.update(
            weave,
            settings,
            ui.visuals().widgets.inactive.text_color(),
            ui.style().visuals.widgets.hovered.weak_bg_fill,
            ui.style()
                .override_font_id
                .clone()
                .unwrap_or_else(|| TextStyle::Monospace.resolve(ui.style())),
        );

        if self.last_seen_cursor_node != state.cursor_node {
            self.last_seen_cursor_node = state.cursor_node;
        }

        let sections = self.highlighting.clone();

        let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
            let layout_job = LayoutJob {
                sections: sections.borrow().clone(),
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

                render_boundaries(ui, &self.snippets, top_left, &textedit.galley);

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
                                state.cursor_node = Some(node);
                            } else {
                                state.cursor_node = None;
                            }
                        }
                        self.last_text_edit_cursor = position;
                    }
                }

                let hover_position = textedit.response.hover_pos();

                if hover_position != self.last_text_edit_hover {
                    if let Some(hover_position) =
                        hover_position.map(|p| textedit.galley.cursor_from_pos(p - top_left).index)
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
                    state.hovered_node = Some(hover_node);
                    self.last_seen_hovered_node = Some(hover_node);
                } else if self.last_seen_hovered_node != state.hovered_node {
                    self.last_text_edit_highlighting_hover = state
                        .hovered_node
                        .map(HighlightingHover::Node)
                        .unwrap_or(HighlightingHover::None);
                    self.last_seen_hovered_node = state.hovered_node;
                }
            });
    }
    fn update(
        &mut self,
        weave: &mut TapestryWeave,
        settings: &Settings,
        default_color: Color32,
        hover_bg: Color32,
        font_id: FontId,
    ) {
        let mut highlighting = self.highlighting.borrow_mut();
        self.text.clear();
        self.bytes.clear();
        self.snippets.clear();
        highlighting.clear();

        let active: Vec<u128> = weave.weave.get_active_thread().iter().copied().collect();

        let mut index = 0;

        for node in active
            .into_iter()
            .rev()
            .filter_map(|id| weave.weave.get_node(&id))
        {
            let color = get_node_color(node, settings);

            match &node.contents.content {
                InnerNodeContent::Snippet(snippet) => {
                    let byte_range = index..(index + snippet.len());

                    self.bytes.extend_from_slice(snippet);
                    self.snippets.push((snippet.len(), Ulid(node.id), index));
                    highlighting.push(LayoutSection {
                        leading_space: 0.0,
                        byte_range: byte_range.clone(),
                        format: calculate_text_format(
                            Ulid(node.id),
                            byte_range,
                            color.unwrap_or(default_color),
                            hover_bg,
                            font_id.clone(),
                            self.last_text_edit_highlighting_hover,
                        ),
                    });
                    index += snippet.len();
                }
                InnerNodeContent::Tokens(tokens) => {
                    for (token, token_metadata) in tokens {
                        let byte_range = index..(index + token.len());
                        let color = get_token_color(color, token_metadata, settings)
                            .unwrap_or(default_color);

                        self.bytes.extend_from_slice(token);
                        self.snippets.push((token.len(), Ulid(node.id), index));
                        highlighting.push(LayoutSection {
                            leading_space: 0.0,
                            byte_range: byte_range.clone(),
                            format: calculate_text_format(
                                Ulid(node.id),
                                byte_range,
                                color,
                                hover_bg,
                                font_id.clone(),
                                self.last_text_edit_highlighting_hover,
                            ),
                        });
                        index += token.len();
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

            for (length, node, _) in self.snippets.iter() {
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

        weave.set_active_content(&self.bytes, IndexMap::default(), |timestamp| {
            if let Some(timestamp) = timestamp {
                Ulid::from_datetime(SystemTime::UNIX_EPOCH + Duration::from_millis(timestamp))
            } else {
                Ulid::new()
            }
        });
    }
}

fn calculate_text_format(
    node: Ulid,
    byte_range: Range<usize>,
    color: Color32,
    hover_bg: Color32,
    font_id: FontId,
    hover: HighlightingHover,
) -> TextFormat {
    let mut format = TextFormat::simple(font_id, color);

    match hover {
        HighlightingHover::Position((_, hover_position)) => {
            if byte_range.contains(&hover_position) {
                format.background = hover_bg;
            }
        }
        HighlightingHover::Node(hover_node) => {
            if hover_node == node {
                format.background = hover_bg;
            }
        }
        HighlightingHover::None => {}
    }

    format
}

fn render_boundaries(ui: &Ui, snippets: &[Snippet], top_left: Pos2, galley: &Galley) {
    // TODO: Node boundary highlighting using textedit.galley
}

#[derive(Debug, Clone, Copy)]
enum HighlightingHover {
    Position((Ulid, usize)),
    Node(Ulid),
    None,
}
