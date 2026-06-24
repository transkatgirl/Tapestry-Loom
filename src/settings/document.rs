use std::{path::PathBuf, time::Duration};

use eframe::egui::{Slider, SliderClamping};
use serde::{Deserialize, Serialize};

use crate::{APP_NAME, settings::Editable};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentSettings {
    pub location: PathBuf,
    pub save_interval: Duration,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            location: dirs_next::document_dir().unwrap_or_default().join(APP_NAME),
            save_interval: Duration::from_secs(30),
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

        let mut save_interval = self.save_interval.as_secs_f32();
        if ui
            .add(
                Slider::new(&mut save_interval, 1.0..=600.0)
                    .clamping(SliderClamping::Never)
                    .logarithmic(true)
                    .suffix("s")
                    .text("Autosave interval"),
            )
            .on_hover_text("Weaves are automatically saved at fixed intervals based on this setting.\n\nIn addition to the autosave interval, weaves will be automatically saved on application close.")
            .changed()
        {
            self.save_interval = Duration::from_secs_f32(save_interval);
        }
    }
}
