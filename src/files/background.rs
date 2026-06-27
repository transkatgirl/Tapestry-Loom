use std::{
    collections::{HashSet, VecDeque},
    ffi::OsString,
    fs::{self, FileType},
    io,
    path::{Path, PathBuf},
    sync::{Arc, atomic},
};

use egui_notify::Toast;
use log::{debug, error, warn};
use parking_lot::Mutex;
use tapestry_weave::universal_weave::indexmap::IndexMap;
use tokio::task::{self, JoinHandle};
use ulid::Ulid;
use walkdir::WalkDir;

use crate::{
    AppShared,
    shared::task::{AbortableBlockingTaskHandle, spawn_blocking_abortable},
};

#[derive(Default, Debug)]
pub struct BackgroundFsManager {
    last_root: Option<PathBuf>,
    last_natural_sort: bool,
    crawler: BackgroundCrawler,
    tasks: VecDeque<JoinHandle<()>>,
}

impl BackgroundFsManager {
    pub fn update(&mut self, shared: &mut AppShared) {
        let had_tasks = !self.tasks.is_empty();

        for _ in 0..self.tasks.len() {
            if let Some(task) = self.tasks.pop_front()
                && !task.is_finished()
            {
                self.tasks.push_back(task);
            }
        }

        if self.last_root.as_ref() != Some(&shared.settings.documents.location)
            || self.last_natural_sort != shared.settings.documents.natural_sort
            || (had_tasks && self.tasks.is_empty())
        {
            self.last_root = Some(shared.settings.documents.location.clone());
            self.last_natural_sort = shared.settings.documents.natural_sort;

            let _runtime = shared.runtime.enter();
            self.crawler.crawl(
                shared.settings.documents.location.clone(),
                shared.async_toasts.clone(),
                shared.settings.documents.natural_sort,
            );
        }
    }
    pub fn read_cached<T>(
        &self,
        f: impl FnOnce(Ulid, Option<&Path>, &IndexMap<PathBuf, FileType>, bool) -> T,
    ) -> T {
        let crawl_state = self.crawler.state.lock();
        f(
            crawl_state.id,
            self.last_root.as_deref(),
            &crawl_state.paths,
            self.crawler
                .task
                .as_ref()
                .map(|t| t.is_finished())
                .unwrap_or(true)
                && self.tasks.is_empty(),
        )
    }
    pub fn likely_exists(&mut self, path: &Path) -> bool {
        let crawl_state = self.crawler.state.lock();
        crawl_state.paths.contains_key(path)
    }
    pub fn create_file(
        &mut self,
        shared: &mut AppShared,
        path: PathBuf,
        contents: impl FnOnce() -> Vec<u8> + Send + 'static,
    ) {
        let toasts = shared.async_toasts.clone();
        let _runtime = shared.runtime.enter();

        self.tasks.push_back(task::spawn_blocking(move || {
            debug!("Started background task create_file({:?})", &path);

            match fs::exists(&path) {
                Ok(true) => {
                    toasts
                        .lock()
                        .push(Toast::error(format!("{:?} already exists", &path)));
                    warn!("Item {:?} already exists", &path);
                }
                Ok(false) => {
                    if let Err(error) = fs::write(&path, contents()) {
                        toasts
                            .lock()
                            .push(Toast::error(format!("Unable to create {:?}", &path)));
                        warn!("Unable to create+write to file at {:?}: {:?}", &path, error);
                    }
                }
                Err(error) => {
                    toasts.lock().push(Toast::error(format!(
                        "Unable to check if {:?} exists",
                        &path
                    )));
                    warn!("Unable to check if {:?} exists: {:?}", &path, error);
                }
            }

            debug!("Finished background task create_file({:?})", &path);
        }));
    }
    pub fn create_directory(&mut self, shared: &mut AppShared, path: PathBuf) {
        let toasts = shared.async_toasts.clone();
        let _runtime = shared.runtime.enter();

        self.tasks.push_back(task::spawn_blocking(move || {
            debug!("Started background task create_directory({:?})", &path);

            if let Err(error) = fs::create_dir_all(&path) {
                toasts
                    .lock()
                    .push(Toast::error(format!("Unable to create {:?}", &path)));
                warn!("Unable to create directory at {:?}: {:?}", &path, error);
            }

            debug!("Finished background task create_directory({:?})", &path);
        }));
    }
    pub fn rename_item(&mut self, shared: &mut AppShared, from: PathBuf, to: PathBuf) {
        let toasts = shared.async_toasts.clone();
        let _runtime = shared.runtime.enter();

        self.tasks.push_back(task::spawn_blocking(move || {
            debug!(
                "Started background task rename_item({:?}, {:?})",
                &from, &to
            );

            match fs::exists(&to) {
                Ok(true) => {
                    toasts
                        .lock()
                        .push(Toast::error(format!("{:?} already exists", &to)));
                    warn!("Item {:?} already exists", &to);
                }
                Ok(false) => {
                    if let Err(error) = fs::rename(&from, &to) {
                        toasts
                            .lock()
                            .push(Toast::error(format!("Unable to rename {:?}", &from)));
                        warn!("Unable to rename {:?} to {:?}: {:?}", &from, &to, error);
                    }
                }
                Err(error) => {
                    toasts
                        .lock()
                        .push(Toast::error(format!("Unable to check if {:?} exists", &to)));
                    warn!("Unable to check if {:?} exists: {:?}", &to, error);
                }
            }

            debug!(
                "Finished background task rename_item({:?}, {:?})",
                &from, &to
            );
        }));
    }
    pub fn copy_item(&mut self, shared: &mut AppShared, from: PathBuf, to: PathBuf) {
        let toasts = shared.async_toasts.clone();
        let _runtime = shared.runtime.enter();

        self.tasks.push_back(task::spawn_blocking(move || {
            debug!("Started background task copy_item({:?}, {:?})", &from, &to);

            match fs::exists(&to) {
                Ok(true) => {
                    toasts
                        .lock()
                        .push(Toast::error(format!("{:?} already exists", &to)));
                    warn!("Item {:?} already exists", &to);
                }
                Ok(false) => match fs::symlink_metadata(&from) {
                    Ok(metadata) => {
                        if metadata.is_dir() {
                            if let Err(error) = copy_dir_all(&from, &to) {
                                toasts
                                    .lock()
                                    .push(Toast::error(format!("Unable to copy {:?}", &from)));
                                warn!(
                                    "Unable to copy directory {:?} to {:?}: {:?}",
                                    &from, &to, error
                                );
                            }
                        } else {
                            if let Err(error) = fs::copy(&from, &to) {
                                toasts
                                    .lock()
                                    .push(Toast::error(format!("Unable to copy {:?}", &from)));
                                warn!("Unable to copy file {:?} to {:?}: {:?}", &from, &to, error);
                            }
                        }
                    }
                    Err(error) => {
                        toasts.lock().push(Toast::error(format!(
                            "Unable to retrieve metadata for {:?}",
                            &from
                        )));
                        warn!("Unable to retrieve metadata for {:?}: {:?}", &from, error);
                    }
                },
                Err(error) => {
                    toasts
                        .lock()
                        .push(Toast::error(format!("Unable to check if {:?} exists", &to)));
                    warn!("Unable to check if {:?} exists: {:?}", &to, error);
                }
            }

            debug!("Finished background task copy_item({:?}, {:?})", &from, &to);
        }));
    }
    pub fn remove_item(&mut self, shared: &mut AppShared, path: PathBuf) {
        let toasts = shared.async_toasts.clone();
        let _runtime = shared.runtime.enter();

        self.tasks.push_back(task::spawn_blocking(move || {
            debug!("Started background task remove_item({:?})", &path);

            if let Err(error) = trash::delete(&path) {
                toasts
                    .lock()
                    .push(Toast::error(format!("Unable to move {:?} to trash", &path)));
                warn!("Unable to move {:?} to trash: {:?}", &path, error);

                match fs::symlink_metadata(&path) {
                    Ok(metadata) => {
                        if metadata.is_dir() {
                            if let Err(error) = fs::remove_dir_all(&path) {
                                toasts
                                    .lock()
                                    .push(Toast::error(format!("Unable to remove {:?}", &path)));
                                warn!("Unable to remove directory {:?}: {:?}", &path, error);
                            }
                        } else {
                            if let Err(error) = fs::remove_file(&path) {
                                toasts
                                    .lock()
                                    .push(Toast::error(format!("Unable to remove {:?}", &path)));
                                warn!("Unable to remove file {:?}: {:?}", &path, error);
                            }
                        }
                    }
                    Err(error) => {
                        toasts.lock().push(Toast::error(format!(
                            "Unable to retrieve metadata for {:?}",
                            &path
                        )));
                        warn!("Unable to retrieve metadata for {:?}: {:?}", &path, error);
                    }
                }
            }

            debug!("Finished background task remove_item({:?})", &path);
        }));
    }
    pub fn refresh(&mut self) {
        self.last_root = None;
    }
}

