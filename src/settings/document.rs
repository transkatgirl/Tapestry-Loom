use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::settings::Editable;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentSettings {
    pub location: PathBuf,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            location: dirs_next::document_dir()
                .unwrap_or_default()
                .join("Tapestry Loom WIP"), // TODO
        }
    }
}

impl Editable for DocumentSettings {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) {
        let location_hover_text = "Changes the path used by the built in file manager.\n\nFile paths in the UI are abbreviated to be relative to the root location whenever possible.";

        let location_label = ui
            .label("Root location:")
            .on_hover_text(location_hover_text);
        let mut document_location = self.location.to_string_lossy().to_string();

        if ui
            .text_edit_singleline(&mut document_location)
            .labelled_by(location_label.id)
            .on_hover_text(location_hover_text)
            .changed()
        {
            self.location = PathBuf::from(document_location);
        }
    }
}
