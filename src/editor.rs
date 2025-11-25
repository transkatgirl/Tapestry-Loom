use std::{
    cell::RefCell,
    collections::HashSet,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc, Barrier,
        mpsc::{self, Receiver, Sender},
    },
    time::Instant,
};

use eframe::egui::{Align, Layout, Spinner, TopBottomPanel, Ui};
use egui_notify::Toasts;
use egui_tiles::TileId;
use parking_lot::Mutex;
use tapestry_weave::{
    VersionedWeave,
    ulid::Ulid,
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
    old_path: Option<PathBuf>,
    weave: Arc<Mutex<Option<TapestryWeave>>>,
    error_channel: (Arc<Sender<String>>, Receiver<String>),
    last_save: Instant,
    closing: bool,
    panel_identifier: String,
    modal_identifier: String,
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

        let identifier = Ulid::new().to_string();

        Self {
            settings,
            toasts,
            threadpool,
            open_documents,
            runtime,
            title: generate_title(&path),
            path: Arc::new(Mutex::new(path.clone())),
            old_path: path,
            weave: Arc::new(Mutex::new(None)),
            error_channel: (Arc::new(sender), receiver),
            last_save: Instant::now(),
            closing: false,
            panel_identifier: ["editor-", &identifier, "-bottom-panel"].concat(),
            modal_identifier: ["editor-", &identifier, "-modal"].concat(),
        }
    }
    pub fn render(&mut self, ui: &mut Ui) {
        if let Some(mut weave) = self.weave.clone().try_lock() {
            match weave.as_mut() {
                Some(weave) => {
                    self.render_weave(ui, weave);

                    let mut toasts = self.toasts.borrow_mut();
                    while let Ok(message) = self.error_channel.1.try_recv() {
                        toasts.error(message);
                    }

                    return;
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
                            match read_bytes(filepath) {
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
                }
            }
        }

        TopBottomPanel::bottom(self.panel_identifier.clone()).show_animated_inside(
            ui,
            true,
            |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.add(Spinner::new());
                        ui.label("Loading weave...");
                    });
                });
            },
        );

        let mut toasts = self.toasts.borrow_mut();
        while let Ok(message) = self.error_channel.1.try_recv() {
            toasts.error(message);
        }
    }
    fn render_weave(&mut self, ui: &mut Ui, weave: &mut TapestryWeave) {
        let settings = self.settings.borrow();
        let path = self.path.lock();

        if self.old_path != *path {
            self.title = generate_title(&path);
            if let Some(path) = &self.old_path {
                self.open_documents.borrow_mut().remove(path);
            }
            if let Some(path) = path.as_ref() {
                self.open_documents.borrow_mut().insert(path.clone());
            }
            self.old_path = path.clone();
        }

        TopBottomPanel::bottom(self.panel_identifier.clone()).show_animated_inside(
            ui,
            true,
            |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        if let Some(path) = path.as_ref() {
                            if let Ok(path) = path.strip_prefix(&settings.documents.location) {
                                ui.label(path.to_string_lossy());
                            } else {
                                ui.label(path.to_string_lossy());
                            }
                        } else {
                            ui.button("Save As...");
                            //ui.label("New Weave");
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {});
                });
            },
        );

        if self.last_save.elapsed() > settings.documents.save_interval {
            self.last_save = Instant::now();
            self.save(false);
        }
    }
    fn save(&self, unload: bool) {
        let weave = self.weave.clone();
        let path = self.path.clone();
        let error_sender = self.error_channel.0.clone();

        self.threadpool.execute(move || {
            let mut path_lock = path.lock();
            let mut weave_lock = weave.lock();

            if let Some(path) = path_lock.as_ref()
                && let Some(weave) = weave_lock.as_ref()
            {
                match weave.to_versioned_bytes() {
                    Ok(bytes) => {
                        if let Err(error) = write_bytes(path, &bytes) {
                            let _ = error_sender.send(format!("Filesystem error: {error:#?}"));
                        } else if unload {
                            *weave_lock = None;
                            *path_lock = None;
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
        self.save(true);
        self.closing = true;

        if let Some(path) = &self.path.lock().as_ref() {
            self.open_documents.borrow_mut().remove(*path);
        }

        true
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        if !self.closing
            && let Some(weave) = self.weave.lock().as_ref()
            && let Some(path) = self.path.lock().as_ref()
            && let Ok(bytes) = weave.to_versioned_bytes()
        {
            let _ = fs::write(path, bytes);
        }
    }
}

fn generate_title(path: &Option<PathBuf>) -> String {
    match path {
        Some(path) => {
            if let Some(filename) = path.file_stem() {
                filename.to_string_lossy().to_string()
            } else {
                "Editor".to_string()
            }
        }
        None => "New Weave".to_string(),
    }
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, io::Error> {
    let mut file = File::open(path)?;
    file.lock()?;

    let size = file
        .metadata()
        .map(|m| m.len() as usize)
        .unwrap_or_default();

    let mut bytes = Vec::with_capacity(size);
    file.read_to_end(&mut bytes)?;

    Ok(bytes)
}

fn write_bytes(path: &Path, contents: &[u8]) -> Result<(), io::Error> {
    let mut file = File::create(path)?;
    file.lock()?;

    file.write_all(contents)
}

pub fn blank_weave_bytes() -> Result<Vec<u8>, rancor::Error> {
    TapestryWeave::with_capacity(0, IndexMap::with_capacity(0)).to_versioned_bytes()
}
