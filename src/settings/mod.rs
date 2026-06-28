use eframe::egui::{Context, Frame, ScrollArea, Ui, WidgetText};
use serde::{Deserialize, Serialize};

use crate::{
    AppShared,
    common::{task::BACKGROUND_REFRESH_INTERVAL, view::View},
    settings::document::DocumentSettings,
};

mod document;
mod interface;

#[derive(Default, Debug)]
pub struct SettingsView {}

impl View<AppShared> for SettingsView {
    fn title(&self, _shared: &AppShared) -> WidgetText {
        WidgetText::Text("\u{E154} Settings".to_string())
    }
    fn logic(&mut self, _shared: &mut AppShared, _force_close: impl FnOnce(), _ctx: &Context) {}
    fn modals(&mut self, shared: &mut AppShared, ctx: &Context) -> bool {
        false
    }
    fn ui(&mut self, shared: &mut AppShared, ui: &mut Ui) {
        ScrollArea::both()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(ui.style().spacing.menu_margin)
                    .show(ui, |ui| {
                        shared.settings.ui(ui);
                    })
            });

        ui.request_repaint_after(BACKGROUND_REFRESH_INTERVAL);
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Settings {
    pub documents: DocumentSettings,
}

impl Settings {
    pub fn deserialize(data: &str) -> ron::error::SpannedResult<Self> {
        ron::from_str(data)
    }
    pub fn serialize(&self) -> ron::error::Result<String> {
        ron::to_string(self)
    }
}

impl Editable for Settings {
    fn ui(&mut self, ui: &mut Ui) {
        ui.heading("Document");
        self.documents.ui(ui);
    }
}

trait Editable {
    fn ui(&mut self, ui: &mut Ui);
}
