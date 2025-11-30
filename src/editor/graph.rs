use eframe::egui::Ui;
use egui_notify::Toasts;
use flagset::FlagSet;
use tapestry_weave::v0::TapestryWeave;

use crate::{
    editor::shared::SharedState,
    settings::{Settings, shortcuts::Shortcuts},
};

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
        state: &mut SharedState,
        shortcuts: FlagSet<Shortcuts>,
    ) {
        ui.heading("Unimplemented");

        /*if shortcuts.contains(Shortcuts::FitToCursor) {
            // TODO
        }

        if shortcuts.contains(Shortcuts::FitToWeave) {
            // TODO
        }*/
    }
}
