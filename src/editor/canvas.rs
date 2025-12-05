use eframe::egui::Ui;
use egui_notify::Toasts;
use flagset::FlagSet;

use crate::{
    editor::shared::{SharedState, weave::WeaveWrapper},
    settings::{Settings, shortcuts::Shortcuts},
};

#[derive(Default, Debug)]
pub struct CanvasView {}

impl CanvasView {
    pub fn reset(&mut self) {}
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
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
