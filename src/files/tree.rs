use std::{
    collections::VecDeque,
    fs::{self, FileType},
    path::{Path, PathBuf},
    sync::{Arc, atomic},
};

use egui_notify::Toast;
use log::{error, warn};
use parking_lot::Mutex;
use tapestry_weave::universal_weave::indexmap::IndexMap;
use tokio::task::{self, JoinHandle};
use walkdir::WalkDir;

use crate::{
    AppShared,
    shared::task::{AbortableBlockingTaskHandle, spawn_blocking_abortable},
};

#[derive(Default, Debug)]
pub struct FileTree {
    last_root: Option<PathBuf>,
    crawler: BackgroundCrawler,
    tasks: VecDeque<JoinHandle<()>>,
}

impl FileTree {
    pub fn update(&mut self, shared: &mut AppShared) {
        if self.last_root.as_ref() != Some(&shared.settings.documents.location) // TODO: Debounce changes
            || shared.open_documents_updated
        {
            self.last_root = Some(shared.settings.documents.location.clone());
            shared.open_documents_updated = false;

            let _runtime = shared.runtime.enter();
            self.crawler.crawl(
                shared.settings.documents.location.clone(),
                shared.async_toasts.clone(),
                true,
            );
        }

        let had_tasks = !self.tasks.is_empty();

        for _ in 0..self.tasks.len() {
            if let Some(task) = self.tasks.pop_front()
                && !task.is_finished()
            {
                self.tasks.push_back(task);
            }
        }

        if had_tasks && self.tasks.is_empty() {
            self.rescan();
        }
    }
    pub fn read<T>(&self, f: impl FnOnce(PathBuf, &IndexMap<PathBuf, FileType>, bool) -> T) -> T {
        let crawl_state = self.crawler.state.lock();
        f(
            self.last_root.clone().unwrap_or_default(),
            &crawl_state.paths,
            crawl_state.completed && self.tasks.is_empty(),
        )
    }

    pub fn likely_exists(&mut self, path: &Path) -> bool {
        let crawl_state = self.crawler.state.lock();
        crawl_state.paths.contains_key(path)
    }
    pub fn create_document(&mut self, shared: &mut AppShared, path: PathBuf) {
        shared.load_document_queue.push(path.to_path_buf()); // TODO: Refresh
    }
    pub fn create_directory(&mut self, shared: &mut AppShared, path: PathBuf) {
        let toasts = shared.async_toasts.clone();
        let _runtime = shared.runtime.enter();

        self.tasks.push_back(task::spawn_blocking(move || {
            if let Err(error) = fs::create_dir_all(&path) {
                toasts
                    .lock()
                    .push(Toast::error("Unable to create directory"));
                warn!("Unable to create directory at {:?}: {:?}", &path, error);
            }
        }));
    }
    pub fn rename_item(&mut self, shared: &mut AppShared, from: PathBuf, to: PathBuf) {
        let toasts = shared.async_toasts.clone();
        let _runtime = shared.runtime.enter();

        self.tasks.push_back(task::spawn_blocking(|| {}));

        todo!()
    }
    pub fn copy_item(&mut self, shared: &mut AppShared, from: PathBuf, to: PathBuf) {
        let toasts = shared.async_toasts.clone();
        let _runtime = shared.runtime.enter();

        self.tasks.push_back(task::spawn_blocking(|| {}));

        todo!()
    }
    pub fn remove_item(&mut self, shared: &mut AppShared, path: PathBuf) {
        let toasts = shared.async_toasts.clone();
        let _runtime = shared.runtime.enter();

        self.tasks.push_back(task::spawn_blocking(move || {
            if let Err(error) = trash::delete(&path) {
                toasts
                    .lock()
                    .push(Toast::error("Unable to move item to trash"));
                warn!("Unable to move {:?} to trash: {:?}", &path, error);

                match fs::symlink_metadata(&path) {
                    Ok(metadata) => {
                        if metadata.is_dir() {
                            if let Err(error) = fs::remove_dir_all(&path) {
                                toasts
                                    .lock()
                                    .push(Toast::error("Unable to remove directory"));
                                warn!("Unable to remove directory {:?}: {:?}", &path, error);
                            }
                        } else {
                            if let Err(error) = fs::remove_file(&path) {
                                toasts.lock().push(Toast::error("Unable to remove file"));
                                warn!("Unable to remove file {:?}: {:?}", &path, error);
                            }
                        }
                    }
                    Err(error) => {
                        toasts
                            .lock()
                            .push(Toast::error("Unable to retrieve item metadata"));
                        warn!("Unable to retrieve metadata for {:?}: {:?}", &path, error);
                    }
                }
            }
        }));
    }
    pub fn rescan(&mut self) {
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
    completed: bool,
}

impl CrawlState {
    fn new() -> Self {
        Self {
            paths: IndexMap::with_capacity(16384),
            completed: false,
        }
    }
    fn reset(&mut self) {
        self.paths.clear();
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
    fn crawl(&mut self, root: PathBuf, toasts: Arc<Mutex<Vec<Toast>>>, natural_sort: bool) {
        if let Some(task) = &self.task {
            task.abort();
            if !task.is_finished() {
                self.state = Arc::new(Mutex::new(CrawlState::new()));
            } else {
                let mut state = self.state.lock();
                state.reset();
            }
        } else {
            let mut state = self.state.lock();
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
                        toasts
                            .lock()
                            .push(Toast::error("Failed to create root directory"));
                        error!("Failed to create directory at {:?}: {:?}", &root, error);
                    }
                }
                Err(error) => {
                    toasts
                        .lock()
                        .push(Toast::error("Failed to determine if root directory exists"));
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
                match entry {
                    Ok(entry) => {
                        let mut state = state.lock();

                        let file_type = entry.file_type();
                        state.paths.insert(entry.into_path(), file_type);
                    }
                    Err(error) => match error.path() {
                        Some(path) => {
                            toasts
                                .lock()
                                .push(Toast::warning(format!("Failed to scan {:?}", path)));
                            warn!("Scanning {:?} failed: {:?}", path, error);
                        }
                        None => {
                            toasts.lock().push(Toast::warning("Failed to scan item"));
                            warn!("Item scanning failed: {:?}", error);
                        }
                    },
                }

                if abort.load(atomic::Ordering::Relaxed) {
                    return false;
                }
            }

            let mut state = state.lock();
            state.completed = true;

            true
        }));
    }
}
