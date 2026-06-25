use std::path::PathBuf;

use eframe::egui::{Context, Key, Modal, Sides, Ui, WidgetText};

use crate::{
    AppShared,
    files::background::BackgroundFsManager,
    shared::{task::BACKGROUND_REFRESH_WAIT, ui::abbreviate_path, view::View},
};

mod background;

#[derive(Default, Debug)]
pub struct FileManager {
    background: BackgroundFsManager,
    modal: FileModal,
}

impl View<AppShared> for FileManager {
    fn title(&self, _shared: &AppShared) -> WidgetText {
        WidgetText::Text("\u{E33C} Files".to_string())
    }
    fn logic(&mut self, shared: &mut AppShared, ctx: &Context) {
        self.background.update(shared);
        self.background.read_cached(|id, root, files, finished| {
            if !finished {
                ctx.request_repaint_after(BACKGROUND_REFRESH_WAIT);
            }
        })

        // TODO
    }
    fn modals(&mut self, shared: &mut AppShared, ctx: &Context) -> bool {
        self.modal.ui(&mut self.background, shared, ctx);
        self.modal != FileModal::default()
    }
    fn ui(&mut self, shared: &mut AppShared, ui: &mut Ui) {
        // TODO
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
    todo!()
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
                                        background.rename_item(shared, from.to_path_buf(), to);
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
                                        background.copy_item(shared, from.to_path_buf(), to);
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
                                    background.remove_item(shared, path.to_path_buf());
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
