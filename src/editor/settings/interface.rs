use eframe::egui::Ui;
use serde::{Deserialize, Serialize};

use crate::common::view::Edit;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterfaceSettings {}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {}
    }
}

impl Edit for InterfaceSettings {
    fn ui(&mut self, ui: &mut Ui) {}
}
