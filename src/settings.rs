use std::{path::PathBuf, time::Duration};

use eframe::egui::{Frame, Ui};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Settings {
    pub interface: UISettings,
    pub documents: DocumentSettings,
    pub inference: InferenceSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UISettings {
    pub max_tree_depth: usize,
}

impl Default for UISettings {
    fn default() -> Self {
        Self { max_tree_depth: 4 }
    }
}

impl UISettings {
    fn render(&mut self, ui: &mut Ui) {}
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
        let mut document_location = self.location.to_string_lossy().to_string();

        if ui.text_edit_singleline(&mut document_location).changed() {
            self.location = PathBuf::from(document_location);
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
            });
    }
}
