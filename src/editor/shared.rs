use std::path::PathBuf;

use eframe::egui::{Context, Panel, Ui};
use ulid::Ulid;

use crate::AppShared;

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
        Panel::bottom(ui.id()).show_inside(ui, |ui| {});
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
