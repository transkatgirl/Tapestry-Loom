use std::{
    cell::RefCell,
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use eframe::egui::Ui;
use egui_notify::Toasts;
use parking_lot::Mutex;
use tapestry_weave::{
    universal_weave::{indexmap::IndexMap, rkyv::rancor},
    v0::TapestryWeave,
};
use threadpool::ThreadPool;
use tokio::runtime::Runtime;

use crate::settings::Settings;

pub struct Editor {
    settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
    threadpool: Rc<ThreadPool>,
    open_documents: Rc<RefCell<HashSet<PathBuf>>>,
    runtime: Arc<Runtime>,
    pub title: String,
    path: Option<PathBuf>,
    weave: Arc<Mutex<Option<TapestryWeave>>>,
}

impl Editor {
    pub fn new(
        settings: Rc<RefCell<Settings>>,
        toasts: Rc<RefCell<Toasts>>,
        threadpool: Rc<ThreadPool>,
        open_documents: Rc<RefCell<HashSet<PathBuf>>>,
        runtime: Arc<Runtime>,
        path: Option<PathBuf>,
    ) -> Self {
        if let Some(path) = &path {
            open_documents.borrow_mut().insert(path.clone());
        }

        Self {
            settings,
            toasts,
            threadpool,
            open_documents,
            runtime,
            title: "Editor".to_string(),
            path,
            weave: Arc::new(Mutex::new(None)),
        }
    }
    pub fn render(&mut self, ui: &mut Ui) {
        let settings = self.settings.borrow();

        ui.label(format!("{:#?}", self.path));

        /*self.toasts
        .borrow_mut()
        .error(format!("Document loading failed: {error:#?}"));*/
    }
    pub fn save(&mut self) {
        if let Some(path) = &self.path {
            self.open_documents.borrow_mut().remove(path);
        }
    }
}

pub fn blank_weave_bytes() -> Result<Vec<u8>, rancor::Error> {
    Ok(TapestryWeave::with_capacity(0, IndexMap::with_capacity(0)).to_versioned_bytes()?)
}
