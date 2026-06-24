use std::path::{Path, PathBuf};

use crate::AppShared;

#[derive(Default, Debug)]
pub struct FileTree {
    last_root: Option<PathBuf>,
}

impl FileTree {
    pub fn update(&mut self, shared: &mut AppShared) {
        if self.last_root.as_ref() != Some(&shared.settings.documents.location)
            || shared.open_documents_updated
        {
            // TODO
            self.last_root = Some(shared.settings.documents.location.clone());
            shared.open_documents_updated = false;
        }
    }

    pub fn likely_exists(&mut self, path: &Path) -> bool {
        todo!()
    }
    pub fn create_document(&mut self, shared: &mut AppShared, path: PathBuf) {
        shared.load_document_queue.push(path.to_path_buf()); // TODO: Refresh
    }
    pub fn create_directory(&mut self, shared: &mut AppShared, path: PathBuf) {
        todo!()
    }
    pub fn rename_item(&mut self, shared: &mut AppShared, from: PathBuf, to: PathBuf) {
        todo!()
    }
    pub fn copy_item(&mut self, shared: &mut AppShared, from: PathBuf, to: PathBuf) {
        todo!()
    }
    pub fn remove_item(&mut self, shared: &mut AppShared, path: PathBuf) {
        todo!()
    }
    pub fn reset(&mut self) {
        self.last_root = None;
    }
}
