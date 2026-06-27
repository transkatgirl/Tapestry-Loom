use std::path::PathBuf;

use crate::{AppShared, editor::Editor};

pub struct EditorPreloadHandle {
    path: PathBuf,
}

impl EditorPreloadHandle {
    pub fn new(path: PathBuf, shared: &mut AppShared) -> Self {
        // TODO
        Self { path }
    }
    pub fn upgrade(self, shared: &mut AppShared) -> Editor {
        // TODO
        Editor::new(Some(self.path), shared)
    }
}
