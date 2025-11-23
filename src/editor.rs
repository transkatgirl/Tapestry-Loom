use std::rc::Rc;

use eframe::egui::Ui;
use parking_lot::Mutex;

use crate::settings::Settings;

pub struct Editor {
    settings: Rc<Mutex<Settings>>,
    document: Option<Document>,
}

impl Editor {
    pub fn new(settings: Rc<Mutex<Settings>>) -> Self {
        Self {
            settings,
            document: None,
        }
    }
    pub fn render_main(&mut self, ui: &mut Ui) {}
    pub fn render_bar(&mut self, ui: &mut Ui) {}
}

struct Document {}
