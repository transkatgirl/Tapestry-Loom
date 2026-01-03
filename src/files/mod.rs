use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashSet,
    ffi::OsString,
    ops::DerefMut,
    path::{MAIN_SEPARATOR_STR, Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use eframe::egui::{
    Align, Button, Frame, Key, Layout, Modal, OutputCommand, RichText, ScrollArea, Sense, Sides,
    Spinner, TextStyle, TopBottomPanel, Ui, UiBuilder, UiKind, UiStackInfo,
};
use egui_notify::Toasts;
use flagset::FlagSet;
use log::warn;
use tapestry_weave::{VERSIONED_WEAVE_FILE_EXTENSION, treeless::FILE_EXTENSION};
use tokio::runtime::Runtime;
use unicode_segmentation::UnicodeSegmentation;

mod tree;

use crate::{
    editor::blank_weave_bytes,
    files::tree::{FileTreeManager, ScannedItem, ScannedItemType, TreeItem},
    format_large_number_detailed, listing_margin,
    settings::{Settings, shortcuts::Shortcuts},
};

pub struct FileManager {
    //settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
    tree: FileTreeManager,
    open_documents: Rc<RefCell<HashSet<PathBuf>>>,
    item_list: Vec<ScannedItem>,
    open_folders: HashSet<PathBuf>,
    ignore_list: HashSet<&'static str>,
    modal: RefCell<ModalType>,
}

impl FileManager {
    pub fn new(
        settings: Rc<RefCell<Settings>>,
        toasts: Rc<RefCell<Toasts>>,
        runtime: Arc<Runtime>,
        open_documents: Rc<RefCell<HashSet<PathBuf>>>,
    ) -> Self {
        Self {
            tree: FileTreeManager::new(settings, runtime),
            //settings,
            toasts,
            open_documents,
            item_list: Vec::with_capacity(32768),
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
    pub fn refresh(&mut self) {
        self.tree.refresh();
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        shortcuts: FlagSet<Shortcuts>,
        open_callback: impl FnMut(&PathBuf),
    ) {
        self.update_items();

        TopBottomPanel::bottom("filemanager-bottom-panel").show_animated_inside(ui, true, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let contents = self.tree.contents();

                    if !contents.finished {
                        ui.add(Spinner::new());
                    }
                    ui.label(format!(
                        "{}, {}",
                        format_large_number_detailed(*contents.file_count, "file", "files"),
                        format_large_number_detailed(*contents.folder_count, "folder", "folders"),
                    ))
                    .on_hover_text(contents.path.to_string_lossy())
                    .context_menu(|ui| {
                        if ui.button("Copy path").clicked() {
                            ui.output_mut(|o| {
                                o.commands.push(OutputCommand::CopyText(
                                    contents.path.to_string_lossy().to_string(),
                                ))
                            });
                        };
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("\u{E145}").on_hover_text("Refresh").clicked() {
                        self.open_folders.clear();
                        self.tree.refresh();
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

        ui.scope_builder(
            UiBuilder::new()
                .ui_stack_info(UiStackInfo::new(UiKind::CentralPanel))
                .sense(Sense::click()),
            |ui| {
                self.render_items(ui, shortcuts, open_callback);
                self.render_modals(ui);
            },
        );
    }
    fn render_items(
        &mut self,
        ui: &mut Ui,
        _shortcuts: FlagSet<Shortcuts>,
        mut open_callback: impl FnMut(&PathBuf),
    ) {
        let items = self.item_list.clone();

        if items.is_empty() {
            Frame::new()
                .outer_margin(listing_margin(ui))
                .show(ui, |ui| {
                    ui.disable();
                    ui.label("No files found");
                });
        }

        let mut should_full_refresh = false;

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
                        let contents = self.tree.contents();
                        let mut should_refresh = false;

                        for item in &items[range] {
                            let (padding, label) = if let Some(parent) = item.path.parent() {
                                if let Ok(without_prefix) = item.path.strip_prefix(parent) {
                                    let parent_length: usize = UnicodeSegmentation::graphemes(
                                        parent.to_string_lossy().as_ref(),
                                        true,
                                    )
                                    .map(|_| 1)
                                    .sum();
                                    if parent_length > 0 && contents.items.contains_key(parent) {
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

                            let full_path = contents.path.join(&item.path);

                            let (icon, suffix) = match item.r#type {
                                ScannedItemType::File => ("📄", ""),
                                ScannedItemType::Directory => ("📂", MAIN_SEPARATOR_STR),
                                ScannedItemType::Other => ("?", ""),
                            };

                            ui.horizontal(|ui| {
                                ui.scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                                    ui.scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                                        ui.add_space(ch * padding as f32);

                                        let mut button = Button::new(
                                            RichText::new(format!("{icon} {label}{suffix}"))
                                                .family(eframe::egui::FontFamily::Monospace),
                                        );
                                        let mut enabled = true;

                                        if item.r#type == ScannedItemType::File {
                                            if !(item.path.extension() == Some(&file_extension_normal)
                                                || item.path.extension()
                                                    == Some(&file_extension_treeless))
                                                || self.open_documents.borrow().contains(&full_path)
                                            {
                                                enabled = false;
                                            }
                                        } else if self.open_folders.contains(&item.path) {
                                            //button = button.selected(true);
                                            button = button.fill(ui.style().visuals.extreme_bg_color);
                                        }

                                        let button_response = if enabled {
                                            ui.add(button)
                                        } else {
                                            ui.scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                                                ui.add_enabled(enabled, button)
                                            }).response
                                        };

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
                                                        open_callback(&full_path);
                                                    }
                                                    ui.separator();
                                                };

                                                if ui.button("Copy item path").clicked() {
                                                    ui.output_mut(|o| {
                                                        o.commands.push(OutputCommand::CopyText(
                                                            contents
                                                                .path
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
                                                open_callback(&full_path);
                                            } else {
                                                if self.open_folders.contains(&item.path) {
                                                    self.open_folders.remove(&item.path);
                                                } else {
                                                    self.open_folders.insert(item.path.clone());
                                                }
                                                should_refresh = true;
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
                                                    *self.modal.borrow_mut() =
                                                        ModalType::CreateDirectory(
                                                            item.path
                                                                .join("Untitled Folder")
                                                                .to_string_lossy()
                                                                .to_string(),
                                                        );
                                                }
                                            }

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

                                            if ui
                                                .button("\u{E4F0}")
                                                .on_hover_text("Rename item")
                                                .clicked()
                                            {
                                                *self.modal.borrow_mut() = ModalType::Rename((
                                                    item.path.clone(),
                                                    item.path.to_string_lossy().to_string(),
                                                ));
                                            };

                                            if ui
                                                .button("\u{E18E}")
                                                .on_hover_text("Delete item")
                                                .clicked()
                                            {
                                                *self.modal.borrow_mut() =
                                                    ModalType::Delete(item.path.clone());
                                            };
                                        };

                                        ui.add_space(ui.spacing().menu_spacing);
                                    });

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.add_space(0.0);
                                    });

                                    ui.response().context_menu(|ui| {
                                        render_global_context_menu(ui, contents.path, &mut self.modal.borrow_mut(), &mut should_full_refresh);
                                    });
                                });
                            });
                        }

                        if should_refresh {
                            self.update_item_list();
                        }
                    });
            });

        ui.response().context_menu(|ui| {
            render_global_context_menu(
                ui,
                self.tree.contents().path,
                &mut self.modal.borrow_mut(),
                &mut should_full_refresh,
            );
        });

        if should_full_refresh {
            self.open_folders.clear();
            self.tree.refresh();
        }
    }
    pub fn render_modals(&mut self, ui: &mut Ui) {
        let root_path = self.tree.contents().path.clone();

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
                                if ui.button("Apply").clicked()
                                    || ui.input(|input| input.key_pressed(Key::Enter))
                                {
                                    let path = PathBuf::from(path.clone());
                                    if !self
                                        .open_documents
                                        .borrow()
                                        .contains(&root_path.join(&path))
                                        && !self.tree.contents().items.contains_key(&path)
                                    {
                                        self.tree.create_file(
                                            path,
                                            blank_weave_bytes().unwrap(),
                                            true,
                                        );
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
                                if ui.button("Apply").clicked()
                                    || ui.input(|input| input.key_pressed(Key::Enter))
                                {
                                    let path = PathBuf::from(path.clone());
                                    if !self
                                        .open_documents
                                        .borrow()
                                        .contains(&root_path.join(&path))
                                        && !self.tree.contents().items.contains_key(&path)
                                    {
                                        self.tree.create_directory(path);
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
                                if ui.button("Apply").clicked()
                                    || ui.input(|input| input.key_pressed(Key::Enter))
                                {
                                    let to = PathBuf::from(to.clone());
                                    if from != &to
                                        && !self
                                            .open_documents
                                            .borrow()
                                            .contains(&root_path.join(&to))
                                        && !self.tree.contents().items.contains_key(&to)
                                    {
                                        self.tree.move_item(from.clone(), to, true);
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
                                if ui.button("Apply").clicked()
                                    || ui.input(|input| input.key_pressed(Key::Enter))
                                {
                                    let to = PathBuf::from(to.clone());
                                    if from != &to
                                        && !self
                                            .open_documents
                                            .borrow()
                                            .contains(&root_path.join(&to))
                                        && !self.tree.contents().items.contains_key(&to)
                                    {
                                        self.tree.copy_item(from.clone(), to, true);
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
                if Modal::new("filemanager-confirm-deletion-modal".into())
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
                                if ui.button("Confirm").clicked()
                                    || ui.input(|input| input.key_pressed(Key::Enter))
                                {
                                    self.tree.remove_item(path.clone());
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
    }
    fn update_item_list(&mut self) {
        self.item_list.clear();
        self.build_item_list(
            self.tree
                .contents()
                .roots
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
        );
    }
    fn build_item_list(&mut self, items: impl IntoIterator<Item = PathBuf>) {
        let contents = self.tree.contents();

        let items = items
            .into_iter()
            .filter_map(|p| contents.items.get(&p).cloned())
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
    fn update_items(&mut self) {
        let mut toasts = self.toasts.borrow_mut();

        let has_changed = self.tree.update_items(
            |error| {
                toasts.warning(format!("Filesystem error: {}", error));
                warn!("Filesystem error: {error:#?}")
            },
            10000,
        );

        if has_changed {
            drop(toasts);
            self.update_item_list();
        }
    }
}

enum ModalType {
    CreateWeave(String),
    CreateDirectory(String),
    Rename((PathBuf, String)),
    Copy((PathBuf, String)),
    Delete(PathBuf),
    None,
}

fn render_global_context_menu(ui: &mut Ui, path: &Path, modal: &mut ModalType, refresh: &mut bool) {
    if ui.button("New weave").clicked() {
        *modal = ModalType::CreateWeave(["Untitled.", VERSIONED_WEAVE_FILE_EXTENSION].concat());
    }
    if ui.button("New folder").clicked() {
        *modal = ModalType::CreateDirectory("Untitled Folder".to_string());
    }

    ui.separator();

    if ui.button("Copy path").clicked() {
        ui.output_mut(|o| {
            o.commands
                .push(OutputCommand::CopyText(path.to_string_lossy().to_string()))
        });
    };
    if ui.button("Refresh").clicked() {
        *refresh = true;
    };
}
