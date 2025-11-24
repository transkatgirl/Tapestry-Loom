use std::{
    cell::RefCell,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc, Barrier,
        mpsc::{self, Receiver, Sender},
    },
};

use eframe::egui::{Spinner, Ui};
use egui_notify::Toasts;
use parking_lot::Mutex;
use tapestry_weave::{
    VersionedWeave,
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
    path: Arc<Mutex<Option<PathBuf>>>,
    weave: Arc<Mutex<Option<TapestryWeave>>>,
    error_channel: (Arc<Sender<String>>, Receiver<String>),
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

        let (sender, receiver) = mpsc::channel();

        Self {
            settings,
            toasts,
            threadpool,
            open_documents,
            runtime,
            title: "Editor".to_string(),
            path: Arc::new(Mutex::new(path)),
            weave: Arc::new(Mutex::new(None)),
            error_channel: (Arc::new(sender), receiver),
        }
    }
    pub fn render(&mut self, ui: &mut Ui) {
        if let Some(mut weave) = self.weave.try_lock() {
            match weave.as_mut() {
                Some(weave) => {
                    // TODO,
                    ui.label("weave loaded");
                }
                None => {
                    drop(weave);
                    let weave = self.weave.clone();
                    let barrier = Arc::new(Barrier::new(2));
                    let thread_barrier = barrier.clone();
                    let path = self.path.clone();
                    let error_sender = self.error_channel.0.clone();

                    self.threadpool.execute(move || {
                        let mut weave_dest = weave.lock();
                        let mut path = path.lock();
                        thread_barrier.wait();

                        if let Some(filepath) = path.as_deref() {
                            match fs::read(filepath) {
                                Ok(bytes) => match VersionedWeave::from_bytes(&bytes) {
                                    Some(Ok(weave)) => {
                                        let mut weave = weave.into_latest();

                                        if weave.capacity() < 16384 {
                                            weave.reserve(16384 - weave.capacity());
                                        }

                                        *weave_dest = Some(weave);
                                    }
                                    Some(Err(error)) => {
                                        let _ = error_sender.send(format!(
                                            "Weave deserialization failed: {error:#?}"
                                        ));
                                        *path = None;
                                        *weave_dest = Some(TapestryWeave::with_capacity(
                                            16384,
                                            IndexMap::default(),
                                        ));
                                    }
                                    None => {
                                        let _ =
                                            error_sender.send("Invalid weave header".to_string());
                                        *path = None;
                                        *weave_dest = Some(TapestryWeave::with_capacity(
                                            16384,
                                            IndexMap::default(),
                                        ));
                                    }
                                },
                                Err(error) => {
                                    let _ =
                                        error_sender.send(format!("Filesystem error: {error:#?}"));
                                    *path = None;
                                    *weave_dest = Some(TapestryWeave::with_capacity(
                                        16384,
                                        IndexMap::default(),
                                    ));
                                }
                            }
                        } else {
                            *weave_dest =
                                Some(TapestryWeave::with_capacity(16384, IndexMap::default()));
                        }
                    });
                    barrier.wait();

                    //
                }
            }
        } else {
            ui.add(Spinner::new());
        }

        //let settings = self.settings.borrow();

        ui.label(format!("{:#?}", self.path));

        /*self.toasts
        .borrow_mut()
        .error(format!("Document loading failed: {error:#?}"));*/

        let mut toasts = self.toasts.borrow_mut();
        while let Ok(message) = self.error_channel.1.try_recv() {
            toasts.error(message);
        }
    }
    fn save(&self) {
        let weave = self.weave.clone();
        let path = self.path.clone();
        let error_sender = self.error_channel.0.clone();

        self.threadpool.execute(move || {
            if let Some(path) = path.lock().as_ref()
                && let Some(weave) = weave.lock().as_ref()
            {
                match weave.to_versioned_bytes() {
                    Ok(bytes) => {
                        if let Err(error) = fs::write(path, bytes) {
                            let _ = error_sender.send(format!("Filesystem error: {error:#?}"));
                        }
                    }
                    Err(error) => {
                        let _ =
                            error_sender.send(format!("Weave serialization failed: {error:#?}"));
                    }
                }
            }
        });
    }
    pub fn close(&mut self) -> bool {
        self.save();

        if let Some(path) = &self.path.lock().as_ref() {
            self.open_documents.borrow_mut().remove(*path);
        }

        true
    }
}

pub fn blank_weave_bytes() -> Result<Vec<u8>, rancor::Error> {
    TapestryWeave::with_capacity(0, IndexMap::with_capacity(0)).to_versioned_bytes()
}
