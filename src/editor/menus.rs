use eframe::egui::Ui;
use egui_notify::Toasts;
use flagset::FlagSet;
use tapestry_weave::v0::TapestryWeave;

use crate::{
    editor::shared::SharedState,
    settings::{Settings, shortcuts::Shortcuts},
};

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
        shortcuts: FlagSet<Shortcuts>,
    ) {
        ui.heading("Unimplemented");
    }
    pub fn render_rtl_panel(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
        shortcuts: FlagSet<Shortcuts>,
    ) {
        if shortcuts.contains(Shortcuts::ResetParameters) {
            // TODO
        }
    }
}
