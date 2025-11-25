use std::{path::PathBuf, time::Duration};

use eframe::egui::Ui;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Settings {
    pub interface: UISettings,
    pub documents: DocumentSettings,
    pub inference: InferenceSettings,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct UISettings {}

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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct InferenceSettings {}

impl Settings {
    pub fn render(&mut self, ui: &mut Ui) -> bool {
        ui.heading("Settings");

        let mut document_location = self.documents.location.to_string_lossy().to_string();

        if ui.text_edit_singleline(&mut document_location).changed() {
            self.documents.location = PathBuf::from(document_location);
        }

        false
    }
}