#[derive(Debug)]
struct BackgroundCrawler {
    ignore_list: Arc<HashSet<OsString>>,
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
    id: Ulid,
    paths: IndexMap<PathBuf, FileType>,
}

impl CrawlState {
    fn new() -> Self {
        Self {
            id: Ulid::new(),
            paths: IndexMap::with_capacity(16384),
        }
    }
    fn reset(&mut self) {
        self.paths.clear();
        self.id = Ulid::new();
    }
}

impl BackgroundCrawler {
    fn new() -> Self {
        Self {
            ignore_list: Arc::new(HashSet::from_iter(
                [
                    ".directory",
                    ".ds_store",
                    "__macosx",
                    ".appledouble",
                    ".lsoverride",
                    "thumbs.db",
                    "thumbs.db:encryptable",
                    "ehthumbs.db",
                    "desktop.ini",
                ]
                .into_iter()
                .map(OsString::from),
            )),
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

        let ignore_list = self.ignore_list.clone();
        let state = self.state.clone();
        self.task = Some(spawn_blocking_abortable(move |abort| {
            debug!("Started crawl task for {:?}", &root);

            match fs::exists(&root) {
                Ok(true) => {}
                Ok(false) => {
                    debug!(
                        "Root directory {:?} does not exist, ending crawl early",
                        &root
                    );
                    return true;
                }
                Err(error) => {
                    toasts.lock().push(Toast::error(format!(
                        "Unable to check if {:?} exists",
                        &root
                    )));
                    error!("Unable to check if {:?} exists: {:?}", &root, error);
                    debug!("Aborted crawling {:?}", &root);
                    return false;
                }
            }

            if abort.load(atomic::Ordering::Relaxed) {
                debug!("Aborted crawling {:?}", &root);
                return false;
            }

            let walkdir = if natural_sort {
                WalkDir::new(&root).follow_links(false).sort_by(|a, b| {
                    b.file_type().is_dir().cmp(&a.file_type().is_dir()).then(
                        lexicmp::natural_lexical_cmp(
                            &a.file_name().to_string_lossy(),
                            &b.file_name().to_string_lossy(),
                        ),
                    )
                })
            } else {
                WalkDir::new(&root).follow_links(false).sort_by(|a, b| {
                    b.file_type()
                        .is_dir()
                        .cmp(&a.file_type().is_dir())
                        .then(a.file_name().cmp(b.file_name()))
                })
            };

            for entry in walkdir {
                match entry {
                    Ok(entry) => {
                        let file_type = entry.file_type();
                        if file_type.is_dir()
                            || !ignore_list.contains(&entry.file_name().to_ascii_lowercase())
                        {
                            let mut state = state.lock();
                            state.paths.insert(entry.into_path(), file_type);
                        }
                    }
                    Err(error) => match error.path() {
                        Some(path) => {
                            toasts
                                .lock()
                                .push(Toast::warning(format!("Failed to scan {:?}", path)));
                            warn!("Scanning {:?} failed: {:?}", path, error);
                        }
                        None => {
                            toasts.lock().push(Toast::warning("Scanning failed"));
                            warn!("Item scanning failed: {:?}", error);
                        }
                    },
                }

                if abort.load(atomic::Ordering::Relaxed) {
                    debug!("Aborted crawling {:?}", &root);
                    return false;
                }
            }

            debug!("Finished crawling {:?}", &root);

            true
        }));
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
