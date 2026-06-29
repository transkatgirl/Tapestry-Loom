use eframe::egui::{Slider, SliderClamping, Ui};
use serde::{Deserialize, Serialize};

use crate::settings::{Editable, SettingsView};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterfaceSettings {}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {}
    }
}

impl Editable<SettingsView> for InterfaceSettings {
    fn ui(&mut self, shared: &mut SettingsView, ui: &mut Ui) {}
}
