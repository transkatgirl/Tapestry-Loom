use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{self, AtomicBool},
    },
};

use eframe::egui::{Align, Context, Layout, OutputCommand, Panel, Ui};
use log::{debug, error, warn};
use parking_lot::Mutex;
use tapestry_weave::{VersionedWeave, v1::dependent::TapestryWeave};
use tokio::task::{self, JoinHandle};
use ulid::Ulid;

mod disk;
pub mod preload;

use crate::{
    AppShared,
    common::{task::AbortableBlockingTaskHandle, ui::abbreviate_path},
    editor::{
        preload::EditorPreloadHandle,
        shared::disk::{DiskTask, DiskTaskData},
    },
};

pub(super) struct EditorShared {
    pub id: Ulid,
    path: Option<PathBuf>, // TODO: Document loading, create root dir if it doesn't exist

    disk_task: DiskTask,
    disk_task_data: Arc<Mutex<DiskTaskData>>, // Panics on lock
    pub weave: Option<TapestryWeave>,
}

impl EditorShared {
    pub(super) fn new(mut path: Option<PathBuf>, shared: &mut AppShared) -> Self {
        if let Some(unwrapped_path) = &path {
            if shared.open_documents.insert(unwrapped_path.clone()) {
                shared.open_documents_updated = true;
            } else {
                path = None;
            }
        }

        debug!("Created Editor (path = {:?})", &path);

        let mut disk_task = DiskTask::None;
        let disk_task_data = Arc::new(Mutex::new(DiskTaskData::new()));

        if let Some(path) = path.clone() {
            let data = disk_task_data.clone();
            let _runtime = shared.runtime.enter();
            disk_task = DiskTask::read(path, data);
        }

        Self {
            id: Ulid::new(),
            path,
            disk_task,
            disk_task_data,
            weave: None,
        }
    }
    fn from_preload(preload: EditorPreloadHandle, shared: &mut AppShared) -> Self {
        debug!("Created Editor from preload (path = {:?})", &preload.path);

        assert!(shared.open_documents.insert(preload.path.clone()));
        shared.open_documents_updated = true;

        Self {
            id: Ulid::new(),
            path: Some(preload.path),
            disk_task: DiskTask::from(preload.task),
            disk_task_data: preload.task_data,
            weave: None,
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
    pub(super) fn path(&self) -> &Option<PathBuf> {
        &self.path
    }
    pub(super) fn check_close(&mut self, shared: &mut AppShared) -> bool {
        if let Some(weave) = &self.weave {
            weave.is_empty_including_metadata()
        } else {
            true
        }
    }
    pub(super) fn close(&mut self, shared: &mut AppShared) -> bool {
        if let Some(path) = &self.path {
            if let Some(weave) = &self.weave {
                // TODO
            }

            if shared.open_documents.remove(path) {
                shared.open_documents_updated = true;
            };
        }

        true
    }
}
