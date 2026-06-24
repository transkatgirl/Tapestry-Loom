use std::path::PathBuf;

use eframe::egui::{Align, Context, Layout, OutputCommand, Panel, Ui};
use ulid::Ulid;

use crate::{AppShared, shared::ui::abbreviate_path};

pub(super) struct EditorShared {
    pub id: Ulid,
    path: Option<PathBuf>,
}

impl EditorShared {
    pub(super) fn new(mut path: Option<PathBuf>, shared: &mut AppShared) -> Self {
        if let Some(unwrapped_path) = &path
            && !shared.open_documents.insert(unwrapped_path.clone())
        {
            path = None;
        }

        Self {
            id: Ulid::new(),
            path,
        }
    }
    pub(super) fn logic(&mut self, _ctx: &Context, shared: &mut AppShared) {}
    pub(super) fn modals(&mut self, ctx: &Context, shared: &mut AppShared) -> bool {
        false
    }
    pub(super) fn ui(&mut self, ui: &mut Ui, shared: &mut AppShared) {
        Panel::bottom(ui.id()).show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    /*ui.add(Spinner::new());
                    ui.label("Loading weave...");*/

                    if let Some(path) = &self.path {
                        ui.label(
                            abbreviate_path(&shared.settings.documents.location, path)
                                .to_string_lossy(),
                        )
                        .on_hover_text(path.to_string_lossy())
                        .context_menu(|ui| {
                            if ui.button("Copy path").clicked() {
                                ui.output_mut(|o| {
                                    o.commands.push(OutputCommand::CopyText(
                                        path.to_string_lossy().to_string(),
                                    ))
                                });
                            };
                        });
                    } /*else if ui.button("Save as...").clicked() {
                    // TODO
                    }*/
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // TODO
                });
            });
        });
    }

    pub(super) fn save(&mut self, shared: &mut AppShared) {}

    pub(super) fn title(&self, shared: &AppShared) -> String {
        match &self.path {
            Some(path) => {
                if let Some(filename) = path.file_stem() {
                    filename.to_string_lossy().to_string()
                } else {
                    "Untitled Weave".to_string()
                }
            }
            None => "New Weave".to_string(),
        }
    }
    pub(super) fn check_close(&mut self, shared: &mut AppShared) -> bool {
        true
    }
    pub(super) fn close(&mut self, shared: &mut AppShared) -> bool {
        if let Some(path) = &self.path {
            shared.open_documents.remove(path);
        }

        true
    }
}
