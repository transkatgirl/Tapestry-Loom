use std::{
    ops::Range,
    time::{Duration, SystemTime},
};

use eframe::egui::{Color32, ScrollArea, TextBuffer, TextEdit, TextStyle, Ui};
use egui_notify::Toasts;
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{Weave, indexmap::IndexMap},
    v0::{InnerNodeContent, TapestryWeave},
};

use crate::{
    editor::shared::{SharedState, get_node_color},
    settings::Settings,
};

#[derive(Debug)]
pub struct TextEditorView {
    cached_text: String,
    cached_bytes: Vec<u8>,
    byte_buffer: Vec<u8>,
    snippets: Vec<(Range<usize>, Color32)>,
    last_seen_cursor_node: Option<Ulid>,
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
                    if let Some(active) = weave.get_active_thread().next().map(|node| Ulid(node.id))
                    {
                        state.cursor_node = Some(active);
                    } else {
                        state.cursor_node = None;
                    }
                    //self.update_cache(weave, settings, ui.visuals().widgets.inactive.text_color());
                }

                //if textedit.response.hovered() {}
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

        let mut offset = 0;

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
                }
                InnerNodeContent::Tokens(tokens) => {
                    for (token, token_metadata) in tokens {
                        self.cached_bytes.extend_from_slice(token);
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
