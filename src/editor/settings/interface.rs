use eframe::egui::Ui;
use flagset::FlagSet;
use serde::{Deserialize, Serialize};

use crate::{common::view::Edit, editor::settings::shortcuts::Shortcuts};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct InterfaceSettings {
    pub node_colors: NodeColors,
    pub token_colors: TokenColors,
    pub min_token_opacity: f32,
    #[serde(default = "default_auto_scroll")]
    pub auto_scroll: bool,

    #[serde(default)]
    last_node_colors: Option<NodeColors>,
    #[serde(default)]
    last_token_colors: Option<TokenColors>,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            node_colors: NodeColors::default(),
            token_colors: TokenColors::default(),
            min_token_opacity: 0.65,
            auto_scroll: default_auto_scroll(),
            last_node_colors: None,
            last_token_colors: None,
        }
    }
}

fn default_auto_scroll() -> bool {
    true
}

impl InterfaceSettings {
    pub fn update(&mut self, shortcuts: FlagSet<Shortcuts>) {
        if shortcuts.contains(Shortcuts::ToggleColors) {
            if matches!(self.node_colors, NodeColors::None) {
                self.node_colors = self.last_node_colors.take().unwrap_or_default();
            } else {
                self.last_node_colors = Some(self.node_colors);
                self.node_colors = NodeColors::None;
            }
        }

        if shortcuts.contains(Shortcuts::ToggleColorOverride) {
            // TODO
        }

        if shortcuts.contains(Shortcuts::ToggleProbabilities) {
            if matches!(self.token_colors, TokenColors::None) {
                self.token_colors = self.last_token_colors.take().unwrap_or_default();
            } else {
                self.last_token_colors = Some(self.token_colors);
                self.token_colors = TokenColors::None;
            }
        }

        if shortcuts.contains(Shortcuts::ToggleAutoScroll) {
            self.auto_scroll = !self.auto_scroll;
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeColors {
    None,
    #[default]
    Creator,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
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
