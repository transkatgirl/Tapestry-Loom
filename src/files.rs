use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{BTreeMap, HashSet},
    fs,
    path::{MAIN_SEPARATOR_STR, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

use eframe::egui::{Align, Layout, RichText, ScrollArea, TextStyle, Ui, Vec2};
use egui_notify::Toasts;
use notify::{
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, event::ModifyKind,
    recommended_watcher,
};
use threadpool::ThreadPool;
use unicode_segmentation::UnicodeSegmentation;
use walkdir::WalkDir;

use crate::settings::Settings;

pub struct FileManager {
    settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
    watcher: Option<RecommendedWatcher>,
    scan_threadpool: ThreadPool,
    action_threadpool: ThreadPool,
    channel: (Sender<ScanResult>, Receiver<ScanResult>),
    path: PathBuf,
    items: BTreeMap<PathBuf, ScannedItem>,
    scanned: bool,
    stop_scanning: Arc<AtomicBool>,
    last_selected_items: HashSet<PathBuf>,
}

type ScanResult = Result<ItemScanEvent, String>;

impl FileManager {
    pub fn new(settings: Rc<RefCell<Settings>>, toasts: Rc<RefCell<Toasts>>) -> Self {
        let path = settings.borrow().documents.location.clone();

        let (sender, receiver) = mpsc::channel::<ScanResult>();

        let tx = sender.clone();
        let watcher = match recommended_watcher(move |event: Result<Event, notify::Error>| {
            let insert_item = |path: PathBuf| match path.metadata() {
                Ok(metadata) => {
                    let filetype = metadata.file_type();

                    let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
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
                    let _ = tx.send(Err(format!("{error:#?}")));
                }
            };
            let remove_item = |path: PathBuf| {
                let _ = tx.send(Ok(ItemScanEvent::Delete(path)));
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
                    let _ = tx.send(Err(format!("{error:#?}")));
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
                    let _ = tx.send(Err(format!("{error:#?}")));
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
            scan_threadpool: ThreadPool::new(1),
            action_threadpool: ThreadPool::new(16),
            path,
            items: BTreeMap::new(),
            scanned: false,
            stop_scanning: Arc::new(AtomicBool::new(false)),
            last_selected_items: HashSet::with_capacity(16),
        }
    }
    pub fn render(&mut self, ui: &mut Ui) -> Option<Vec<PathBuf>> {
        self.update_items();

        let items = self.items();

        let text_style = TextStyle::Monospace;
        let row_height = (*ui).text_style_height(&text_style);
        let ch = ui.fonts_mut(|f| f.glyph_width(&text_style.resolve(ui.style()), ' '));
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show_rows(ui, row_height, items.len(), |ui, range| {
                for item in &items[range] {
                    let (padding, label) = if let Some(parent) = item.path.parent() {
                        if let Ok(without_prefix) = item.path.strip_prefix(parent) {
                            let parent_length: usize = UnicodeSegmentation::graphemes(
                                parent.to_string_lossy().as_ref(),
                                true,
                            )
                            .map(|_| 1)
                            .sum();
                            if parent_length > 2
                                && self
                                    .items
                                    .contains_key(&self.path.join(PathBuf::from(parent)))
                            {
                                (
                                    parent_length - 1,
                                    Cow::Owned(
                                        [
                                            ".",
                                            MAIN_SEPARATOR_STR,
                                            without_prefix.to_string_lossy().as_ref(),
                                        ]
                                        .concat(),
                                    ),
                                )
                            } else {
                                (0, item.path.to_string_lossy())
                            }
                        } else {
                            (0, item.path.to_string_lossy())
                        }
                    } else {
                        (0, item.path.to_string_lossy())
                    };

                    //let padding = (0..(padding)).map(|_| " ").collect::<String>();
                    let (icon, suffix) = match item.r#type {
                        ScannedItemType::File => ("📄", ""),
                        ScannedItemType::Directory => ("📂", MAIN_SEPARATOR_STR),
                        ScannedItemType::Other => ("?", ""),
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(ch * padding as f32);
                        ui.button(
                            RichText::new(format!("{icon} {label}{suffix}"))
                                .family(eframe::egui::FontFamily::Monospace),
                        );
                    });
                }
            });

        None
    }
    fn items(&self) -> Vec<ScannedItem> {
        self.items
            .iter()
            .map(|i| i.1.clone())
            .filter_map(|mut item| match item.path.strip_prefix(&self.path) {
                Ok(new_path) => {
                    item.path = new_path.to_path_buf();
                    Some(item)
                }
                Err(_) => None,
            })
            .filter(|item| {
                let lowercase_name = item
                    .path
                    .file_name()
                    .map(|s| s.to_os_string())
                    .unwrap_or_default()
                    .to_ascii_lowercase();

                #[allow(clippy::nonminimal_bool)]
                !(lowercase_name.is_empty()
                    || (item.r#type == ScannedItemType::File
                        && lowercase_name.to_string_lossy().chars().nth(0) == Some('.'))
                    || lowercase_name == "thumbs.db"
                    || lowercase_name == "thumbs.db"
                    || lowercase_name == "Thumbs.db:encryptable"
                    || lowercase_name == "ehthumbs.db"
                    || lowercase_name == "desktop.ini"
                    || item.r#type == ScannedItemType::Other)
            })
            .collect()
    }
    fn create_directory(&mut self, item: PathBuf) {
        let path = self.path.join(item);
        let tx = self.channel.0.clone();

        self.action_threadpool
            .execute(move || match fs::create_dir_all(path) {
                Ok(_) => {}
                Err(error) => {
                    let _ = tx.send(Err(format!("{error:#?}")));
                }
            });
    }
    fn move_item(&mut self, item: PathBuf, to: PathBuf) {
        let from = self.path.join(item);
        let to = self.path.join(to);
        let tx = self.channel.0.clone();

        self.action_threadpool
            .execute(move || match fs::rename(from, to) {
                Ok(_) => {}
                Err(error) => {
                    let _ = tx.send(Err(format!("{error:#?}")));
                }
            });
    }
    fn remove_item(&mut self, item: PathBuf) {
        let path = self.path.join(item);
        let tx = self.channel.0.clone();

        self.action_threadpool
            .execute(move || match path.metadata() {
                Ok(metadata) => {
                    if metadata.is_dir() {
                        match fs::remove_dir_all(path) {
                            Ok(_) => {}
                            Err(error) => {
                                let _ = tx.send(Err(format!("{error:#?}")));
                            }
                        }
                    } else {
                        match fs::remove_file(path) {
                            Ok(_) => {}
                            Err(error) => {
                                let _ = tx.send(Err(format!("{error:#?}")));
                            }
                        }
                    }
                }
                Err(error) => {
                    let _ = tx.send(Err(format!("{error:#?}")));
                }
            });
    }
    fn refresh(&mut self) {
        let mut toasts = self.toasts.borrow_mut();

        self.stop_scanning.store(true, Ordering::SeqCst);
        if self.scanned
            && let Some(watcher) = &mut self.watcher
            && let Err(error) = watcher.unwatch(&self.path)
        {
            toasts.error(format!("Unable to unwatch folder: {error:#?}"));
        }
        self.items.clear();
        self.scanned = false;
    }
    fn update_items(&mut self) {
        let settings = self.settings.borrow();

        if settings.documents.location != self.path {
            let settings_location = settings.documents.location.clone();
            drop(settings);
            self.refresh();
            self.path = settings_location;
        }

        let mut toasts = self.toasts.borrow_mut();

        if !self.scanned {
            if self.stop_scanning.load(Ordering::Relaxed) {
                self.scan_threadpool.join();
                self.stop_scanning.store(false, Ordering::SeqCst);
            }
            while self.channel.1.try_recv().is_ok() {}
            let tx = self.channel.0.clone();
            let path = self.path.clone();
            let stop_scanning = self.stop_scanning.clone();
            self.scan_threadpool.execute(move || {
                match fs::exists(&path) {
                    Ok(exists) => {
                        if !exists {
                            if let Err(error) = fs::create_dir_all(&path) {
                                let _ = tx.send(Err(format!("{error:#?}")));
                            } else {
                                return;
                            }
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(Err(format!("{error:#?}")));
                    }
                }

                let _ = tx.send(Ok(ItemScanEvent::Watch(path.clone())));

                for entry in WalkDir::new(path) {
                    if stop_scanning.load(Ordering::SeqCst) {
                        stop_scanning.store(false, Ordering::SeqCst);
                        break;
                    }

                    match entry {
                        Ok(entry) => {
                            let filetype = entry.file_type();

                            let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                                path: entry.path().to_path_buf(),
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
                            let _ = tx.send(Err(format!("{error:#?}")));
                        }
                    }
                }
            });

            self.scanned = true;
        }

        while let Ok(message) = self.channel.1.try_recv() {
            match message {
                Ok(message) => match message {
                    ItemScanEvent::Insert(insert) => {
                        self.items.insert(insert.path.clone(), insert);
                    }
                    ItemScanEvent::Delete(delete) => {
                        self.items.remove(&delete);
                    }
                    ItemScanEvent::Watch(watch) => {
                        if let Some(watcher) = &mut self.watcher
                            && let Err(error) = watcher.watch(&watch, RecursiveMode::Recursive)
                        {
                            toasts.error(format!("Unable to watch folder: {error:#?}"));
                        };
                    }
                },
                Err(error) => {
                    toasts.warning(format!("Filesystem error: {error:#?}"));
                }
            }
        }
    }
}

enum ItemScanEvent {
    Insert(ScannedItem),
    Delete(PathBuf),
    Watch(PathBuf),
}

#[derive(Debug, Clone)]
struct ScannedItem {
    path: PathBuf,
    r#type: ScannedItemType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ScannedItemType {
    File,
    Directory,
    Other,
}
