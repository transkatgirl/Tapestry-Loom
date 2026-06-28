use std::{fs, mem, path::PathBuf, sync::Arc};

use eframe::egui::{Align, Context, Key, Layout, Modal, OutputCommand, Panel, Sides, Spinner, Ui};
use log::debug;
use parking_lot::Mutex;
use tapestry_weave::{VERSIONED_WEAVE_FILE_EXTENSION, v1::dependent::TapestryWeave};
use ulid::Ulid;

mod disk;
pub(super) mod inference;
pub mod preload;

use crate::{
    AppShared,
    common::{
        task::BACKGROUND_REFRESH_INTERVAL,
        ui::{
            abbreviate_path, after_ui_interaction, format_file_size, format_large_number,
            format_large_number_detailed,
        },
    },
    editor::{
        preload::EditorPreloadHandle,
        shared::{
            disk::{DiskTask, DiskTaskData, block_until_read, block_until_write},
            inference::InferenceEngine,
        },
    },
};

pub(super) struct EditorShared {
    pub id: Ulid,
    path: Option<PathBuf>,
    modal: EditorModal,

    disk_task: DiskTask,
    disk_task_data: Arc<Mutex<DiskTaskData>>, // Panics if more than one lock is held at a time
    pub weave: Option<TapestryWeave>,
    close_ready: bool,
    close_now: bool,
    close_after_save: bool,

