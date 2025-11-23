use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::Error;
use eframe::egui::Ui;

use crate::settings::Settings;

pub struct Editor {
    settings: Rc<RefCell<Settings>>,
    document: Document,
    pub title: String,
}

impl Editor {
    pub fn new(settings: Rc<RefCell<Settings>>, path: &Path) -> Result<Self, Error> {
        Ok(Self {
            settings,
            document: Document::load(path)?,
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
