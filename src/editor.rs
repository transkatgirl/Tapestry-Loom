use eframe::egui::{Context, Ui};

use crate::settings::Settings;

pub struct Editor {
    settings: Settings,
    document: Option<Document>,
}

impl Editor {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            document: None,
        }
    }
    pub fn update_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }
    pub fn render_main(&mut self, ui: &mut Ui) {}
    pub fn render_bar(&mut self, ui: &mut Ui) {}
}

struct Document {}
