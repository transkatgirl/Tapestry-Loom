use std::{path::PathBuf, time::Duration};

use eframe::egui::{Context, Ui};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Settings {
    interface: UISettings,
    documents: DocumentSettings,
    inference: InferenceSettings,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct UISettings {}

#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentSettings {
    location: PathBuf,
    autosave_interval: Duration,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            location: dirs_next::download_dir()
                .unwrap_or_default()
                .join("Tapestry Loom"),
            autosave_interval: Duration::from_secs(30),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct InferenceSettings {}

impl Settings {
    pub fn render(&mut self, ctx: &Context, ui: &mut Ui) -> bool {
        ui.heading("My egui Application");

        false
    }
}
