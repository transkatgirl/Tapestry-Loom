use eframe::egui::{Context, Ui, WidgetText};

use crate::{common::view::View, editor::EditorShared};

#[derive(Default, Debug)]
pub struct CanvasView {}

impl View<EditorShared> for CanvasView {
    fn title(&self, _shared: &EditorShared) -> WidgetText {
        WidgetText::Text("\u{E125} Canvas".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, _force_close: impl FnOnce(), ctx: &Context) {}
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        if let Some(weave) = &mut shared.weave {
            // TODO
        }
    }
}
