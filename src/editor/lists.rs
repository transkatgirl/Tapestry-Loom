use eframe::egui::Ui;
use egui_notify::Toasts;
use tapestry_weave::{ulid::Ulid, v0::TapestryWeave};

use crate::{editor::shared::SharedState, settings::Settings};

#[derive(Default, Debug)]
pub struct ListView {}

impl ListView {
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
}

#[derive(Default, Debug)]
pub struct TreeListView {
    hoist: Option<Ulid>,
}

impl TreeListView {
    pub fn reset(&mut self) {
        self.hoist = None;
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
    }
}
