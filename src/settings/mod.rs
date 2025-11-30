use std::{fmt::Display, path::PathBuf, time::Duration};

use eframe::egui::{
    ComboBox, Context, Frame, ScrollArea, Slider, SliderClamping, TextStyle, Ui, Visuals,
};
use flagset::FlagSet;
use serde::{Deserialize, Serialize};

use crate::settings::shortcuts::{KeyboardShortcuts, Shortcuts};

pub mod shortcuts;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Settings {
    pub interface: UISettings,
    pub shortcuts: KeyboardShortcuts,
    pub documents: DocumentSettings,
    pub inference: InferenceSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct UISettings {
    pub ui_scale: f32,
    pub ui_theme: UITheme,
    pub displayed_ui_scale: f32,
    pub show_model_colors: bool,
    pub show_token_probabilities: bool,
    pub max_tree_depth: usize,
}

impl Default for UISettings {
    fn default() -> Self {
        Self {
            ui_scale: 1.25,
            ui_theme: UITheme::Dark,
            displayed_ui_scale: 1.25,
            show_model_colors: true,
            show_token_probabilities: true,
            max_tree_depth: 8,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UITheme {
    Dark,
    Light,
}

impl Display for UITheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dark => f.write_str("Dark"),
            Self::Light => f.write_str("Light"),
        }
    }
}

impl UITheme {
    fn get_visuals(&self) -> Visuals {
        match &self {
            Self::Dark => Visuals::dark(),
            Self::Light => Visuals::light(),
        }
    }
}

impl UISettings {
    pub fn apply(&self, ctx: &Context) {
        ctx.set_zoom_factor(self.ui_scale);
        ctx.set_visuals(self.ui_theme.get_visuals());
    }
    fn render(&mut self, ui: &mut Ui) {
        ComboBox::from_label("Theme")
            .selected_text(format!("{:?}", self.ui_theme))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.ui_theme, UITheme::Dark, UITheme::Dark.to_string());
                ui.selectable_value(
                    &mut self.ui_theme,
                    UITheme::Light,
                    UITheme::Light.to_string(),
                );
            });
        let ui_slider = ui.add(
            Slider::new(&mut self.displayed_ui_scale, 0.5..=4.0)
                .logarithmic(true)
                .clamping(SliderClamping::Never)
                .text("Scale")
                .suffix("x"),
        );
        if !(ui_slider.has_focus() || ui_slider.hovered()) {
            self.ui_scale = self.displayed_ui_scale;
        }

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.75);

        ui.checkbox(&mut self.show_model_colors, "Show model colors");
        ui.checkbox(
            &mut self.show_token_probabilities,
            "Show token probabilities in editor",
        );
        ui.add(
            Slider::new(&mut self.max_tree_depth, 1..=32)
                .clamping(SliderClamping::Never)
                .text("Maximum tree list depth"),
        );

        // TODO: Add editor layout presets
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentSettings {
    pub location: PathBuf,
    pub save_interval: Duration,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            location: dirs_next::document_dir()
                .unwrap_or_default()
                .join("Tapestry Loom"),
            save_interval: Duration::from_secs(30),
        }
    }
}

impl DocumentSettings {
    fn render(&mut self, ui: &mut Ui) {
        let location_label = ui.label("Root location:");
        let mut document_location = self.location.to_string_lossy().to_string();

        if ui
            .text_edit_singleline(&mut document_location)
            .labelled_by(location_label.id)
            .changed()
        {
            self.location = PathBuf::from(document_location);
        }

        let mut save_interval = self.save_interval.as_secs_f32();
        if ui
            .add(
                Slider::new(&mut save_interval, 1.0..=600.0)
                    .clamping(SliderClamping::Never)
                    .logarithmic(true)
                    .suffix("s")
                    .text("Autosave interval"),
            )
            .changed()
        {
            self.save_interval = Duration::from_secs_f32(save_interval);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct InferenceSettings {}

impl InferenceSettings {
    fn render(&mut self, ui: &mut Ui) {}
}

impl Settings {
    pub fn render(&mut self, ui: &mut Ui) {
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(ui.style().spacing.menu_margin)
                    .show(ui, |ui| {
                        ui.heading("Interface");
                        self.interface.render(ui);
                        ui.separator();
                        ui.heading("Shortcuts");
                        self.shortcuts.render(ui);
                        ui.separator();
                        ui.heading("Document");
                        self.documents.render(ui);
                        ui.separator();
                        ui.heading("Inference");
                        self.inference.render(ui);

                        #[cfg(debug_assertions)]
                        {
                            ui.separator();
                            ui.collapsing("Debug", |ui| {
                                ui.ctx().clone().settings_ui(ui);
                                //ui.ctx().clone().inspection_ui(ui);
                                ui.ctx().clone().texture_ui(ui);
                                ui.ctx().clone().memory_ui(ui);
                            });
                        }
                    });
            });
    }
    pub fn handle_shortcuts(&mut self, shortcuts: FlagSet<Shortcuts>) {
        if shortcuts.contains(Shortcuts::ToggleColors) {
            self.interface.show_model_colors = !self.interface.show_model_colors;
        }
    }
}
