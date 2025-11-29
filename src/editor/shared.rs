use std::sync::Arc;

use chrono::{DateTime, offset};
use eframe::egui::{Color32, Rgba, Ui};
use egui_notify::Toasts;
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{dependent::DependentNode, indexmap::IndexMap},
    v0::{NodeContent, TapestryWeave},
};
use tokio::runtime::Runtime;

use crate::settings::Settings;

#[derive(Debug)]
pub struct SharedState {
    pub identifier: Ulid,
    pub runtime: Arc<Runtime>,
    cursor_node: Option<Ulid>,
    last_cursor_node: Option<Ulid>,
    hovered_node: Option<Ulid>,
    last_hovered_node: Option<Ulid>,
    triggered_unimplemented: bool,
}

impl SharedState {
    pub fn new(identifier: Ulid, runtime: Arc<Runtime>) -> Self {
        Self {
            identifier,
            runtime,
            cursor_node: None,
            last_cursor_node: None,
            hovered_node: None,
            last_hovered_node: None,
            triggered_unimplemented: false,
        }
    }
    pub fn update(&mut self, weave: &mut TapestryWeave, settings: &Settings, toasts: &mut Toasts) {
        self.last_hovered_node = self.hovered_node;
        self.hovered_node = None;
        if let Some(cursor_node) = self.cursor_node
            && !weave.contains(&cursor_node)
        {
            self.cursor_node = None;
        }
        if self.cursor_node.is_none()
            && let Some(active) = weave.get_active_thread().next().map(|node| Ulid(node.id))
        {
            self.cursor_node = Some(active);
        }
        self.last_cursor_node = self.cursor_node;
        if self.triggered_unimplemented {
            toasts.info("Unimplemented");
            self.triggered_unimplemented = false;
        }
    }
    pub fn reset(&mut self) {
        self.cursor_node = None;
        self.last_cursor_node = None;
        self.hovered_node = None;
        self.last_hovered_node = None;
    }
    pub fn get_cursor_node(&self) -> Option<Ulid> {
        self.last_cursor_node
    }
    pub fn get_hovered_node(&self) -> Option<Ulid> {
        self.last_hovered_node
    }
    pub fn set_cursor_node(&mut self, value: Option<Ulid>) {
        self.cursor_node = value;
    }
    pub fn set_hovered_node(&mut self, value: Option<Ulid>) {
        self.hovered_node = value;
    }
    pub fn generate_children(
        &mut self,
        weave: &mut TapestryWeave,
        parent: Option<Ulid>,
        settings: &Settings,
    ) {
        self.triggered_unimplemented = true;
    }
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

    let datetime: DateTime<offset::Local> = DateTime::from(Ulid(node.id).datetime());

    ui.label(format!("{}", datetime.format("%x %r")));

    #[cfg(debug_assertions)]
    ui.label(Ulid(node.id).to_string());
}

pub fn render_token_metadata_tooltip(ui: &mut Ui, token_metadata: &IndexMap<String, String>) {
    for (key, value) in token_metadata {
        if key == "probability"
            && let Ok(probability) = value.parse::<f32>()
        {
            ui.label(format!("probability: {:.2}%", probability * 100.0));
        } else {
            ui.label(format!("{key}: {value}"));
        }
    }
}

pub fn get_token_color(
    node_color: Option<Color32>,
    token_metadata: &IndexMap<String, String>,
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
