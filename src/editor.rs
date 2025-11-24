use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use anyhow::Error;
use eframe::egui::Ui;
use egui_notify::Toasts;
use tapestry_weave::{universal_weave::indexmap::IndexMap, v0::TapestryWeave};
use tokio::runtime::Runtime;

use crate::settings::Settings;

pub struct Editor {
    settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
    runtime: Arc<Runtime>,
    pub title: String,
    path: Option<PathBuf>,
    document: Option<Document>,
}

impl Editor {
    pub fn new(
        settings: Rc<RefCell<Settings>>,
        toasts: Rc<RefCell<Toasts>>,
        runtime: Arc<Runtime>,
        path: Option<PathBuf>,
    ) -> Self {
        Self {
            settings,
            toasts,
            runtime,
            title: "Editor".to_string(),
            path,
            document: None,
        }
    }
    pub fn render(&mut self, ui: &mut Ui) {
        let settings = self.settings.borrow();

        ui.label(format!("{:#?}", self.path));

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

pub fn blank_weave_bytes() -> Result<Vec<u8>, Error> {
    Ok(TapestryWeave::with_capacity(0, IndexMap::with_capacity(0)).to_versioned_bytes()?)
}
