use eframe::egui::{Context, Ui, WidgetText};

use crate::{common::view::View, editor::EditorShared};

#[derive(Default, Debug)]
pub struct InfoView {}

impl View<EditorShared> for InfoView {
    fn title(&self, shared: &EditorShared) -> WidgetText {
        //todo!()
        WidgetText::Text("\u{E0F9} Info".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, ctx: &Context) {}
    fn modals(&mut self, shared: &mut EditorShared, ctx: &Context) -> bool {
        false
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {}
}

#[derive(Default, Debug)]
pub struct MenuView {}

impl View<EditorShared> for MenuView {
    fn title(&self, shared: &EditorShared) -> WidgetText {
        //todo!()
        WidgetText::Text("\u{E1B1} Menu".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, ctx: &Context) {}
    fn modals(&mut self, shared: &mut EditorShared, ctx: &Context) -> bool {
        false
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {}
}
