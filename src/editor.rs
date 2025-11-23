use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::Error;
use eframe::egui::Ui;
use egui_notify::Toasts;

use crate::settings::Settings;

pub struct Editor {
    settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
    //document: Document,
    pub title: String,
}

impl Editor {
    pub fn new(
        settings: Rc<RefCell<Settings>>,
        toasts: Rc<RefCell<Toasts>>,
        path: Option<&Path>,
    ) -> Option<Self> {
        /*self.toasts
        .borrow_mut()
        .error(format!("Document loading failed: {error:#?}"));*/

        Some(Self {
            settings,
            toasts,
            //document: Document::load(path)?,
            title: "Editor".to_string(),
        })
    }
    pub fn render(&mut self, ui: &mut Ui) {}
}

struct Document {}

impl Document {
    fn load(path: &Path) -> Result<Self, Error> {
        Ok(Self {})
    }
}
