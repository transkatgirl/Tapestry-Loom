use std::{
    ops::Range,
    time::{Duration, SystemTime},
};

use eframe::egui::{Color32, Pos2, ScrollArea, TextBuffer, TextEdit, TextStyle, Ui, text::CCursor};
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
    snippets: Vec<(usize, Ulid, Color32)>,
    last_seen_cursor_node: Option<Ulid>,
    last_text_edit_cursor: Option<CCursor>,
    last_text_edit_hover: Option<Pos2>,
    last_text_edit_hover_node: Option<Ulid>,
}

const SUBSTITUTION_CHAR: char = '␚';
const SUBSTITUTION_BYTE: u8 = "␚".as_bytes()[0];

impl Default for TextEditorView {
    fn default() -> Self {
        Self {
            cached_text: String::with_capacity(262144),
            cached_bytes: Vec::with_capacity(262144),
            byte_buffer: Vec::with_capacity(262144),
            snippets: Vec::with_capacity(65536),
            last_seen_cursor_node: None,
            last_text_edit_cursor: None,
            last_text_edit_hover: None,
            last_text_edit_hover_node: None,
        }
    }
}

impl TextEditorView {
    pub fn reset(&mut self) {
        self.cached_text.clear();
        self.cached_bytes.clear();
        self.byte_buffer.clear();
        self.snippets.clear();
        self.last_seen_cursor_node = None;
        self.last_text_edit_cursor = None;
        self.last_text_edit_hover = None;
        self.last_text_edit_hover_node = None;
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

        /*let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
            let mut layout_job: egui::text::LayoutJob = my_memoized_highlighter(buf.as_str());
            layout_job.wrap.max_width = wrap_width;
            ui.fonts_mut(|f| f.layout_job(layout_job))
        };*/

        // TODO: Display node metadata on hover
        // TODO: Display node colors & node/token boundaries

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
                    //.layouter(&mut layouter)
                    .show(ui);

                if textedit.response.changed() {
                    self.update_weave(weave);
                    /*if let Some(active) = weave.get_active_thread().next().map(|node| Ulid(node.id))
                    {
                        state.cursor_node = Some(active);
                    } else {
                        state.cursor_node = None;
                    }*/
                    let position = textedit.cursor_range.map(|c| c.sorted_cursors()[0]);
                    state.cursor_node = self.calculate_cursor(weave, position.map(|p| p.index));
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
                        state.cursor_node = self.calculate_cursor(weave, position.map(|p| p.index));
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
                        self.last_text_edit_hover_node =
                            self.calculate_cursor(weave, Some(hover_position));
                    } else {
                        self.last_text_edit_hover_node = None;
                    }

                    self.last_text_edit_hover = hover_position;
                }

                if let Some(hover_node) = self.last_text_edit_hover_node {
                    state.hovered_node = Some(hover_node);
                }
            });
    }
    fn update_cache(
        &mut self,
        weave: &mut TapestryWeave,
        settings: &Settings,
        default_color: Color32,
    ) {
        self.cached_text.clear();
        self.cached_bytes.clear();
        self.snippets.clear();

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
                    self.snippets.push((
                        snippet.len(),
                        Ulid(node.id),
                        color.unwrap_or(default_color),
                    ));
                }
                InnerNodeContent::Tokens(tokens) => {
                    for (token, token_metadata) in tokens {
                        self.cached_bytes.extend_from_slice(token);
                        self.snippets.push((
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
        self.snippets.clear();

        let active: Vec<u128> = weave.weave.get_active_thread().iter().copied().collect();

        for node in active
            .into_iter()
            .rev()
            .filter_map(|id| weave.weave.get_node(&id))
        {
            let color = get_node_color(node, settings);

            match &node.contents.content {
                InnerNodeContent::Snippet(snippet) => {
                    self.snippets.push((
                        snippet.len(),
                        Ulid(node.id),
                        color.unwrap_or(default_color),
                    ));
                }
                InnerNodeContent::Tokens(tokens) => {
                    for (token, token_metadata) in tokens {
                        self.snippets.push((
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
    ) -> Option<Ulid> {
        let mut cursor_node = None;

        if let Some(char_index) = char_position {
            let index = self.cached_text.byte_index_from_char_index(char_index);

            let mut offset = 0;

            for (length, node, _) in &self.snippets {
                offset += length;
                if offset >= index {
                    cursor_node = Some(*node);
                    if offset > index {
                        break;
                    }
                }
            }
        } else if let Some(active) = weave.get_active_thread().next().map(|node| Ulid(node.id)) {
            cursor_node = Some(active);
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
