use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use eframe::egui::Ui;
use egui_notify::Toasts;

use crate::settings::Settings;

pub struct FileManager {
    settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
}

impl FileManager {
    pub fn new(settings: Rc<RefCell<Settings>>, toasts: Rc<RefCell<Toasts>>) -> Self {
        Self { settings, toasts }
    }
    pub fn render(&mut self, ui: &mut Ui) -> Option<PathBuf> {
        ui.heading("File Manager");

        None
    }
}
