use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
};

use eframe::egui::Ui;
use egui_notify::Toasts;
use notify::{
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{Flag, ModifyKind},
    recommended_watcher,
};
use parking_lot::Mutex;
use threadpool::ThreadPool;

use crate::settings::Settings;

pub struct FileManager {
    settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
    watcher: Option<RecommendedWatcher>,
    threadpool: ThreadPool,
    channel: (Sender<ScanResult>, Receiver<ScanResult>),
    path: PathBuf,
    items: Arc<Mutex<HashMap<PathBuf, ScannedItem>>>,
    scanned: bool,
}

type ScanResult = Result<ItemScanEvent, String>;

impl FileManager {
    pub fn new(settings: Rc<RefCell<Settings>>, toasts: Rc<RefCell<Toasts>>) -> Self {
        let path = settings.borrow().documents.location.clone();

        let (sender, receiver) = mpsc::channel::<ScanResult>();

        let watcher_sender = sender.clone();
        let watcher = match recommended_watcher(move |event: Result<Event, notify::Error>| {
            let insert_item = |path: PathBuf| match path.metadata() {
                Ok(metadata) => {
                    let filetype = metadata.file_type();

                    let _ = watcher_sender.send(Ok(ItemScanEvent::Insert(ScannedItem {
                        name: path.file_name().map(|n| n.to_owned()).unwrap_or_default(),
                        path,
                        r#type: if filetype.is_file() {
                            ScannedItemType::File
                        } else if filetype.is_dir() {
                            ScannedItemType::Directory
                        } else {
                            ScannedItemType::Other
                        },
                    })));
                }
                Err(error) => {
                    let _ = watcher_sender.send(Err(format!("{error:#?}")));
                }
            };
            let remove_item = |path: PathBuf| {
                let _ = watcher_sender.send(Ok(ItemScanEvent::Delete(path)));
            };
            let unknown_item = |path: PathBuf| match fs::exists(&path) {
                Ok(exists) => {
                    if exists {
                        insert_item(path);
                    } else {
                        remove_item(path);
                    }
                }
                Err(error) => {
                    let _ = watcher_sender.send(Err(format!("{error:#?}")));
                }
            };

            match event {
                Ok(event) => match event.kind {
                    EventKind::Create(_) => {
                        for path in event.paths {
                            insert_item(path);
                        }
                    }
                    EventKind::Modify(modify) => match modify {
                        ModifyKind::Any | ModifyKind::Name(_) | ModifyKind::Other => {
                            for path in event.paths {
                                unknown_item(path);
                            }
                        }
                        ModifyKind::Data(_) | ModifyKind::Metadata(_) => {}
                    },
                    EventKind::Remove(_) => {
                        for path in event.paths {
                            remove_item(path);
                        }
                    }
                    EventKind::Any | EventKind::Other => {
                        for path in event.paths {
                            unknown_item(path);
                        }
                    }
                    EventKind::Access(_) => {}
                },
                Err(error) => {
                    let _ = watcher_sender.send(Err(format!("{error:#?}")));
                }
            }
        }) {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                toasts
                    .borrow_mut()
                    .error(format!("Filesystem watcher creation failed: {error:#?}"));
                None
            }
        };

        Self {
            settings,
            toasts,
            watcher,
            channel: (sender, receiver),
            threadpool: ThreadPool::new(8),
            path,
            items: Arc::new(Mutex::new(HashMap::with_capacity(512))),
            scanned: false,
        }
    }
    pub fn render(&mut self, ui: &mut Ui) -> Option<Vec<PathBuf>> {
        self.update_items();

        ui.heading("File Manager");

        None
    }
    fn update_items(&mut self) {
        let settings = self.settings.borrow();
        let mut toasts = self.toasts.borrow_mut();

        if settings.documents.location != self.path {
            if let Some(watcher) = &mut self.watcher {
                watcher.unwatch(&self.path);
            }
            self.items.lock().clear();
            if let Some(watcher) = &mut self.watcher {
                watcher.watch(&self.path, RecursiveMode::Recursive);
            }
            self.scanned = false;
        }

        if !self.scanned {
            if let Some(watcher) = &mut self.watcher {
                watcher.watch(&self.path, RecursiveMode::Recursive);
            }

            // TODO

            self.scanned = true;
        }

        /*if let Some(watcher) = &self.watcher {
            while let Ok(message) = watcher.1.try_recv() {
                match message {
                    Ok(event) => {
                        // TODO
                    }
                    Err(error) => {
                        toasts.warning(format!("Filesystem watcher returned error: {error:#?}"));
                    }
                }
            }
        }*/
    }
}

enum ItemScanEvent {
    Insert(ScannedItem),
    Delete(PathBuf),
}

struct ScannedItem {
    name: OsString,
    path: PathBuf,
    r#type: ScannedItemType,
}

enum ScannedItemType {
    File,
    Directory,
    Other,
}
