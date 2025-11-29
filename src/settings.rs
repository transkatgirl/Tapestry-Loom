use std::{path::PathBuf, time::Duration};

use eframe::egui::{Frame, ScrollArea, Slider, SliderClamping, Ui};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Settings {
    pub interface: UISettings,
    pub documents: DocumentSettings,
    pub inference: InferenceSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UISettings {
    pub show_model_colors: bool,
    pub show_token_probabilities: bool,
    pub max_tree_depth: usize,
}

impl Default for UISettings {
    fn default() -> Self {
        Self {
            show_model_colors: true,
            show_token_probabilities: true,
            max_tree_depth: 4,
        }
    }
}

impl UISettings {
    fn render(&mut self, ui: &mut Ui) {
        ui.checkbox(&mut self.show_model_colors, "Show model colors");
        ui.checkbox(
            &mut self.show_token_probabilities,
            "Show token probabilities",
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
}
