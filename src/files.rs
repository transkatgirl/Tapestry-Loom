use std::path::{Path, PathBuf};

use eframe::egui::{Context, Ui};

use crate::{AppComponent, settings::Settings};

pub struct FileManager {
    settings: Settings,
}

impl AppComponent<Option<PathBuf>> for FileManager {
    fn new(settings: Settings) -> Self {
        Self { settings }
    }
    fn update_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }
    fn render(&mut self, ctx: &Context, ui: &mut Ui) -> Option<PathBuf> {
        None
    }
}
