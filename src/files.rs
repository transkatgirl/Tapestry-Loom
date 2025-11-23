use std::path::{Path, PathBuf};

use eframe::egui::{Context, Ui};

use crate::settings::Settings;

pub struct FileManager {
    settings: Settings,
}

impl FileManager {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }
    pub fn update_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }
    pub fn render(&mut self, ui: &mut Ui) -> Option<PathBuf> {
        None
    }
}
