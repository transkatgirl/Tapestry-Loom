use eframe::egui::{Context, Ui, WidgetText};

use crate::{common::view::View, editor::EditorShared};

#[derive(Default, Debug)]
pub struct TreeListView {}

impl View<EditorShared> for TreeListView {
    fn title(&self, shared: &EditorShared) -> WidgetText {
        //todo!()
        WidgetText::Text("\u{E408} Tree".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, ctx: &Context) {}
    fn modals(&mut self, shared: &mut EditorShared, ctx: &Context) -> bool {
        false
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {}
}

#[derive(Default, Debug)]
pub struct ListView {}

impl View<EditorShared> for ListView {
    fn title(&self, shared: &EditorShared) -> WidgetText {
        //todo!()
        WidgetText::Text("\u{E106} List".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, ctx: &Context) {}
    fn modals(&mut self, shared: &mut EditorShared, ctx: &Context) -> bool {
        false
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {}
}

#[derive(Default, Debug)]
pub struct BookmarkView {}

impl View<EditorShared> for BookmarkView {
    fn title(&self, shared: &EditorShared) -> WidgetText {
        //todo!()
        WidgetText::Text("\u{E060} Bookmarks".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, ctx: &Context) {}
    fn modals(&mut self, shared: &mut EditorShared, ctx: &Context) -> bool {
        false
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {}
}
