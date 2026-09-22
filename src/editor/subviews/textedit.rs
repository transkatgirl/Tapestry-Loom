use eframe::egui::{Color32, Context, Ui, WidgetText};
use tapestry_weave::{
    InnerNodeContent, ShortId, universal_weave::Weave, weave::wrappers::LoggedTapestryWeave,
};

use crate::{
    common::view::View,
    editor::{EditorShared, shared::ui::WeaveUi},
};

#[derive(Debug)]
pub struct TextEditView {
    text_buffer: String,
    byte_buffer: Vec<u8>,
    path_buffer: Vec<ShortId>,
    snippets: Vec<Snippet>,
}

impl Default for TextEditView {
    fn default() -> Self {
        Self {
            text_buffer: String::with_capacity(16384),
            byte_buffer: Vec::with_capacity(16384),
            path_buffer: Vec::with_capacity(1024),
            snippets: Vec::with_capacity(16384),
        }
    }
}

#[derive(Debug)]
struct Snippet {
    length: usize,
    node: ShortId,
    color: Option<Color32>,
    token_index: Option<usize>,
}

impl TextEditView {
    fn build_contents(&mut self, weave: &mut LoggedTapestryWeave, ui: &mut WeaveUi) {
        self.byte_buffer.clear();
        self.byte_buffer.extend(weave.active_text());
        from_utf8_lossy_in_place(&self.byte_buffer, &mut self.text_buffer);

        self.snippets.clear();
        weave.get_active_path(&mut self.path_buffer);

        for id in self.path_buffer.iter().rev() {
            let Some(node) = weave.get(id) else {
                continue;
            };
            let color = ui.node_color(node);

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
    fn logic(&mut self, shared: &mut EditorShared, _force_close: impl FnOnce(), _ctx: &Context) {
        if shared.weave_changed
            // TODO: Handle setting changes
            && let Some(weave) = &mut shared.weave
        {
            self.build_contents(weave, &mut shared.ui);
        }
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        if let Some(weave) = &mut shared.weave {

            // TODO
        }
    }
}

fn from_utf8_lossy_in_place(input: &[u8], output: &mut String) {
    output.clear();
    output.reserve(input.len());

    const REPLACEMENT: char = '\u{1A}';

    debug_assert_eq!(REPLACEMENT.len_utf8(), 1);

    for chunk in input.utf8_chunks() {
        output.push_str(chunk.valid());

        for _ in chunk.invalid() {
            output.push(REPLACEMENT);
        }
    }

    debug_assert_eq!(input.len(), output.len());
}
