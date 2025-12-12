use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashSet},
    ffi::OsString,
    fs, io,
    ops::DerefMut,
    path::{MAIN_SEPARATOR_STR, Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

use eframe::egui::{
    Align, Button, Frame, Layout, Modal, OutputCommand, RichText, ScrollArea, Sides, Spinner,
    TextStyle, TopBottomPanel, Ui,
};
use egui_notify::Toasts;
use flagset::FlagSet;
use log::warn;
use tapestry_weave::{VERSIONED_WEAVE_FILE_EXTENSION, treeless::FILE_EXTENSION};
use threadpool::ThreadPool;
use unicode_segmentation::UnicodeSegmentation;
use walkdir::WalkDir;

use crate::{
    editor::blank_weave_bytes,
    listing_margin,
    settings::{Settings, shortcuts::Shortcuts},
};

// TODO: Update this to use logical ordering (directories before files, 1000.txt > 1.txt)

pub struct FileManager {
    settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
    open_documents: Rc<RefCell<HashSet<PathBuf>>>,
    scan_threadpool: ThreadPool,
    action_threadpool: Rc<ThreadPool>,
    channel: (Sender<ScanResult>, Receiver<ScanResult>),
    path: PathBuf,
    roots: BTreeSet<PathBuf>,
    items: BTreeMap<PathBuf, TreeItem>,
    item_list: Vec<ScannedItem>,
    scanned: bool,
    finished: bool,
    file_count: usize,
    folder_count: usize,
    stop_scanning: Arc<AtomicBool>,
    open_folders: HashSet<PathBuf>,
    ignore_list: HashSet<&'static str>,
    modal: RefCell<ModalType>,
}

type ScanResult = Result<ItemScanEvent, anyhow::Error>;

impl FileManager {
    pub fn new(
        settings: Rc<RefCell<Settings>>,
        toasts: Rc<RefCell<Toasts>>,
        action_threadpool: Rc<ThreadPool>,
        open_documents: Rc<RefCell<HashSet<PathBuf>>>,
    ) -> Self {
        let path = settings.borrow().documents.location.clone();

        let (sender, receiver) = mpsc::channel::<ScanResult>();

        Self {
            settings,
            toasts,
            open_documents,
            channel: (sender, receiver),
            scan_threadpool: ThreadPool::new(1),
            action_threadpool,
            path,
            roots: BTreeSet::new(),
            items: BTreeMap::new(),
            item_list: Vec::with_capacity(65536),
            scanned: false,
            finished: false,
            file_count: 0,
            folder_count: 0,
            stop_scanning: Arc::new(AtomicBool::new(false)),
            open_folders: HashSet::with_capacity(256),
            ignore_list: HashSet::from_iter([
                ".directory",
                ".ds_store",
                "__macosx",
                ".appledouble",
                ".lsoverride",
                "thumbs.db",
                "thumbs.db:encryptable",
                "ehthumbs.db",
                "desktop.ini",
            ]),
            modal: RefCell::new(ModalType::None),
        }
    }
    pub fn update(&mut self) {
        self.scanned = false;
    }
    pub fn render(&mut self, ui: &mut Ui, _shortcuts: FlagSet<Shortcuts>) -> Vec<PathBuf> {
        self.update_items();

        TopBottomPanel::bottom("filemanager-bottom-panel").show_animated_inside(ui, true, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if !self.finished {
                        ui.add(Spinner::new());
                    }
                    let file_label = if self.file_count == 1 {
                        "file"
                    } else {
                        "files"
                    };
                    let folder_label = if self.folder_count.saturating_sub(1) == 1 {
                        "folder"
                    } else {
                        "folders"
                    };
                    ui.label(format!(
                        "{} {file_label}, {} {folder_label}",
                        self.file_count,
                        self.folder_count.saturating_sub(1)
                    ))
                    .on_hover_text(self.path.to_string_lossy())
                    .context_menu(|ui| {
                        if ui.button("Copy path").clicked() {
                            ui.output_mut(|o| {
                                o.commands.push(OutputCommand::CopyText(
                                    self.path.to_string_lossy().to_string(),
                                ))
                            });
                        };
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("\u{E145}").on_hover_text("Refresh").clicked() {
                        self.open_folders.clear();
                        self.scanned = false;
                    }
                    if ui.button("\u{E0D9}").on_hover_text("New folder").clicked() {
                        *self.modal.borrow_mut() =
                            ModalType::CreateDirectory("Untitled Folder".to_string());
                    }
                    if ui.button("\u{E0C9}").on_hover_text("New weave").clicked() {
                        *self.modal.borrow_mut() = ModalType::CreateWeave(
                            ["Untitled.", VERSIONED_WEAVE_FILE_EXTENSION].concat(),
                        );
                    }
                });
            });
        });

        let mut selected_items = Vec::new();

        let items = self.item_list.clone();

        let text_style = TextStyle::Monospace;
        let row_height = ui.spacing().interact_size.y;
        let ch = ui.fonts_mut(|f| f.glyph_width(&text_style.resolve(ui.style()), ' '));
        let file_extension_normal = OsString::from(VERSIONED_WEAVE_FILE_EXTENSION);
        let file_extension_treeless = OsString::from(FILE_EXTENSION);
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show_rows(ui, row_height, items.len(), |ui, range| {
                Frame::new()
                    .outer_margin(listing_margin(ui))
                    .show(ui, |ui| {
                        for item in &items[range] {
                            let (padding, label) = if let Some(parent) = item.path.parent() {
                                if let Ok(without_prefix) = item.path.strip_prefix(parent) {
                                    let parent_length: usize = UnicodeSegmentation::graphemes(
                                        parent.to_string_lossy().as_ref(),
                                        true,
                                    )
                                    .map(|_| 1)
                                    .sum();
                                    if parent_length > 0 && self.items.contains_key(parent) {
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

                            let full_path = self.path.join(&item.path);

                            let (icon, suffix) = match item.r#type {
                                ScannedItemType::File => ("📄", ""),
                                ScannedItemType::Directory => ("📂", MAIN_SEPARATOR_STR),
                                ScannedItemType::Other => ("?", ""),
                            };
                            ui.horizontal(|ui| {
                                ui.add_space(ch * padding as f32);

                                let mut button = Button::new(
                                    RichText::new(format!("{icon} {label}{suffix}"))
                                        .family(eframe::egui::FontFamily::Monospace),
                                );
                                let mut enabled = true;

                                if item.r#type == ScannedItemType::File {
                                    if !(item.path.extension() == Some(&file_extension_normal)
                                        || item.path.extension() == Some(&file_extension_treeless))
                                        || self.open_documents.borrow().contains(&full_path)
                                    {
                                        enabled = false;
                                    }
                                } else if self.open_folders.contains(&item.path) {
                                    //button = button.selected(true);
                                    button = button.fill(ui.style().visuals.extreme_bg_color);
                                }

                                let button_response = ui.add_enabled(enabled, button);

                                if !self.open_documents.borrow().contains(&full_path) {
                                    button_response.context_menu(|ui| {
                                        if item.r#type == ScannedItemType::Directory {
                                            if ui.button("New weave").clicked() {
                                                *self.modal.borrow_mut() = ModalType::CreateWeave(
                                                    item.path
                                                        .join(
                                                            [
                                                                "Untitled.",
                                                                VERSIONED_WEAVE_FILE_EXTENSION,
                                                            ]
                                                            .concat(),
                                                        )
                                                        .to_string_lossy()
                                                        .to_string(),
                                                );
                                            }
                                            if ui.button("New folder").clicked() {
                                                *self.modal.borrow_mut() =
                                                    ModalType::CreateDirectory(
                                                        item.path
                                                            .join("Untitled Folder")
                                                            .to_string_lossy()
                                                            .to_string(),
                                                    );
                                            }
                                            ui.separator();
                                        } else if item.r#type == ScannedItemType::File
                                            && (item.path.extension()
                                                == Some(&file_extension_normal)
                                                || item.path.extension()
                                                    == Some(&file_extension_treeless))
                                        {
                                            if ui.button("Open weave").clicked() {
                                                selected_items.push(full_path.clone());
                                            }
                                            ui.separator();
                                        };

                                        if ui.button("Copy item path").clicked() {
                                            ui.output_mut(|o| {
                                                o.commands.push(OutputCommand::CopyText(
                                                    self.path
                                                        .join(&item.path)
                                                        .to_string_lossy()
                                                        .to_string(),
                                                ))
                                            });
                                        };

                                        ui.separator();

                                        if ui.button("Duplicate item").clicked() {
                                            *self.modal.borrow_mut() = ModalType::Copy((
                                                item.path.clone(),
                                                item.path.to_string_lossy().to_string(),
                                            ));
                                        }

                                        if ui.button("Rename item").clicked() {
                                            *self.modal.borrow_mut() = ModalType::Rename((
                                                item.path.clone(),
                                                item.path.to_string_lossy().to_string(),
                                            ));
                                        };

                                        if ui.button("Delete item").clicked() {
                                            *self.modal.borrow_mut() =
                                                ModalType::Delete(item.path.clone());
                                        };
                                    });
                                }

                                if button_response.clicked() {
                                    if item.r#type == ScannedItemType::File {
                                        selected_items.push(full_path.clone());
                                    } else {
                                        if self.open_folders.contains(&item.path) {
                                            self.open_folders.remove(&item.path);
                                        } else {
                                            self.open_folders.insert(item.path.clone());
                                        }
                                        self.update_item_list();
                                    }
                                };

                                if ui.rect_contains_pointer(ui.max_rect())
                                    && !self.open_documents.borrow().contains(&full_path)
                                {
                                    if item.r#type == ScannedItemType::Directory
                                        && self.open_folders.contains(&item.path)
                                    {
                                        if ui
                                            .button("\u{E0C9}")
                                            .on_hover_text("New weave")
                                            .clicked()
                                        {
                                            *self.modal.borrow_mut() = ModalType::CreateWeave(
                                                item.path
                                                    .join(
                                                        [
                                                            "Untitled.",
                                                            VERSIONED_WEAVE_FILE_EXTENSION,
                                                        ]
                                                        .concat(),
                                                    )
                                                    .to_string_lossy()
                                                    .to_string(),
                                            );
                                        }
                                        if ui
                                            .button("\u{E0D9}")
                                            .on_hover_text("New folder")
                                            .clicked()
                                        {
                                            *self.modal.borrow_mut() = ModalType::CreateDirectory(
                                                item.path
                                                    .join("Untitled Folder")
                                                    .to_string_lossy()
                                                    .to_string(),
                                            );
                                        }
                                    }

                                    /*if ui
                                        .button("\u{E225}")
                                        .on_hover_text("Copy item path")
                                        .clicked()
                                    {
                                        ui.output_mut(|o| {
                                            o.commands.push(OutputCommand::CopyText(
                                                self.path
                                                    .join(&item.path)
                                                    .to_string_lossy()
                                                    .to_string(),
                                            ))
                                        });
                                    };*/

                                    if ui
                                        .button("\u{E09E}")
                                        .on_hover_text("Duplicate item")
                                        .clicked()
                                    {
                                        *self.modal.borrow_mut() = ModalType::Copy((
                                            item.path.clone(),
                                            item.path.to_string_lossy().to_string(),
                                        ));
                                    };

                                    if ui.button("\u{E4F0}").on_hover_text("Rename item").clicked()
                                    {
                                        *self.modal.borrow_mut() = ModalType::Rename((
                                            item.path.clone(),
                                            item.path.to_string_lossy().to_string(),
                                        ));
                                    };

                                    if ui.button("\u{E18E}").on_hover_text("Delete item").clicked()
                                    {
                                        *self.modal.borrow_mut() =
                                            ModalType::Delete(item.path.clone());
                                    };
                                };
                            });
                        }
                    });
            });

        let mut modal = self.modal.borrow_mut();
        match &mut modal.deref_mut() {
            ModalType::CreateWeave(path) => {
                if Modal::new("filemanager-create-weave-modal".into())
                    .show(ui.ctx(), |ui| {
                        ui.set_width(280.0);
                        ui.heading("Create Weave");
                        let label = ui.label("Path:");
                        ui.text_edit_singleline(path).labelled_by(label.id);
                        Sides::new().show(
                            ui,
                            |_ui| {},
                            |ui| {
                                if ui.button("Cancel").clicked() {
                                    ui.close();
                                }
                                if ui.button("Save").clicked() {
                                    let path = PathBuf::from(path.clone());
                                    if !self
                                        .open_documents
                                        .borrow()
                                        .contains(&self.path.join(&path))
                                    {
                                        self.create_weave(path);
                                        ui.close();
                                    }
                                }
                            },
                        );
                    })
                    .should_close()
                {
                    *modal = ModalType::None;
                };
            }
            ModalType::CreateDirectory(path) => {
                if Modal::new("filemanager-create-directory-modal".into())
                    .show(ui.ctx(), |ui| {
                        ui.set_width(280.0);
                        ui.heading("Create Folder");
                        let label = ui.label("Path:");
                        ui.text_edit_singleline(path).labelled_by(label.id);
                        Sides::new().show(
                            ui,
                            |_ui| {},
                            |ui| {
                                if ui.button("Cancel").clicked() {
                                    ui.close();
                                }
                                if ui.button("Save").clicked() {
                                    let path = PathBuf::from(path.clone());
                                    if !self
                                        .open_documents
                                        .borrow()
                                        .contains(&self.path.join(&path))
                                    {
                                        self.create_directory(path);
                                        ui.close();
                                    }
                                }
                            },
                        );
                    })
                    .should_close()
                {
                    *modal = ModalType::None;
                };
            }
            ModalType::Rename((from, to)) => {
                if Modal::new("filemanager-rename-item-modal".into())
                    .show(ui.ctx(), |ui| {
                        ui.set_width(280.0);
                        ui.heading("Move or Rename Item");
                        let label = ui.label("New Path:");
                        ui.text_edit_singleline(to).labelled_by(label.id);
                        Sides::new().show(
                            ui,
                            |_ui| {},
                            |ui| {
                                if ui.button("Cancel").clicked() {
                                    ui.close();
                                }
                                if ui.button("Save").clicked() {
                                    let to = PathBuf::from(to.clone());
                                    if from != &to
                                        && !self
                                            .open_documents
                                            .borrow()
                                            .contains(&self.path.join(&to))
                                    {
                                        self.move_item(from.clone(), to);
                                        ui.close();
                                    }
                                }
                            },
                        );
                    })
                    .should_close()
                {
                    *modal = ModalType::None;
                };
            }
            ModalType::Copy((from, to)) => {
                if Modal::new("filemanager-copy-item-modal".into())
                    .show(ui.ctx(), |ui| {
                        ui.set_width(280.0);
                        ui.heading("Duplicate Item");
                        let label = ui.label("New Path:");
                        ui.text_edit_singleline(to).labelled_by(label.id);
                        Sides::new().show(
                            ui,
                            |_ui| {},
                            |ui| {
                                if ui.button("Cancel").clicked() {
                                    ui.close();
                                }
                                if ui.button("Save").clicked() {
                                    let to = PathBuf::from(to.clone());
                                    if from != &to
                                        && !self
                                            .open_documents
                                            .borrow()
                                            .contains(&self.path.join(&to))
                                    {
                                        self.copy_item(from.clone(), to);
                                        ui.close();
                                    }
                                }
                            },
                        );
                    })
                    .should_close()
                {
                    *modal = ModalType::None;
                };
            }
            ModalType::Delete(path) => {
                if Modal::new("filemanager-confirmed-deletion-modal".into())
                    .show(ui.ctx(), |ui| {
                        ui.set_width(280.0);
                        ui.heading("Confirm Deletion");
                        ui.label("The following item will be deleted:");
                        ui.label(path.to_string_lossy());
                        Sides::new().show(
                            ui,
                            |_ui| {},
                            |ui| {
                                if ui.button("Cancel").clicked() {
                                    ui.close();
                                }
                                if ui.button("Confirm").clicked() {
                                    self.remove_item(path.clone());
                                    ui.close();
                                }
                            },
                        );
                    })
                    .should_close()
                {
                    *modal = ModalType::None;
                };
            }
            ModalType::None => {}
        }

        selected_items
    }
    fn update_item_list(&mut self) {
        self.item_list.clear();
        self.build_item_list(self.roots.iter().cloned().collect::<Vec<_>>());
    }
    fn build_item_list(&mut self, items: impl IntoIterator<Item = PathBuf>) {
        let items = items
            .into_iter()
            .filter_map(|p| self.items.get(&p).cloned())
            .collect::<Vec<_>>();

        for item in items {
            if let TreeItem::Other(_) = item {
                continue;
            }

            let lowercase_name = item
                .path()
                .file_name()
                .map(|s| s.to_os_string())
                .unwrap_or_default()
                .to_ascii_lowercase();

            if !(lowercase_name.is_empty()
                || self
                    .ignore_list
                    .contains(lowercase_name.to_string_lossy().as_ref()))
            {
                self.item_list.push(item.clone().into());
            }

            if let TreeItem::Directory(_, children) = &item
                && (self.open_folders.contains(item.path()))
            {
                self.build_item_list(children.iter().cloned().collect::<Vec<_>>());
            }
        }
    }
    fn create_weave(&self, item: PathBuf) {
        let path = self.path.join(item);
        let tx = self.channel.0.clone();

        self.action_threadpool
            .execute(move || match blank_weave_bytes() {
                Ok(bytes) => match fs::write(&path, bytes) {
                    Ok(_) => {
                        let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                            path,
                            r#type: ScannedItemType::File,
                        })));
                    }
                    Err(error) => {
                        let _ = tx.send(Err(error.into()));
                    }
                },
                Err(error) => {
                    let _ = tx.send(Err(error.into()));
                }
            });
    }
    fn create_directory(&self, item: PathBuf) {
        let path = self.path.join(item);
        let tx = self.channel.0.clone();

        self.action_threadpool
            .execute(move || match fs::create_dir(&path) {
                Ok(_) => {
                    let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                        path,
                        r#type: ScannedItemType::Directory,
                    })));
                }
                Err(error) => {
                    let _ = tx.send(Err(error.into()));
                }
            });
    }
    fn move_item(&self, item: PathBuf, to: PathBuf) {
        let from = self.path.join(item);
        let to = self.path.join(to);
        let tx = self.channel.0.clone();
        let stop_scanning = self.stop_scanning.clone();

        self.action_threadpool
            .execute(move || match fs::rename(&from, &to) {
                Ok(_) => {
                    let _ = tx.send(Ok(ItemScanEvent::Delete(from)));
                    match to.metadata() {
                        Ok(metadata) => {
                            let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                                path: to.clone(),
                                r#type: if metadata.is_dir() {
                                    ScannedItemType::Directory
                                } else if metadata.is_file() {
                                    ScannedItemType::File
                                } else {
                                    ScannedItemType::Other
                                },
                            })));
                            if metadata.is_dir() {
                                for entry in WalkDir::new(&to) {
                                    if stop_scanning.load(Ordering::SeqCst) {
                                        break;
                                    }

                                    match entry {
                                        Ok(entry) => {
                                            let filetype = entry.file_type();

                                            let _ =
                                                tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
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
                                            let _ = tx.send(Err(error.into()));
                                        }
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            let _ = tx.send(Err(error.into()));
                        }
                    }
                }
                Err(error) => {
                    let _ = tx.send(Err(error.into()));
                }
            });
    }
    fn copy_item(&self, item: PathBuf, to: PathBuf) {
        let from = self.path.join(item);
        let to = self.path.join(to);
        let tx = self.channel.0.clone();
        let stop_scanning = self.stop_scanning.clone();

        self.action_threadpool
            .execute(move || match from.metadata() {
                Ok(metadata) => {
                    if metadata.is_dir() {
                        match copy_dir_all(&from, &to) {
                            Ok(_) => {
                                let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                                    path: to.clone(),
                                    r#type: if metadata.is_dir() {
                                        ScannedItemType::Directory
                                    } else if metadata.is_file() {
                                        ScannedItemType::File
                                    } else {
                                        ScannedItemType::Other
                                    },
                                })));
                                for entry in WalkDir::new(&to) {
                                    if stop_scanning.load(Ordering::SeqCst) {
                                        break;
                                    }

                                    match entry {
                                        Ok(entry) => {
                                            let filetype = entry.file_type();

                                            let _ =
                                                tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
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
                                            let _ = tx.send(Err(error.into()));
                                        }
                                    }
                                }
                            }
                            Err(error) => {
                                let _ = tx.send(Err(error.into()));
                            }
                        }
                    } else {
                        match fs::copy(&from, &to) {
                            Ok(_) => {
                                let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                                    path: to.clone(),
                                    r#type: if metadata.is_file() {
                                        ScannedItemType::File
                                    } else {
                                        ScannedItemType::Other
                                    },
                                })));
                            }
                            Err(error) => {
                                let _ = tx.send(Err(error.into()));
                            }
                        }
                    }
                }
                Err(error) => {
                    let _ = tx.send(Err(error.into()));
                }
            });
    }
    fn remove_item(&self, item: PathBuf) {
        let path = self.path.join(item);
        let tx = self.channel.0.clone();

        self.action_threadpool
            .execute(move || match trash::delete(&path) {
                Ok(_) => {
                    let _ = tx.send(Ok(ItemScanEvent::Delete(path)));
                }
                Err(error) => {
                    let _ = tx.send(Err(error.into()));
                    match path.metadata() {
                        Ok(metadata) => {
                            if metadata.is_dir() {
                                match fs::remove_dir_all(&path) {
                                    Ok(_) => {
                                        let _ = tx.send(Ok(ItemScanEvent::Delete(path)));
                                    }
                                    Err(error) => {
                                        let _ = tx.send(Err(error.into()));
                                    }
                                }
                            } else {
                                match fs::remove_file(&path) {
                                    Ok(_) => {
                                        let _ = tx.send(Ok(ItemScanEvent::Delete(path)));
                                    }
                                    Err(error) => {
                                        let _ = tx.send(Err(error.into()));
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            let _ = tx.send(Err(error.into()));
                        }
                    }
                }
            });
    }
    fn update_items(&mut self) {
        let mut has_changed = false;

        let settings = self.settings.borrow();

        if settings.documents.location != self.path {
            self.scanned = false;
            has_changed = true;
            self.open_folders.clear();
            let settings_location = settings.documents.location.clone();
            drop(settings);
            self.path = settings_location;
        } else {
            drop(settings);
        }

        let mut toasts = self.toasts.borrow_mut();

        if !self.scanned {
            self.items.clear();
            self.roots.clear();
            self.finished = false;
            self.file_count = 0;
            self.folder_count = 0;
            self.stop_scanning.store(true, Ordering::SeqCst);
            self.scan_threadpool.join();
            while self.channel.1.try_recv().is_ok() {}
            self.stop_scanning.store(false, Ordering::SeqCst);
            let tx = self.channel.0.clone();
            let path = self.path.clone();
            let stop_scanning = self.stop_scanning.clone();
            self.scan_threadpool.execute(move || {
                match fs::exists(&path) {
                    Ok(exists) => {
                        if !exists && let Err(error) = fs::create_dir_all(&path) {
                            let _ = tx.send(Err(error.into()));
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(Err(error.into()));
                    }
                }

                for entry in WalkDir::new(&path) {
                    if stop_scanning.load(Ordering::SeqCst) {
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
                            let _ = tx.send(Err(error.into()));
                        }
                    }
                }

                let _ = tx.send(Ok(ItemScanEvent::Finish));
            });

            self.scanned = true;
        }

        let mut handled_messages: u32 = 0;
        while let Ok(message) = self.channel.1.try_recv() {
            match message {
                Ok(message) => match message {
                    ItemScanEvent::Insert(insert) => {
                        if insert.path.starts_with(&self.path) {
                            let path = insert
                                .path
                                .strip_prefix(&self.path)
                                .map(|p| p.to_path_buf())
                                .unwrap_or_default();

                            if let Some(parent) = path.parent() {
                                if let Some(TreeItem::Directory(_, children)) =
                                    self.items.get_mut(parent)
                                {
                                    children.insert(path.clone());
                                }
                                if parent == PathBuf::default() {
                                    self.roots.insert(path.clone());
                                }
                            } else if path != PathBuf::default() {
                                self.roots.insert(path.clone());
                            }

                            self.items.insert(
                                path.clone(),
                                match insert.r#type {
                                    ScannedItemType::Directory => {
                                        self.folder_count += 1;
                                        TreeItem::Directory(path, BTreeSet::new())
                                    }
                                    ScannedItemType::File => {
                                        self.file_count += 1;
                                        TreeItem::File(path)
                                    }
                                    ScannedItemType::Other => {
                                        self.file_count += 1;
                                        TreeItem::Other(path)
                                    }
                                },
                            );
                            has_changed = true;
                        }
                    }
                    ItemScanEvent::Delete(delete) => {
                        if let Some(item) = self.items.remove(
                            &delete
                                .strip_prefix(&self.path)
                                .map(|p| p.to_path_buf())
                                .unwrap_or_default(),
                        ) {
                            self.roots.remove(item.path());
                            if let Some(parent) = item.path().parent()
                                && let Some(TreeItem::Directory(_, children)) =
                                    self.items.get_mut(parent)
                            {
                                children.remove(item.path());
                            }

                            match item {
                                TreeItem::Directory(_, _) => {
                                    self.folder_count -= 1;
                                }
                                TreeItem::File(_) => {
                                    self.file_count -= 1;
                                }
                                TreeItem::Other(_) => {
                                    self.file_count -= 1;
                                }
                            }
                        }
                        has_changed = true;
                    }
                    ItemScanEvent::Finish => {
                        self.finished = true;
                    }
                },
                Err(error) => {
                    toasts.warning(format!("Filesystem error: {}", error));
                    warn!("Filesystem error: {error:#?}")
                }
            }
            handled_messages += 1;
            if handled_messages > 10000 {
                break;
            }
        }

        if has_changed {
            drop(toasts);
            self.update_item_list();
        }
    }
}

enum ItemScanEvent {
    Insert(ScannedItem),
    Delete(PathBuf),
    Finish,
}

#[derive(Debug, Clone)]
struct ScannedItem {
    path: PathBuf,
    r#type: ScannedItemType,
}

impl From<TreeItem> for ScannedItem {
    fn from(value: TreeItem) -> Self {
        match value {
            TreeItem::Directory(path, _) => ScannedItem {
                path,
                r#type: ScannedItemType::Directory,
            },
            TreeItem::File(path) => ScannedItem {
                path,
                r#type: ScannedItemType::File,
            },
            TreeItem::Other(path) => ScannedItem {
                path,
                r#type: ScannedItemType::Other,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ScannedItemType {
    File,
    Directory,
    Other,
}

enum ModalType {
    CreateWeave(String),
    CreateDirectory(String),
    Rename((PathBuf, String)),
    Copy((PathBuf, String)),
    Delete(PathBuf),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TreeItem {
    Directory(PathBuf, BTreeSet<PathBuf>),
    File(PathBuf),
    Other(PathBuf),
}

impl TreeItem {
    fn path(&self) -> &PathBuf {
        match self {
            Self::Directory(path, _) => path,
            Self::File(path) => path,
            Self::Other(path) => path,
        }
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
