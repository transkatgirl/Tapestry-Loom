// TODO: Improve this file manager implementation (fs watching, incremental updating, drag-and-drop, etc) and then turn it into it's own crate

use std::{
    borrow::Cow,
    collections::HashSet,
    ffi::OsString,
    ops::Range,
    path::{MAIN_SEPARATOR_STR, PathBuf},
};

use eframe::egui::{
    Align, Button, Context, Frame, Id, Key, Layout, Modal, OutputCommand, Panel, RichText,
    ScrollArea, Sense, Sides, Spinner, TextStyle, Ui, UiBuilder, UiKind, UiStackInfo, WidgetText,
};
use tapestry_weave::{
    VERSIONED_WEAVE_FILE_EXTENSION,
    v1::{dependent::TapestryWeave, treeless::FILE_EXTENSION},
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    AppShared,
    files::{
        background::BackgroundFsManager,
        tree::{FileTree, FileTreeState, TreeItem},
    },
    shared::{
        task::BACKGROUND_REFRESH_INTERVAL,
        ui::{abbreviate_path, clicked_rising_edge, format_large_number_detailed, listing_margin},
        view::View,
    },
};

mod background;
mod tree;

#[derive(Default, Debug)]
pub struct FileManager {
    background: BackgroundFsManager,
    tree: FileTree,
    modal: FileModal,

    opened: HashSet<PathBuf>,
    opened_changed: bool,
    displayed: Vec<(PathBuf, FileType)>,
    finished: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileType {
    Directory,
    File,
    Other,
}

impl View<AppShared> for FileManager {
    fn title(&self, _shared: &AppShared) -> WidgetText {
        WidgetText::Text("\u{E33C} Files".to_string())
    }
    fn logic(&mut self, shared: &mut AppShared, ctx: &Context) {
        self.background.update(shared);

        if self.tree.update(&mut self.background) {
            ctx.request_repaint_after(BACKGROUND_REFRESH_INTERVAL);
            self.finished = false;
        } else {
            self.finished = true
        }

        let tree = self.tree.view();

        if tree.updated || self.opened_changed || shared.open_documents_updated {
            if tree.root_changed {
                self.opened.clear();
            }

            self.displayed.clear();
            update_displayed(
                &tree,
                &self.opened,
                &mut self.displayed,
                tree.roots.iter().cloned(),
            );

            self.opened_changed = false;
            shared.open_documents_updated = false;
        }
    }
    fn modals(&mut self, shared: &mut AppShared, ctx: &Context) -> bool {
        self.modal.ui(&mut self.background, shared, ctx);
        self.modal != FileModal::default()
    }
    fn ui(&mut self, shared: &mut AppShared, ui: &mut Ui) {
        Panel::bottom("filemanager-bottom-panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let tree = self.tree.view();

                    if !self.finished {
                        ui.add(Spinner::new());
                    }
                    ui.label(format!(
                        "{}, {}",
                        format_large_number_detailed(tree.file_count, "file", "files"),
                        format_large_number_detailed(tree.directory_count, "folder", "folders"),
                    ))
                    .on_hover_text(shared.settings.documents.location.to_string_lossy())
                    .context_menu(|ui| {
                        if ui.button("Copy path").clicked() {
                            ui.output_mut(|o| {
                                o.commands.push(OutputCommand::CopyText(
                                    shared
                                        .settings
                                        .documents
                                        .location
                                        .to_string_lossy()
                                        .to_string(),
                                ))
                            });
                        };
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("\u{E145}").on_hover_text("Refresh").clicked() {
                        self.background.refresh();
                        ui.request_repaint();
                    }
                    if ui.button("\u{E0D9}").on_hover_text("New folder").clicked() {
                        self.modal = FileModal::CreateDirectory("Untitled Folder".to_string());
                        ui.request_repaint();
                    }
                    if ui.button("\u{E0C9}").on_hover_text("New weave").clicked() {
                        self.modal = FileModal::CreateWeave(
                            ["Untitled.", VERSIONED_WEAVE_FILE_EXTENSION].concat(),
                        );
                        ui.request_repaint();
                    }
                });
            });
        });

        ui.scope_builder(
            UiBuilder::new()
                .id(Id::new("filemanager-central-panel"))
                .ui_stack_info(UiStackInfo::new(UiKind::CentralPanel))
                .sense(Sense::CLICK),
            |ui| {
                if self.displayed.is_empty() {
                    Frame::new()
                        .outer_margin(listing_margin(ui))
                        .show(ui, |ui| {
                            ui.disable();
                            ui.label("No files found");
                        });
                    return;
                }

                ScrollArea::vertical()
                    .auto_shrink(false)
                    .animated(false)
                    .show_rows(
                        ui,
                        ui.spacing().interact_size.y,
                        self.displayed.len(),
                        |ui, range| {
                            ui.scope_builder(UiBuilder::new().sense(Sense::CLICK), |ui| {
                                Frame::new()
                                    .outer_margin(listing_margin(ui))
                                    .show(ui, |ui| {
                                        self.file_listing(shared, ui, range);
                                    });

                                ui.response().context_menu(|ui| {
                                    self.global_context_menu(shared, ui);
                                });
                            });
                        },
                    );

                ui.response().context_menu(|ui| {
                    self.global_context_menu(shared, ui);
                });
            },
        );
    }
}

