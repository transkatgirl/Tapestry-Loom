use std::{
    fs::{self, FileType},
    path::{Path, PathBuf},
    sync::{Arc, atomic},
};

use log::{error, warn};
use tapestry_weave::universal_weave::indexmap::IndexMap;
use tokio::sync::Mutex;
use walkdir::WalkDir;

use crate::{
    AppShared,
    shared::task::{AbortableBlockingTaskHandle, spawn_blocking_abortable},
};

#[derive(Default, Debug)]
pub struct FileTree {
    last_root: Option<PathBuf>,
    crawler: BackgroundCrawler,
    last_completed: bool,
}

impl FileTree {
    pub fn update(&mut self, shared: &mut AppShared) {
        if self.last_root.as_ref() != Some(&shared.settings.documents.location)
            || shared.open_documents_updated
        {
            self.last_root = Some(shared.settings.documents.location.clone());
            self.last_completed = false;
            shared.open_documents_updated = false;

            let _runtime = shared.runtime.enter();
            self.crawler
                .crawl(shared.settings.documents.location.clone(), true);
        }

        if let Ok(crawl_state) = self.crawler.state.try_lock() {
            if !self.last_completed {
                println!("{:?}", crawl_state);

                // TODO
            }

            if crawl_state.completed && !self.last_completed {
                self.last_completed = true;
            }
        }
    }

    pub fn likely_exists(&mut self, path: &Path) -> bool {
        let crawl_state = self.crawler.state.blocking_lock();
        crawl_state.paths.contains_key(path)
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

#[derive(Debug)]
struct BackgroundCrawler {
    state: Arc<Mutex<CrawlState>>,
    task: Option<AbortableBlockingTaskHandle<bool>>,
}

impl Default for BackgroundCrawler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct CrawlState {
    paths: IndexMap<PathBuf, FileType>,
    errors: Vec<String>,
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
    fn crawl(&mut self, root: PathBuf, natural_sort: bool) {
        if let Some(task) = &self.task {
            task.abort();
            if !task.is_finished() {
                self.state = Arc::new(Mutex::new(CrawlState::new()));
            } else {
                let mut state = self.state.blocking_lock();
                state.reset();
            }
        } else {
            let mut state = self.state.blocking_lock();
            state.reset();
        }

        let state = self.state.clone();
        self.task = Some(spawn_blocking_abortable(move |abort| {
            match fs::exists(&root) {
                Ok(exists) => {
                    if abort.load(atomic::Ordering::Relaxed) {
                        return false;
                    }

                    if !exists && let Err(error) = fs::create_dir_all(&root) {
                        let mut state = state.blocking_lock();
                        state
                            .errors
                            .push("Failed to create root directory".to_string());
                        error!("Failed to create directory at {:?}: {:?}", &root, error);
                    }
                }
                Err(error) => {
                    let mut state = state.blocking_lock();
                    state
                        .errors
                        .push("Failed to determine if root directory exists".to_string());
                    error!("Failed to determine if {:?} exists: {:?}", &root, error);
                    return false;
                }
            }

            let walkdir = if natural_sort {
                WalkDir::new(&root)
                    .follow_links(true)
                    .same_file_system(false)
                    .sort_by(|a, b| {
                        lexicmp::natural_lexical_cmp(
                            &a.file_name().to_string_lossy(),
                            &b.file_name().to_string_lossy(),
                        )
                    })
            } else {
                WalkDir::new(&root)
                    .follow_links(true)
                    .same_file_system(false)
                    .sort_by_file_name()
            };

            for entry in walkdir {
                let mut state = state.blocking_lock();

                match entry {
                    Ok(entry) => {
                        let file_type = entry.file_type();
                        state.paths.insert(entry.into_path(), file_type);
                    }
                    Err(error) => match error.path() {
                        Some(path) => {
                            state.errors.push(format!("Failed to scan {:?}", path));
                            warn!("Scanning {:?} failed: {:?}", path, error);
                        }
                        None => {
                            state.errors.push("Failed to scan item".to_string());
                            warn!("Item scanning failed: {:?}", error);
                        }
                    },
                }

                if abort.load(atomic::Ordering::Relaxed) {
                    return false;
                }
            }

            let mut state = state.blocking_lock();
            state.completed = true;

            true
        }));
    }
}
