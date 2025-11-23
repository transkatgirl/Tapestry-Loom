use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use eframe::egui::Ui;
use parking_lot::Mutex;

use crate::settings::Settings;

pub struct FileManager {
    settings: Rc<Mutex<Settings>>,
}

impl FileManager {
    pub fn new(settings: Rc<Mutex<Settings>>) -> Self {
        Self { settings }
    }
    pub fn render(&mut self, ui: &mut Ui) -> Option<PathBuf> {
        None
    }
}
