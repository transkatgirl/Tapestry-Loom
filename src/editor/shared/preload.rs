use std::{path::PathBuf, sync::Arc};

use log::debug;
use parking_lot::Mutex;

use crate::{
    AppShared,
    editor::{
        Editor, EditorShared,
        shared::{DiskTaskData, disk::DiskPreloadTask},
    },
};

pub struct EditorPreloadHandle {
    pub(super) path: PathBuf,
    pub(super) task: DiskPreloadTask,
    pub(super) task_data: Arc<Mutex<DiskTaskData>>,
}

impl EditorPreloadHandle {
    pub fn new(path: PathBuf, shared: &mut AppShared) -> Self {
        debug!("Preloading document {:?}", &path);

        let task_data = Arc::new(Mutex::new(DiskTaskData::new()));
        let data = task_data.clone();

        let _runtime = shared.runtime.enter();
        let task = DiskPreloadTask::new(path.clone(), data);

        Self {
            path,
            task,
            task_data,
        }
    }
    pub fn upgrade(self, shared: &mut AppShared) -> Editor {
        Editor::from(EditorShared::from_preload(self, shared))
    }
}
