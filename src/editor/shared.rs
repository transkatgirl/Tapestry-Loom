use std::{collections::HashMap, sync::Arc};

use eframe::egui::{Color32, Rgba, Ui};
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
    pub cursor_node: Option<Ulid>,
}

impl SharedState {
    pub fn new(identifier: Ulid, runtime: Arc<Runtime>) -> Self {
        Self {
            identifier,
            runtime,
            cursor_node: None,
        }
    }
    pub fn update(&mut self, weave: &mut TapestryWeave, settings: &Settings, toasts: &mut Toasts) {
        self.cursor_node = None;
    }
    pub fn reset(&mut self) {
        self.cursor_node = None;
    }
    pub fn generate_children(
        &mut self,
        weave: &mut TapestryWeave,
        parent: Option<Ulid>,
        settings: &Settings,
    ) {
    }
}

pub fn should_render_node_metadata_tooltip(node: &DependentNode<NodeContent>) -> bool {
    !(node.contents.metadata.is_empty() && node.contents.model.is_none())
}

pub fn render_node_metadata_tooltip(ui: &mut Ui, node: &DependentNode<NodeContent>) {
    ui.set_max_width(ui.spacing().tooltip_width);

    if let Some(model) = &node.contents.model {
        if let Some(color) = model
            .metadata
            .get("color")
            .and_then(|h| Color32::from_hex(h).ok())
        {
            ui.colored_label(color, &model.label);
        } else {
            ui.label(&model.label);
        }
    }

    for (key, value) in &node.contents.metadata {
        ui.label(format!("{key}: {value}"));
    }
}

pub fn get_token_color(
    node_color: Option<Color32>,
    token_metadata: HashMap<String, String>,
    settings: &Settings,
) -> Option<Color32> {
    if let Some(color) = node_color {
        if settings.interface.show_token_probabilities
            && let Some(probability) = token_metadata
                .get("key")
                .and_then(|p| p.parse::<f32>().ok())
        {
            let probability = probability.clamp(0.0, 1.0);
            let rgba = Rgba::from(color).to_opaque();
            let opacity = (1.0 - (f32::log10(1.0 / probability)) / 4.0).min(0.3);

            Some(Color32::from(Rgba::from_rgba_unmultiplied(
                rgba.r(),
                rgba.g(),
                rgba.b(),
                opacity,
            )))
        } else {
            Some(color)
        }
    } else if settings.interface.show_token_probabilities
        && let Some(probability) = token_metadata
            .get("key")
            .and_then(|p| p.parse::<f32>().ok())
    {
        // TODO: Perform color blending in perceptual color space
        let probability = probability.clamp(0.0, 1.0);
        let (r, g, b) = if probability > 0.5 {
            ((1.0 - ((probability - 0.5) * 2.0)) * 255.0, 255.0, 0.0)
        } else {
            (255.0, (probability * 2.0) * 255.0, 0.0)
        };

        Some(Color32::from(Rgba::from_rgb(r, g, b)))
    } else {
        None
    }
}

pub fn get_node_color(node: &DependentNode<NodeContent>, settings: &Settings) -> Option<Color32> {
    if settings.interface.show_model_colors {
        node.contents.model.as_ref().and_then(|model| {
            model
                .metadata
                .get("color")
                .and_then(|h| Color32::from_hex(h).ok())
        })
    } else {
        None
    }
}