fn global_context_menu(
    modal: &mut FileModal,
    background: &mut BackgroundFsManager,
    shared: &mut AppShared,
    ui: &mut Ui,
) {
    if ui.button("New weave").clicked() {
        *modal = FileModal::CreateWeave(["Untitled.", VERSIONED_WEAVE_FILE_EXTENSION].concat());
        ui.request_repaint();
    }
    if ui.button("New folder").clicked() {
        *modal = FileModal::CreateDirectory("Untitled Folder".to_string());
        ui.request_repaint();
    }

    ui.separator();

    if ui.button("Copy path").clicked() {
        ui.output_mut(|o| {
            o.commands.push(OutputCommand::CopyText(
                shared
                    .settings
                    .documents
                    .location
                    .to_string_lossy()
                    .to_string(),
            ))
        });
    };
    if ui.button("Refresh").clicked() {
        background.refresh();
        ui.request_repaint();
    };
}

impl FileManager {
    fn global_context_menu(&mut self, shared: &mut AppShared, ui: &mut Ui) {
        global_context_menu(&mut self.modal, &mut self.background, shared, ui);
    }
    fn file_listing(&mut self, shared: &mut AppShared, ui: &mut Ui, range: Range<usize>) {
        let text_style = TextStyle::Monospace;
        let ch = ui.fonts_mut(|f| f.glyph_width(&text_style.resolve(ui.style()), ' '));
        let file_extension_normal = OsString::from(VERSIONED_WEAVE_FILE_EXTENSION);
        let file_extension_treeless = OsString::from(FILE_EXTENSION);

        for (path, item_type) in &self.displayed[range] {
            let item_type = *item_type;
            let abbreviated_path = abbreviate_path(&shared.settings.documents.location, path);

            let (padding, label) = if let Some(parent) = abbreviated_path.parent()
                && let Ok(without_prefix) = abbreviated_path.strip_prefix(parent)
            {
                let parent_length: usize =
                    UnicodeSegmentation::graphemes(parent.to_string_lossy().as_ref(), true)
                        .map(|_| 1)
                        .sum();

                if parent_length > 0
                    && path
                        .parent()
                        .map(|parent| {
                            self.opened
                                .contains(&shared.settings.documents.location.join(parent))
                        })
                        .unwrap_or_default()
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
                    (0, abbreviated_path.to_string_lossy())
                }
            } else {
                (0, abbreviated_path.to_string_lossy())
            };

            let (icon, suffix) = match item_type {
                FileType::Directory => ("📂", MAIN_SEPARATOR_STR),
                FileType::File => ("📄", ""),
                FileType::Other => ("❔", ""),
            };

            let mut spacing = ch * padding as f32;

            let menu_spacing = if spacing >= ui.spacing().menu_spacing {
                spacing -= ui.spacing().menu_spacing;
                true
            } else {
                false
            };

            ui.scope_builder(
                UiBuilder::new().id(Id::new(["filemanager-item-", &path.to_string_lossy()])),
                |ui| {
                    ui.horizontal(|ui| {
                        ui.scope_builder(UiBuilder::new().sense(Sense::CLICK), |ui| {
                            ui.add_space(spacing);

                            ui.scope_builder(UiBuilder::new().sense(Sense::CLICK), |ui| {
                                if menu_spacing {
                                    ui.add_space(ui.spacing().menu_spacing);
                                }

                                let mut button = Button::new(
                                    RichText::new(format!("{icon} {label}{suffix}"))
                                        .family(eframe::egui::FontFamily::Monospace),
                                );
                                let mut enabled = item_type != FileType::Other;

                                if item_type == FileType::File {
                                    if !(path.extension() == Some(&file_extension_normal)
                                        || path.extension() == Some(&file_extension_treeless))
                                        || shared.open_documents.contains(path)
                                    {
                                        enabled = false;
                                    }
                                } else if self.opened.contains(path) {
                                    //button = button.selected(true);
                                    button = button.fill(ui.style().visuals.extreme_bg_color);
                                }

                                let button_response = if enabled {
                                    ui.add(button)
                                } else {
                                    ui.scope_builder(UiBuilder::new().sense(Sense::CLICK), |ui| {
                                        ui.add_enabled(enabled, button)
                                    })
                                    .response
                                };

                                if !shared.open_documents.contains(path) {
                                    button_response.context_menu(|ui| {
                                        if item_type == FileType::Directory {
                                            if ui.button("New weave").clicked() {
                                                self.modal = FileModal::CreateWeave(
                                                    abbreviated_path
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
                                                ui.request_repaint();
                                            }
                                            if ui.button("New folder").clicked() {
                                                self.modal = FileModal::CreateDirectory(
                                                    abbreviated_path
                                                        .join("Untitled Folder")
                                                        .to_string_lossy()
                                                        .to_string(),
                                                );
                                                ui.request_repaint();
                                            }
                                            ui.separator();
                                        } else if item_type == FileType::File
                                            && (path.extension() == Some(&file_extension_normal)
                                                || path.extension()
                                                    == Some(&file_extension_treeless))
                                        {
                                            if ui.button("Open weave").clicked() {
                                                shared.load_document_queue.push(path.clone());
                                                ui.request_repaint();
                                            }
                                            ui.separator();
                                        };

                                        if ui.button("Copy item path").clicked() {
                                            ui.output_mut(|o| {
                                                o.commands.push(OutputCommand::CopyText(
                                                    path.to_string_lossy().to_string(),
                                                ))
                                            });
                                        };

                                        ui.separator();

                                        if item_type != FileType::Other
                                            && ui.button("Duplicate item").clicked()
                                        {
                                            self.modal = FileModal::Copy(
                                                path.clone(),
                                                abbreviated_path.to_string_lossy().to_string(),
                                            );
                                            ui.request_repaint();
                                        }

                                        if ui.button("Rename item").clicked() {
                                            self.modal = FileModal::Rename(
                                                path.clone(),
                                                abbreviated_path.to_string_lossy().to_string(),
                                            );
                                            ui.request_repaint();
                                        };

                                        if ui.button("Delete item").clicked() {
                                            self.modal = FileModal::Delete(path.clone());
                                            ui.request_repaint();
                                        };
                                    });
                                }

                                if enabled && clicked_rising_edge(&button_response) {
                                    if item_type == FileType::File {
                                        shared.load_document_queue.push(path.clone());
                                        ui.request_repaint();
                                    } else {
                                        if self.opened.contains(path) {
                                            self.opened.remove(path);
                                        } else {
                                            self.opened.insert(path.clone());
                                        }
                                        ui.request_discard("Updated listing");
                                        self.opened_changed = true;
                                        return;
                                    }
                                };

                                if ui.rect_contains_pointer(ui.max_rect())
                                    && !shared.open_documents.contains(path)
                                {
                                    if item_type == FileType::Directory
                                        && self.opened.contains(path)
                                    {
                                        if ui
                                            .button("\u{E0C9}")
                                            .on_hover_text("New weave")
                                            .clicked()
                                        {
                                            self.modal = FileModal::CreateWeave(
                                                abbreviated_path
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
                                            ui.request_repaint();
                                        }
                                        if ui
                                            .button("\u{E0D9}")
                                            .on_hover_text("New folder")
                                            .clicked()
                                        {
                                            self.modal = FileModal::CreateDirectory(
                                                abbreviated_path
                                                    .join("Untitled Folder")
                                                    .to_string_lossy()
                                                    .to_string(),
                                            );
                                            ui.request_repaint();
                                        }
                                    }

                                    if item_type != FileType::Other
                                        && ui
                                            .button("\u{E09E}")
                                            .on_hover_text("Duplicate item")
                                            .clicked()
                                    {
                                        self.modal = FileModal::Copy(
                                            path.clone(),
                                            abbreviated_path.to_string_lossy().to_string(),
                                        );
                                        ui.request_repaint();
                                    };

                                    if ui.button("\u{E4F0}").on_hover_text("Rename item").clicked()
                                    {
                                        self.modal = FileModal::Rename(
                                            path.clone(),
                                            abbreviated_path.to_string_lossy().to_string(),
                                        );
                                        ui.request_repaint();
                                    };

                                    if ui.button("\u{E18E}").on_hover_text("Delete item").clicked()
                                    {
                                        self.modal = FileModal::Delete(path.clone());
                                        ui.request_repaint();
                                    };
                                }

                                ui.add_space(ui.spacing().menu_spacing);
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add_space(0.0);
                            });

                            ui.response().context_menu(|ui| {
                                global_context_menu(
                                    &mut self.modal,
                                    &mut self.background,
                                    shared,
                                    ui,
                                );
                            });
                        });
                    });
                },
            );
        }
    }
}

fn update_displayed(
    tree: &FileTreeState<'_>,
    opened: &HashSet<PathBuf>,
    displayed: &mut Vec<(PathBuf, FileType)>,
    roots: impl IntoIterator<Item = PathBuf>,
) {
    for item in roots {
        match tree.items.get(&item) {
            Some(TreeItem::Directory(children)) => {
                let absolute_path = tree.relative_to.join(item);

                displayed.push((absolute_path.clone(), FileType::Directory));

                if opened.contains(&absolute_path) {
                    update_displayed(tree, opened, displayed, children.iter().cloned());
                }
            }
            Some(TreeItem::File) => displayed.push((tree.relative_to.join(item), FileType::File)),
            Some(_) => displayed.push((tree.relative_to.join(item), FileType::Other)),
            None => panic!(),
        };
    }
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum FileModal {
    #[default]
    None,
    CreateWeave(String),
    CreateDirectory(String),
    Rename(PathBuf, String),
    Copy(PathBuf, String),
    Delete(PathBuf),
}

fn blank_document_bytes() -> Vec<u8> {
    let mut container = Vec::with_capacity(4096);
    TapestryWeave::with_capacity(0)
        .write_versioned_bytes(&mut container)
        .unwrap();
    debug_assert!(container.len() <= container.capacity());
    container.shrink_to_fit();
    container
}

impl FileModal {
    fn ui(&mut self, background: &mut BackgroundFsManager, shared: &mut AppShared, ctx: &Context) {
        match self {
            Self::None => {}
            Self::CreateWeave(path) => {
                if Modal::new("filemanager-create-document-modal".into())
                    .show(ctx, |ui| {
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
                                    let path = shared
                                        .settings
                                        .documents
                                        .location
                                        .join(PathBuf::from(path.clone()));

                                    if !shared.open_documents.contains(&path)
                                        && !background.likely_exists(&path)
                                    {
                                        background.create_file(shared, path, blank_document_bytes);
                                        ui.request_repaint();
                                        ui.close();
                                    }
                                }
                            },
                        );
                    })
                    .should_close()
                {
                    *self = FileModal::None;
                };
            }
            Self::CreateDirectory(path) => {
                if Modal::new("filemanager-create-directory-modal".into())
                    .show(ctx, |ui| {
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
                                    let path = shared
                                        .settings
                                        .documents
                                        .location
                                        .join(PathBuf::from(path.clone()));

                                    if !shared.open_documents.contains(&path)
                                        && !background.likely_exists(&path)
                                    {
                                        background.create_directory(shared, path);
                                        ui.request_repaint();
                                        ui.close();
                                    }
                                }
                            },
                        );
                    })
                    .should_close()
                {
                    *self = FileModal::None;
                };
            }
            Self::Rename(from, to) => {
                if Modal::new("filemanager-rename-item-modal".into())
                    .show(ctx, |ui| {
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
                                    let to = shared
                                        .settings
                                        .documents
                                        .location
                                        .join(PathBuf::from(to.clone()));

                                    if from != &to
                                        && !shared.open_documents.contains(from)
                                        && !shared.open_documents.contains(&to)
                                        && !background.likely_exists(&to)
                                    {
                                        background.rename_item(shared, from.clone(), to);
                                        ui.request_repaint();
                                        ui.close();
                                    }
                                }
                            },
                        );
                    })
                    .should_close()
                {
                    *self = FileModal::None;
                };
            }
            Self::Copy(from, to) => {
                if Modal::new("filemanager-copy-item-modal".into())
                    .show(ctx, |ui| {
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
                                    let to = shared
                                        .settings
                                        .documents
                                        .location
                                        .join(PathBuf::from(to.clone()));

                                    if from != &to
                                        && !shared.open_documents.contains(from)
                                        && !shared.open_documents.contains(&to)
                                        && !background.likely_exists(&to)
                                    {
                                        background.copy_item(shared, from.clone(), to);
                                        ui.request_repaint();
                                        ui.close();
                                    }
                                }
                            },
                        );
                    })
                    .should_close()
                {
                    *self = FileModal::None;
                };
            }
            Self::Delete(path) => {
                if Modal::new("filemanager-confirm-deletion-modal".into())
                    .show(ctx, |ui| {
                        ui.set_width(280.0);
                        ui.heading("Confirm Deletion");
                        ui.label("The following item will be deleted:");
                        ui.label(
                            abbreviate_path(&shared.settings.documents.location, path)
                                .to_string_lossy(),
                        );
                        Sides::new().show(
                            ui,
                            |_ui| {},
                            |ui| {
                                if ui.button("Cancel").clicked() {
                                    ui.close();
                                }
                                if (ui.button("Confirm").clicked()
                                    || ui.input(|input| input.key_pressed(Key::Enter)))
                                    && !shared.open_documents.contains(path)
                                {
                                    background.remove_item(shared, path.clone());
                                    ui.request_repaint();
                                    ui.close();
                                }
                            },
                        );
                    })
                    .should_close()
                {
                    *self = FileModal::None;
                };
            }
        }
    }
}
