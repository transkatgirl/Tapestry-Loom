use eframe::egui::Ui;
use egui_notify::Toasts;
use tapestry_weave::v0::TapestryWeave;

use crate::{editor::shared::SharedState, settings::Settings};

#[derive(Default, Debug)]
pub struct MenuView {}

impl MenuView {
    pub fn reset(&mut self) {}
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
    }
    pub fn render_rtl_panel(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
    }
}
