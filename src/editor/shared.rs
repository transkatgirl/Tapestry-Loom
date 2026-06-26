use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    ops::DerefMut,
    path::PathBuf,
    sync::Arc,
};

use eframe::egui::{Align, Context, Layout, OutputCommand, Panel, Ui};
use parking_lot::Mutex;
use tapestry_weave::{VersionedWeave, v1::dependent::TapestryWeave};
use ulid::Ulid;

use crate::{
    AppShared,
    shared::{task::AbortableBlockingTaskHandle, ui::abbreviate_path},
};

pub(super) struct EditorShared {
    pub id: Ulid,
    path: Option<PathBuf>, // TODO: Document loading, create root dir if it doesn't exist

    disk_task: DiskTask,
    disk_task_data: Arc<Mutex<DiskTaskData>>,
    pub weave: Option<TapestryWeave>,
}

enum DiskTask {
    None,
    Read(AbortableBlockingTaskHandle<Result<VersionedWeave, String>>),
    Write(AbortableBlockingTaskHandle<Result<(), String>>),
}

struct DiskTaskData {
    file: Option<File>,
    buffer: Vec<u8>,
}

impl DiskTaskData {
    fn new() -> Self {
        Self {
            file: None,
            buffer: Vec::with_capacity(16384),
        }
    }
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

        Self {
            id: Ulid::new(),
            path,
            disk_task: DiskTask::None,
            disk_task_data: Arc::new(Mutex::new(DiskTaskData::new())),
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

fn read_to_buffer(path: PathBuf, data: Arc<Mutex<DiskTaskData>>) -> Result<(), io::Error> {
    let mut lock = data.lock();
    let data = lock.deref_mut();

    match &mut data.file {
        Some(file) => {
            file.seek(SeekFrom::Start(0))?;

            let size = file
                .metadata()
                .map(|m| usize::try_from(m.len()).unwrap_or(usize::MAX))
                .ok();

            if let Some(size) = size
                && size > data.buffer.capacity()
            {
                data.buffer.reserve(size - data.buffer.len());
            }

            data.buffer.clear();
            file.read_to_end(&mut data.buffer)?;
        }
        None => {
            let mut file = File::options()
                .create(false)
                .truncate(false)
                .read(true)
                .write(true)
                .open(path)?;

            file.seek(SeekFrom::Start(0))?;

            let size = file
                .metadata()
                .map(|m| usize::try_from(m.len()).unwrap_or(usize::MAX))
                .ok();

            if let Some(size) = size
                && size > data.buffer.capacity()
            {
                data.buffer.reserve(size - data.buffer.len());
            }

            data.buffer.clear();
            file.read_to_end(&mut data.buffer)?;

            data.file = Some(file);
        }
    }

    Ok(())
}

fn write_from_buffer(path: PathBuf, data: Arc<Mutex<DiskTaskData>>) -> Result<(), io::Error> {
    let mut lock = data.lock();
    let data = lock.deref_mut();

    match &mut data.file {
        Some(file) => {
            file.set_len(data.buffer.len() as u64)?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&data.buffer)?;
            file.flush()?;
        }
        None => {
            let mut file = File::options()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(path)?;

            file.set_len(data.buffer.len() as u64)?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&data.buffer)?;
            file.flush()?;

            data.file = Some(file);
        }
    }

    Ok(())
}
