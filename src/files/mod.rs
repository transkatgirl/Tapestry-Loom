use eframe::egui::{Context, Ui, WidgetText};

use crate::{AppShared, shared::view::View};

#[derive(Default, Debug)]
pub struct FileManager {}

impl View<AppShared> for FileManager {
    fn title(&self, _shared: &AppShared) -> WidgetText {
        WidgetText::Text("\u{E33C} Files".to_string())
    }
    fn logic(&mut self, shared: &mut AppShared, ctx: &Context) {}
    fn modals(&mut self, shared: &mut AppShared, ctx: &Context) -> bool {
        false
    }
    fn ui(&mut self, shared: &mut AppShared, ui: &mut Ui) {}
}
