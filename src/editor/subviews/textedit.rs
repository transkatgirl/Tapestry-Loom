use std::ops::Range;

use eframe::egui::{
    Color32, Context, TextBuffer, TextFormat, TextStyle, Ui, WidgetText,
    text::{LayoutJob, LayoutSection, TextWrapping},
};
use tapestry_weave::v1::content::InnerNodeContent;

use crate::{
    common::{ui::from_utf8_lossy_in_place, view::View},
    editor::EditorShared,
};

#[derive(Debug)]
pub struct TextEditView {
    text_buffer: String,
    byte_buffer: Vec<u8>,
    snippets: Vec<Snippet>,
}

impl Default for TextEditView {
    fn default() -> Self {
        Self {
            text_buffer: String::with_capacity(16384),
            byte_buffer: Vec::with_capacity(16384),
            snippets: Vec::with_capacity(16384),
        }
    }
}

#[derive(Debug)]
struct Snippet {
    length: usize,
    node: u64,
    color: Option<Color32>,
    token_index: Option<usize>,
}

impl TextEditView {
    fn build_contents(&mut self, shared: &mut EditorShared) {
        let weave = shared.weave.as_mut().unwrap();

        weave.get_active_content(&mut self.byte_buffer);
        from_utf8_lossy_in_place(&self.byte_buffer, &mut self.text_buffer);

        self.snippets.clear();
        for node in weave.get_active_thread() {
            let color = shared.ui.node_color(node);

            match &node.contents.content {
                InnerNodeContent::Tokens(tokens) => {
                    for (token_index, token) in tokens.iter().enumerate() {
                        if !token.bytes.is_empty() {
                            self.snippets.push(Snippet {
                                length: token.bytes.len(),
                                node: node.id,
                                color,
                                token_index: Some(token_index),
                            });
                        }
                    }
                }
                InnerNodeContent::Snippet(snippet) => {
                    if !snippet.is_empty() {
                        self.snippets.push(Snippet {
                            length: snippet.len(),
                            node: node.id,
                            color,
                            token_index: None,
                        });
                    }
                }
                InnerNodeContent::MetadataOnly => {}
            }
        }
    }
}

impl View<EditorShared> for TextEditView {
    fn title(&self, _shared: &EditorShared) -> WidgetText {
        WidgetText::Text("\u{E265} Editor".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, _force_close: impl FnOnce(), ctx: &Context) {
        if let Some(weave) = &mut shared.weave
            && weave.has_changed()
        // TODO: Handle setting changes
        {
            self.build_contents(shared);
        }
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        if let Some(weave) = &mut shared.weave {

            // TODO
        }
    }
}
