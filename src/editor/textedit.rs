use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, SystemTime},
};

use eframe::egui::{
    Color32, Pos2, ScrollArea, TextBuffer, TextEdit, TextFormat, TextStyle, Ui,
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
    cached_text: String,
    cached_bytes: Vec<u8>,
    byte_buffer: Vec<u8>,
    snippets: Rc<RefCell<Vec<Snippet>>>,
    last_seen_cursor_node: Option<Ulid>,
    last_seen_hovered_node: Option<Ulid>,
    last_text_edit_cursor: Option<CCursor>,
    last_text_edit_hover: Option<Pos2>,
    last_text_edit_highlighting_hover: HighlightingHover,
}

type Snippet = (usize, Ulid, Color32);

const SUBSTITUTION_CHAR: char = '␚';
const SUBSTITUTION_BYTE: u8 = "␚".as_bytes()[0];

impl Default for TextEditorView {
    fn default() -> Self {
        Self {
            cached_text: String::with_capacity(262144),
            cached_bytes: Vec::with_capacity(262144),
            byte_buffer: Vec::with_capacity(262144),
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
        self.cached_text.clear();
        self.cached_bytes.clear();
        self.byte_buffer.clear();
        self.snippets.borrow_mut().clear();
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
        if self.last_seen_cursor_node != state.cursor_node {
            self.update_cache(weave, settings, ui.visuals().widgets.inactive.text_color());
            self.last_seen_cursor_node = state.cursor_node;
        }

        let snippets = self.snippets.clone();
        let last_hover = self.last_text_edit_highlighting_hover;

        let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
            let default_color = ui.visuals().widgets.inactive.text_color();

            let layout_job = LayoutJob {
                sections: calculate_highlighting(
                    ui,
                    &snippets.borrow(),
                    buf.as_str().len(),
                    default_color,
                    last_hover,
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
                let textedit = TextEdit::multiline(&mut self.cached_text)
                    .frame(false)
                    .text_color(ui.visuals().widgets.inactive.text_color())
                    .min_size(ui.available_size())
                    .desired_width(ui.available_size().x)
                    .code_editor()
                    .layouter(&mut layouter)
                    .show(ui);

                // TODO: Node boundary highlighting using textedit.galley

                if textedit.response.changed() {
                    self.update_weave(weave);
                    /*if let Some(active) = weave.get_active_thread().next().map(|node| Ulid(node.id))
                    {
                        state.cursor_node = Some(active);
                    } else {
                        state.cursor_node = None;
                    }*/
                    let position = textedit.cursor_range.map(|c| c.sorted_cursors()[0]);
                    if let Some((node, _)) = self.calculate_cursor(weave, position.map(|p| p.index))
                    {
                        state.cursor_node = Some(node);
                    } else {
                        state.cursor_node = None;
                    }
                    self.last_text_edit_cursor = position;

                    self.update_snippet_cache(
                        weave,
                        settings,
                        ui.visuals().widgets.inactive.text_color(),
                    );
                    //self.update_cache(weave, settings, ui.visuals().widgets.inactive.text_color());
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
                    if let Some(hover_position) = hover_position.map(|p| {
                        textedit
                            .galley
                            .cursor_from_pos(p - textedit.text_clip_rect.left_top())
                            .index
                    }) {
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
                    //self.update_cache(weave, settings, ui.visuals().widgets.inactive.text_color());
                    self.last_text_edit_highlighting_hover = state
                        .hovered_node
                        .map(HighlightingHover::Node)
                        .unwrap_or(HighlightingHover::None);
                    self.last_seen_hovered_node = state.hovered_node;
                }
            });
    }
    fn update_cache(
        &mut self,
        weave: &mut TapestryWeave,
        settings: &Settings,
        default_color: Color32,
    ) {
        let mut snippets = self.snippets.borrow_mut();
        self.cached_text.clear();
        self.cached_bytes.clear();
        snippets.clear();

        let active: Vec<u128> = weave.weave.get_active_thread().iter().copied().collect();

        for node in active
            .into_iter()
            .rev()
            .filter_map(|id| weave.weave.get_node(&id))
        {
            let color = get_node_color(node, settings);

            match &node.contents.content {
                InnerNodeContent::Snippet(snippet) => {
                    self.cached_bytes.extend_from_slice(snippet);
                    snippets.push((snippet.len(), Ulid(node.id), color.unwrap_or(default_color)));
                }
                InnerNodeContent::Tokens(tokens) => {
                    for (token, token_metadata) in tokens {
                        self.cached_bytes.extend_from_slice(token);
                        snippets.push((
                            token.len(),
                            Ulid(node.id),
                            get_token_color(color, token_metadata, settings)
                                .unwrap_or(default_color),
                        ));
                    }
                }
            }
        }

        for chunk in self.cached_bytes.utf8_chunks() {
            self.cached_text.push_str(chunk.valid());

            for _ in chunk.invalid() {
                self.cached_text.push(SUBSTITUTION_CHAR);
            }
        }
    }
    fn update_snippet_cache(
        &mut self,
        weave: &mut TapestryWeave,
        settings: &Settings,
        default_color: Color32,
    ) {
        let mut snippets = self.snippets.borrow_mut();
        snippets.clear();

        let active: Vec<u128> = weave.weave.get_active_thread().iter().copied().collect();

        for node in active
            .into_iter()
            .rev()
            .filter_map(|id| weave.weave.get_node(&id))
        {
            let color = get_node_color(node, settings);

            match &node.contents.content {
                InnerNodeContent::Snippet(snippet) => {
                    snippets.push((snippet.len(), Ulid(node.id), color.unwrap_or(default_color)));
                }
                InnerNodeContent::Tokens(tokens) => {
                    for (token, token_metadata) in tokens {
                        snippets.push((
                            token.len(),
                            Ulid(node.id),
                            get_token_color(color, token_metadata, settings)
                                .unwrap_or(default_color),
                        ));
                    }
                }
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
            let index = self.cached_text.byte_index_from_char_index(char_index);

            let mut offset = 0;

            for (length, node, _) in self.snippets.borrow().iter() {
                offset += length;
                if offset >= index {
                    cursor_node = Some((*node, index));
                    if offset > index {
                        break;
                    }
                }
            }
        } else if let Some(active) = weave.get_active_thread().next().map(|node| Ulid(node.id)) {
            cursor_node = Some((active, self.cached_text.len()));
        } else {
            cursor_node = None;
        }

        cursor_node
    }
    fn update_weave(&mut self, weave: &mut TapestryWeave) {
        self.byte_buffer.clear();
        self.byte_buffer
            .extend_from_slice(self.cached_text.as_bytes());

        for (index, byte) in self
            .byte_buffer
            .iter_mut()
            .take(self.cached_bytes.len())
            .enumerate()
        {
            if *byte == SUBSTITUTION_BYTE {
                *byte = self.cached_bytes[index];
            }
        }

        weave.set_active_content(&self.byte_buffer, IndexMap::default(), |timestamp| {
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

    for (snippet_length, node, color) in snippets.iter().copied() {
        index += snippet_length;

        if index > length {
            index -= snippet_length;
            break;
        }

        let mut format = TextFormat::simple(font_id.clone(), color);

        let byte_range = (index - snippet_length)..index;

        match hover {
            HighlightingHover::Position((_, hover_position)) => {
                if byte_range.contains(&hover_position) {
                    format.background = ui.style().visuals.widgets.hovered.weak_bg_fill;
                }
            }
            HighlightingHover::Node(hover_node) => {
                if hover_node == node {
                    format.background = ui.style().visuals.widgets.hovered.weak_bg_fill;
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

#[derive(Debug, Clone, Copy)]
enum HighlightingHover {
    Position((Ulid, usize)),
    Node(Ulid),
    None,
}