    pub inference: InferenceEngine,
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
            let _runtime = shared.runtime.enter();
            disk_task = DiskTask::read(path, disk_task_data.clone());
        } else {
            weave = Some(TapestryWeave::with_capacity(16384));
        }

        Self {
            id: Ulid::new(),
            path,
            modal: EditorModal::None,
            disk_task,
            disk_task_data,
            weave,
            close_ready: false,
            close_now: false,
            close_after_save: false,
            inference: InferenceEngine::new(shared),
        }
    }
    fn from_preload(preload: EditorPreloadHandle, shared: &mut AppShared) -> Self {
        debug!("Created Editor from preload (path = {:?})", &preload.path);

        assert!(shared.open_documents.insert(preload.path.clone()));
        shared.open_documents_updated = true;

        Self {
            id: Ulid::new(),
            path: Some(preload.path),
            modal: EditorModal::None,
            disk_task: DiskTask::from(preload.task),
            disk_task_data: preload.task_data,
            weave: None,
            close_ready: false,
            close_now: false,
            close_after_save: false,
            inference: InferenceEngine::new(shared),
        }
    }
    fn clear_path(&mut self, shared: &mut AppShared) {
        if let Some(path) = &self.path
            && shared.open_documents.remove(path)
        {
            shared.open_documents_updated = true;
        }
        self.path = None;
        self.disk_task = DiskTask::None;
        self.disk_task_data = Arc::new(Mutex::new(DiskTaskData::new()));
        self.close_ready = false;
        self.close_after_save = false;
    }
    fn prepare_for_close(&mut self, shared: &mut AppShared) {
        if let Some(path) = &self.path
            && shared.open_documents.remove(path)
        {
            shared.open_documents_updated = true;
        }

        self.inference.cancel(shared);

        debug!("Closed Editor (path = {:?})", &self.path);
    }
    pub(super) fn logic(
        &mut self,
        ctx: &Context,
        force_close: impl FnOnce(),
        shared: &mut AppShared,
    ) {
        if self.close_now {
            self.prepare_for_close(shared);
            force_close();
            return;
        }

        match mem::take(&mut self.disk_task) {
            DiskTask::Read(task) => {
                if task.is_finished() {
                    match block_until_read(&shared.runtime, task) {
                        Ok(weave) => {
                            self.weave = Some(weave);
                        }
                        Err(error) => {
                            shared.toasts.error(error);
                            self.clear_path(shared);
                        }
                    }
                } else {
                    self.disk_task = DiskTask::Read(task);
                }
            }
            DiskTask::Write(task) => {
                if task.is_finished() {
                    match block_until_write(&shared.runtime, task) {
                        Ok(()) => {
                            if self.close_after_save {
                                self.prepare_for_close(shared);
                                force_close();
                                return;
                            }
                        }
                        Err(error) => {
                            shared.toasts.error(error);
                            self.clear_path(shared);
                        }
                    }
                } else {
                    self.disk_task = DiskTask::Write(task);
                }
            }
            DiskTask::None => {}
        }

        self.inference.update(shared, &mut self.weave);

        if !self.disk_task.is_none() || self.inference.requests() > 0 {
            ctx.request_repaint_after(BACKGROUND_REFRESH_INTERVAL);
        }
    }
    pub(super) fn modals(&mut self, ctx: &Context, shared: &mut AppShared) -> bool {
        match &mut self.modal {
            EditorModal::None => false,
            EditorModal::SaveAs(path) => {
                if Modal::new(
                    ["editor-", &self.id.to_string(), "-save-modal"]
                        .concat()
                        .into(),
                )
                .show(ctx, |ui| {
                    ui.set_width(280.0);
                    ui.heading("Save Weave");
                    let label = ui.label("Path:");
                    ui.text_edit_singleline(path).labelled_by(label.id);
                    Sides::new().show(
                        ui,
                        |_ui| {},
                        |ui| {
                            if ui.button("Cancel").clicked() {
                                ui.close();
                            }
                            if (ui.button("Save").clicked()
                                || ui.input(|input| input.key_pressed(Key::Enter)))
                                && !path.is_empty()
                            {
                                let mut new_path = shared.settings.documents.location.join(path);
                                if new_path.extension().is_none() {
                                    new_path.set_extension("tapestry");
                                }
                                if !shared.open_documents.contains(&new_path)
                                    && !fs::exists(&new_path).unwrap_or(true)
                                // fs::exists() can be done on the UI thread, as nothing outside of the modal can be interacted with regardless
                                {
                                    let _runtime = shared.runtime.enter();
                                    self.disk_task = DiskTask::write(
                                        new_path.clone(),
                                        self.disk_task_data.clone(),
                                        self.weave.as_ref().unwrap(),
                                    );
                                    self.path = Some(new_path.clone());

                                    shared.open_documents.insert(new_path);
                                    shared.open_documents_updated = true;
                                    shared.fs_needs_refresh = true;

                                    ui.close();
                                }
                            }
                        },
                    );
                })
                .should_close()
                {
                    self.modal = EditorModal::None;
                    after_ui_interaction(ctx);
                }

                true
            }
            EditorModal::ConfirmClose => {
                if Modal::new(
                    ["editor-", &self.id.to_string(), "-close-modal"]
                        .concat()
                        .into(),
                )
                .show(ctx, |ui| {
                    ui.set_width(210.0);
                    ui.heading("Do you want to close this weave without saving?");
                    ui.label("All changes made will be lost.");
                    ui.add_space(ui.style().spacing.menu_spacing);
                    Sides::new().show(
                        ui,
                        |_ui| {},
                        |ui| {
                            if ui.button("Yes").clicked() {
                                self.close_now = true;
                                ui.close();
                            }
                            if ui.button("No").clicked() {
                                ui.close();
                            }
                        },
                    );
                })
                .should_close()
                {
                    self.modal = EditorModal::None;
                    after_ui_interaction(ctx);
                }

                true
            }
        }
    }
    pub(super) fn ui(&mut self, ui: &mut Ui, shared: &mut AppShared) {
        self.close_ready = self.close_after_save;

        Panel::bottom(ui.id()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if let Some(path) = &self.path {
                        if self.weave.is_none() {
                            ui.add(Spinner::new());
                            ui.label("Loading weave...");
                        } else if self.close_after_save {
                            ui.add(Spinner::new());
                            ui.label("Saving weave...");
                        } else {
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
                                    after_ui_interaction(ui);
                                };
                            });
                        }
                    } else if ui.button("Save as...").clicked() {
                        self.modal = EditorModal::SaveAs(
                            ["Untitled.", VERSIONED_WEAVE_FILE_EXTENSION].concat(),
                        );
                        after_ui_interaction(ui);
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(weave) = &self.weave {
                        let requests = self.inference.requests();

                        if requests > 0 {
                            ui.add(Spinner::new());
                            ui.label(format_large_number_detailed(
                                requests, "request", "requests",
                            ))
                            .on_hover_ui(|ui| {
                                if ui.button("Cancel requests").clicked() {
                                    self.inference.cancel(shared);
                                    after_ui_interaction(ui);
                                }
                            });
                        } else {
                            let node_count = weave.len();
                            let active_node_count = weave.get_active_thread_ids().len();
                            let bookmarked_node_count = weave.bookmarks().len();
                            let label = ui.label(if bookmarked_node_count > 0 {
                                format!(
                                    "{}, {}, {}",
                                    format_large_number(node_count, "node", "nodes"),
                                    format_large_number(active_node_count, "active", "active"),
                                    format_large_number(
                                        bookmarked_node_count,
                                        "bookmarked",
                                        "bookmarked"
                                    ),
                                )
                            } else {
                                format!(
                                    "{}, {}",
                                    format_large_number(node_count, "node", "nodes"),
                                    format_large_number(active_node_count, "active", "active")
                                )
                            });

                            if self.disk_task.is_none()
                                && let Some(task_data) = self.disk_task_data.try_lock()
                                && task_data.len() > 0
                            {
                                label.on_hover_ui(|ui| {
                                    ui.label(format_file_size(task_data.len()));
                                });
                            }
                        }
                    }
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
    pub(super) fn title(&self, _shared: &AppShared) -> String {
        match &self.path {
            Some(path) => {
                if let Some(name) = path.file_prefix() {
                    name.to_string_lossy().to_string()
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
                self.modal = EditorModal::ConfirmClose;
                false
            }
        } else {
            true
        }
    }
    pub(super) fn close(&mut self, shared: &mut AppShared, bulk_close: bool) -> bool {
        if let Some(path) = &self.path
            && let Some(weave) = &self.weave
        {
            if bulk_close {
                if let DiskTask::Write(task) = mem::take(&mut self.disk_task) {
                    match block_until_write(&shared.runtime, task) {
                        Ok(()) => {}
                        Err(error) => {
                            shared.toasts.error(error);
                            self.clear_path(shared);
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
                                self.clear_path(shared);
                                return false;
                            }
                        }
                    } else {
                        panic!()
                    }
                }
            } else {
                if !self.close_ready {
                    if let DiskTask::Write(task) = mem::take(&mut self.disk_task) {
                        match block_until_write(&shared.runtime, task) {
                            Ok(()) => {}
                            Err(error) => {
                                shared.toasts.error(error);
                                self.clear_path(shared);
                                return false;
                            }
                        }
                    } else {
                        debug_assert!(self.disk_task.is_none());
                    }

                    let _runtime = shared.runtime.enter();
                    self.disk_task =
                        DiskTask::write(path.clone(), self.disk_task_data.clone(), weave);
                    self.close_ready = true;
                }

                self.close_after_save = true;

                if !self.disk_task.is_none() {
                    return false;
                }
            }
        }

        self.prepare_for_close(shared);

        true
    }
}

#[derive(Default, Debug)]
enum EditorModal {
    #[default]
    None,
    SaveAs(String),
    ConfirmClose,
}
