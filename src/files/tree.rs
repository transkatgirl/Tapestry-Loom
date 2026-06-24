use std::{
    collections::HashSet,
    fs::{FileType, Metadata},
    mem,
    path::{Path, PathBuf},
    sync::{Arc, atomic},
};

use futures::stream::AbortHandle;
use tapestry_weave::universal_weave::indexmap::{IndexMap, IndexSet};
use tokio::{
    sync::Mutex,
    task::{self, JoinHandle},
};
use walkdir::WalkDir;

use crate::{
    AppShared,
    shared::task::{AbortableBlockingTaskHandle, spawn_blocking_abortable},
};

#[derive(Default, Debug)]
pub struct FileTree {
    last_root: Option<PathBuf>,
}

impl FileTree {
    pub fn update(&mut self, shared: &mut AppShared) {
        if self.last_root.as_ref() != Some(&shared.settings.documents.location)
            || shared.open_documents_updated
        {
            self.last_root = Some(shared.settings.documents.location.clone());
            shared.open_documents_updated = false;
            self.scan(shared);
        }
    }
    fn scan(&mut self, shared: &mut AppShared) {}

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

pub struct BackgroundCrawler {
    state: Arc<Mutex<CrawlState>>,
    task: Option<AbortableBlockingTaskHandle<bool>>,
}

pub struct CrawlState {
    paths: IndexMap<PathBuf, FileType>,
    errors: Vec<walkdir::Error>,
    completed: bool,
}

impl CrawlState {
    fn new() -> Self {
        Self {
            paths: IndexMap::with_capacity(16384),
            errors: Vec::new(),
            completed: false,
        }
    }
    fn reset(&mut self) {
        self.paths.clear();
        self.errors.clear();
        self.completed = false;
    }
}

impl BackgroundCrawler {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(CrawlState::new())),
            task: None,
        }
    }
    fn crawl(&mut self, root: PathBuf) {
        if let Some(task) = &self.task {
            task.abort();
            if !task.is_finished() {
                self.state = Arc::new(Mutex::new(CrawlState::new()));
            }
        } else {
            let mut state = self.state.blocking_lock();
            state.reset();
        }

        let state = self.state.clone();
        self.task = Some(spawn_blocking_abortable(move |abort| {
            let walkdir = WalkDir::new(root)
                .follow_links(true)
                .same_file_system(false)
                .sort_by(|a, b| {
                    lexicmp::natural_lexical_cmp(
                        &a.path().to_string_lossy(),
                        &b.path().to_string_lossy(),
                    )
                });

            let mut aborted = false;

            for entry in walkdir {
                let mut state = state.blocking_lock();

                match entry {
                    Ok(entry) => {
                        let file_type = entry.file_type();
                        state.paths.insert(entry.into_path(), file_type);
                    }
                    Err(error) => {
                        state.errors.push(error);
                    }
                }

                if abort.load(atomic::Ordering::Relaxed) {
                    aborted = true;
                    break;
                }
            }

            if !aborted {
                let mut state = state.blocking_lock();
                state.completed = true;
            }

            !aborted
        }));
    }
}
