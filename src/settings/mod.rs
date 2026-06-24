use eframe::egui::{Context, Ui, WidgetText};
use serde::{Deserialize, Serialize};

use crate::{AppShared, shared::view::View};

#[derive(Default, Debug)]
pub struct SettingsView {}

impl View<AppShared> for SettingsView {
    fn title(&self, _shared: &AppShared) -> WidgetText {
        WidgetText::Text("\u{E154} Settings".to_string())
    }
    fn logic(&mut self, shared: &mut AppShared, ctx: &Context) {}
    fn modals(&mut self, shared: &mut AppShared, ctx: &Context) -> bool {
        false
    }
    fn ui(&mut self, shared: &mut AppShared, ui: &mut Ui) {}
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Settings {}

impl Settings {
    pub fn deserialize(data: &str) -> ron::error::SpannedResult<Self> {
        ron::from_str(data)
    }
    pub fn serialize(&self) -> ron::error::Result<String> {
        ron::to_string(self)
    }
}
