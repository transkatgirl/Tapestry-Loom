use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::Error;
use eframe::egui::Ui;
use egui_notify::Toasts;
use threadpool::ThreadPool;

use crate::settings::Settings;

pub struct Editor {
    settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
    threadpool: Rc<RefCell<ThreadPool>>,
    pub title: String,
    path: Option<PathBuf>,
    document: Option<Document>,
}

impl Editor {
    pub fn new(
        settings: Rc<RefCell<Settings>>,
        toasts: Rc<RefCell<Toasts>>,
        threadpool: Rc<RefCell<ThreadPool>>,
        path: Option<PathBuf>,
    ) -> Self {
        Self {
            settings,
            toasts,
            threadpool,
            title: "Editor".to_string(),
            path,
            document: None,
        }
    }
    pub fn render(&mut self, ui: &mut Ui) {
        /*self.toasts
        .borrow_mut()
        .error(format!("Document loading failed: {error:#?}"));*/
    }
}

struct Document {}

impl Document {
    fn load(path: &Path) -> Result<Self, Error> {
        Ok(Self {})
    }
}
