use eframe::egui::Ui;
use egui_notify::Toasts;
use tapestry_weave::v0::TapestryWeave;

use crate::settings::Settings;

#[derive(Default, Debug)]
pub struct GraphView {}

impl GraphView {
    pub fn reset(&mut self) {}
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
    ) {
    }
}
