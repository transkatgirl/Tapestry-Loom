use std::ops::Range;

use eframe::egui::{Color32, Frame, Id, ScrollArea, TextBuffer, TextEdit, TextStyle, Ui};
use egui_notify::Toasts;
use tapestry_weave::{ulid::Ulid, v0::TapestryWeave};

use crate::{editor::shared::SharedState, settings::Settings};

#[derive(Debug)]
pub struct TextEditorView {
    cached_text: String,
    byte_buffer: Vec<u8>,
    snippets: Vec<(Range<usize>, Option<Color32>)>,
    last_seen_cursor_node: Option<Ulid>,
}

impl Default for TextEditorView {
    fn default() -> Self {
        Self {
            cached_text: String::with_capacity(262144),
            byte_buffer: Vec::with_capacity(262144),
            snippets: Vec::with_capacity(65536),
            last_seen_cursor_node: None,
        }
    }
}

impl TextEditorView {
    pub fn reset(&mut self) {
        self.cached_text.clear();
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
            self.update_cache(weave);
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
                }

                //if textedit.response.hovered() {}
            });
    }
    fn update_cache(&mut self, weave: &mut TapestryWeave) {
        self.cached_text.clear();
        self.byte_buffer.clear();
        self.snippets.clear();

        /*for chunk in b"test".utf8_chunks() {
            self.cached_text.push_str(chunk.valid());

            for _ in chunk.invalid() {
                self.cached_text.push('␚');
            }
        }*/
    }
    fn update_weave(&mut self, weave: &mut TapestryWeave) {}
}
