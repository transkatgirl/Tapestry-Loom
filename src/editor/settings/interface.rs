use eframe::egui::Ui;
use serde::{Deserialize, Serialize};

use crate::common::view::Edit;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct InterfaceSettings {
    pub node_colors: NodeColors,
    pub token_colors: TokenColors,
    pub min_token_opacity: f32,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            node_colors: NodeColors::default(),
            token_colors: TokenColors::default(),
            min_token_opacity: 0.65,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy)]
pub enum NodeColors {
    None,
    #[default]
    Creator,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy)]
pub enum TokenColors {
    None,
    Logprob,
    Confidence,
    #[default]
    HybridLogprobConfidence,
    Entropy,
}

impl Edit for InterfaceSettings {
    fn ui(&mut self, ui: &mut Ui) {}
}
