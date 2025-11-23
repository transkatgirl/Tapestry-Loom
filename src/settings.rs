use eframe::egui::{Context, Ui};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Settings {}

impl Settings {
    pub fn render(&mut self, ctx: &Context, ui: &mut Ui) -> bool {
        ui.heading("My egui Application");

        false
    }
}
