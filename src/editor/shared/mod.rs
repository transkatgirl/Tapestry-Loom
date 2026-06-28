use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    mem,
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
    common::ui::abbreviate_path,
    editor::{
        preload::EditorPreloadHandle,
        shared::disk::{DiskTask, DiskTaskData, block_until_read, block_until_write},
    },
};

pub(super) struct EditorShared {
    pub id: Ulid,
    path: Option<PathBuf>, // TODO: Document loading, create root dir if it doesn't exist

    disk_task: DiskTask,
    disk_task_data: Arc<Mutex<DiskTaskData>>, // Panics on lock
    pub weave: Option<TapestryWeave>,
    close_ready: bool,
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
        let mut weave = None;

        if let Some(path) = path.clone() {
            let data = disk_task_data.clone();
            let _runtime = shared.runtime.enter();
            disk_task = DiskTask::read(path, data);
        } else {
            weave = Some(TapestryWeave::with_capacity(16384));
        }

        Self {
            id: Ulid::new(),
            path,
            disk_task,
            disk_task_data,
            weave,
            close_ready: false,
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
            close_ready: false,
        }
    }

    pub(super) fn logic(&mut self, _ctx: &Context, shared: &mut AppShared) {
        self.close_ready = false;

        match mem::take(&mut self.disk_task) {
            DiskTask::Read(task) => {
                if task.is_finished() {
                    match block_until_read(&shared.runtime, task) {
                        Ok(weave) => {
                            self.weave = Some(weave);
                        }
                        Err(error) => {
                            shared.toasts.error(error);
                            self.path = None;
                            self.disk_task_data = Arc::new(Mutex::new(DiskTaskData::new()));
                        }
                    }
                } else {
                    self.disk_task = DiskTask::Read(task);
                }
            }
            DiskTask::Write(task) => {
                if task.is_finished() {
                    match block_until_write(&shared.runtime, task) {
                        Ok(()) => {}
                        Err(error) => {
                            shared.toasts.error(error);
                            self.path = None;
                            self.disk_task_data = Arc::new(Mutex::new(DiskTaskData::new()));
                        }
                    }
                } else {
                    self.disk_task = DiskTask::Write(task);
                }
            }
            DiskTask::None => {}
        }
    }
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

    pub(super) fn save(&mut self, shared: &mut AppShared) {
        if let Some(path) = &self.path
            && self.disk_task.is_none()
            && let Some(weave) = &self.weave
        {
            let _runtime = shared.runtime.enter();
            self.disk_task = DiskTask::write(path.clone(), self.disk_task_data.clone(), weave);
            self.close_ready = true;
        }
    }

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
            if weave.is_empty_including_metadata() {
                true
            } else if let Some(path) = self.path.clone() {
                if self.disk_task.is_none() {
                    let _runtime = shared.runtime.enter();
                    self.disk_task =
                        DiskTask::write(path.clone(), self.disk_task_data.clone(), weave);
                    self.close_ready = true;
                }

                true
            } else {
                false
            }
        } else {
            true
        }
    }
    pub(super) fn close(&mut self, shared: &mut AppShared) -> bool {
        if let Some(path) = self.path.clone() {
            if let Some(weave) = &self.weave {
                if let DiskTask::Write(task) = mem::take(&mut self.disk_task) {
                    match block_until_write(&shared.runtime, task) {
                        Ok(()) => {}
                        Err(error) => {
                            shared.toasts.error(error);
                            self.path = None;
                            self.disk_task_data = Arc::new(Mutex::new(DiskTaskData::new()));
                            return false;
                        }
                    }
                } else {
                    debug_assert!(self.disk_task.is_none());
                }

                if !self.close_ready {
                    let _runtime = shared.runtime.enter();

                    if let DiskTask::Write(task) =
                        DiskTask::write(path.clone(), self.disk_task_data.clone(), weave)
                    {
                        match block_until_write(&shared.runtime, task) {
                            Ok(()) => {}
                            Err(error) => {
                                shared.toasts.error(error);
                                self.path = None;
                                self.disk_task_data = Arc::new(Mutex::new(DiskTaskData::new()));
                                return false;
                            }
                        }
                    } else {
                        panic!()
                    }
                }
            }

            if shared.open_documents.remove(&path) {
                shared.open_documents_updated = true;
            };
        }

        debug!("Closed Editor (path = {:?})", &self.path);

        true
    }
}
