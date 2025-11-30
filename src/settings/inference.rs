use std::{fmt::Display, path::PathBuf, time::Duration};

use eframe::egui::{
    ComboBox, Context, Frame, ScrollArea, Slider, SliderClamping, TextStyle, Ui, Visuals,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct InferenceSettings {}

impl InferenceSettings {
    pub(super) fn render(&mut self, ui: &mut Ui) {}
}
