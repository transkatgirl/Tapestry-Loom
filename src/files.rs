use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use eframe::egui::Ui;
use egui_notify::Toasts;
use threadpool::ThreadPool;

use crate::settings::Settings;

pub struct FileManager {
    settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
    threadpool: ThreadPool,
    path: PathBuf,
}

impl FileManager {
    pub fn new(settings: Rc<RefCell<Settings>>, toasts: Rc<RefCell<Toasts>>) -> Self {
        let path = settings.borrow().documents.location.clone();

        Self {
            path,
            settings,
            toasts,
            threadpool: ThreadPool::new(4),
        }
    }
    pub fn render(&mut self, ui: &mut Ui) -> Option<Vec<PathBuf>> {
        let settings = self.settings.borrow();

        ui.heading("File Manager");

        None
    }
}
