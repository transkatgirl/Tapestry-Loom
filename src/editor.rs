use eframe::egui::{Context, Ui};

use crate::{AppComponent, settings::Settings};

pub struct Editor {
    settings: Settings,
    document: Option<Document>,
}

impl AppComponent<()> for Editor {
    fn new(settings: Settings) -> Self {
        Self {
            settings,
            document: None,
        }
    }
    fn update_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }
    fn render(&mut self, ctx: &Context, ui: &mut Ui) {}
}

struct Document {}
