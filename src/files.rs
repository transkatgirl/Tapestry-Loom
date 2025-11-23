use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use eframe::egui::Ui;

use crate::settings::Settings;

pub struct FileManager {
    settings: Rc<RefCell<Settings>>,
}

impl FileManager {
    pub fn new(settings: Rc<RefCell<Settings>>) -> Self {
        Self { settings }
    }
    pub fn render(&mut self, ui: &mut Ui) -> Option<PathBuf> {
        None
    }
}
