use eframe::egui::{Context, Ui, WidgetText};

use crate::{common::view::View, editor::EditorShared};

#[derive(Default, Debug)]
pub struct TextEditView {}

impl View<EditorShared> for TextEditView {
    fn title(&self, shared: &EditorShared) -> WidgetText {
        //todo!()
        WidgetText::Text("\u{E265} Editor".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, ctx: &Context) {}
    fn modals(&mut self, shared: &mut EditorShared, ctx: &Context) -> bool {
        false
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {}
}
