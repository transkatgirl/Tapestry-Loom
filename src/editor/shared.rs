use std::sync::Arc;

use eframe::egui::Ui;
use egui_notify::Toasts;
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::dependent::DependentNode,
    v0::{NodeContent, TapestryWeave},
};
use tokio::runtime::Runtime;

use crate::settings::Settings;

#[derive(Debug)]
pub struct SharedState {
    pub identifier: Ulid,
    pub runtime: Arc<Runtime>,
}

impl SharedState {
    pub fn new(identifier: Ulid, runtime: Arc<Runtime>) -> Self {
        Self {
            identifier,
            runtime,
        }
    }
    pub fn update(&mut self, weave: &mut TapestryWeave, settings: &Settings, toasts: &mut Toasts) {}
    pub fn reset(&mut self) {}
    pub fn run_inference(
        &mut self,
        weave: &mut TapestryWeave,
        parent: Option<Ulid>,
        settings: &Settings,
    ) {
    }
}

pub fn render_node_metadata_tooltip(ui: &mut Ui, node: &DependentNode<NodeContent>) {
    if let Some(model) = &node.contents.model {
        ui.label(&model.label);
    }

    for (key, value) in &node.contents.metadata {
        ui.label(format!("{key}: {value}"));
    }
}
