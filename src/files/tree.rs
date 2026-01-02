use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs, io, mem,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

use poll_promise::Promise;
use tokio::{runtime::Runtime, task::JoinHandle};
use walkdir::WalkDir;

use crate::settings::Settings;

// TODO: Update this to use logical ordering (directories before files, 1000.txt > 1.txt)

pub struct FileTreeManager {
    settings: Rc<RefCell<Settings>>,
    action_handle: Option<JoinHandle<()>>,
    runtime: Arc<Runtime>,
    channel: (Sender<ScanResult>, Receiver<ScanResult>),
    path: PathBuf,
    roots: BTreeSet<PathBuf>,
    items: BTreeMap<PathBuf, TreeItem>,
    scanned: bool,
    finished: bool,
    file_count: usize,
    folder_count: usize,
    stop_scanning: Arc<AtomicBool>,
}

pub struct FileTree<'a> {
    pub path: &'a PathBuf,
    pub roots: &'a BTreeSet<PathBuf>,
    pub items: &'a BTreeMap<PathBuf, TreeItem>,
    pub finished: &'a bool,
    pub file_count: &'a usize,
    pub folder_count: &'a usize,
}

type ScanResult = Result<ItemScanEvent, anyhow::Error>;

impl FileTreeManager {
    pub fn new(settings: Rc<RefCell<Settings>>, runtime: Arc<Runtime>) -> Self {
        let path = settings.borrow().documents.location.clone();

        let (sender, receiver) = mpsc::channel::<ScanResult>();

        Self {
            settings,
            channel: (sender, receiver),
            action_handle: None,
            runtime,
            path,
            roots: BTreeSet::new(),
            items: BTreeMap::new(),
            scanned: false,
            finished: false,
            file_count: 0,
            folder_count: 0,
            stop_scanning: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn contents(&self) -> FileTree<'_> {
        FileTree {
            path: &self.path,
            roots: &self.roots,
            items: &self.items,
            finished: &self.finished,
            file_count: &self.file_count,
            folder_count: &self.folder_count,
        }
    }
    pub fn refresh(&mut self) {
        self.scanned = false;
    }
    pub fn create_file(&mut self, item: PathBuf, content: Vec<u8>, fail_if_exists: bool) {
        let path = self.path.join(item);
        let tx = self.channel.0.clone();

        let handle = self.runtime.spawn_blocking(move || {
            if fail_if_exists {
                match path.try_exists() {
                    Ok(exists) => {
                        if exists {
                            let _ = tx.send(Err(anyhow::Error::msg("File already exists")));
                            return;
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(Err(error.into()));
                    }
                }
            }

            match fs::write(&path, content) {
                Ok(_) => {
                    let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                        path,
                        r#type: ScannedItemType::File,
                    })));
                }
                Err(error) => {
                    let _ = tx.send(Err(error.into()));
                }
            }
        });

        if self.finished {
            self.action_handle = Some(handle);
        }
    }
    pub fn create_directory(&mut self, item: PathBuf) {
        let path = self.path.join(item);
        let tx = self.channel.0.clone();

        let handle = self
            .runtime
            .spawn_blocking(move || match fs::create_dir_all(&path) {
                Ok(_) => {
                    let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                        path,
                        r#type: ScannedItemType::Directory,
                    })));
                }
                Err(error) => {
                    let _ = tx.send(Err(error.into()));
                }
            });

        if self.finished {
            self.action_handle = Some(handle);
        }
    }
    pub fn move_item(&mut self, item: PathBuf, to: PathBuf, fail_if_exists: bool) {
        let from = self.path.join(item);
        let to = self.path.join(to);
        let tx = self.channel.0.clone();
        let stop_scanning = self.stop_scanning.clone();

        let handle = self.runtime.spawn_blocking(move || {
            if fail_if_exists {
                match to.try_exists() {
                    Ok(exists) => {
                        if exists {
                            let _ = tx.send(Err(anyhow::Error::msg("Path already exists")));
                            return;
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(Err(error.into()));
                    }
                }
            }

            match fs::rename(&from, &to) {
                Ok(_) => {
                    let _ = tx.send(Ok(ItemScanEvent::Delete(from)));
                    match to.metadata() {
                        Ok(metadata) => {
                            if metadata.is_dir() {
                                for entry in WalkDir::new(&to) {
                                    if stop_scanning.load(Ordering::SeqCst) {
                                        break;
                                    }

                                    match entry {
                                        Ok(entry) => {
                                            let filetype = entry.file_type();

                                            let _ =
                                                tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                                                    path: entry.path().to_path_buf(),
                                                    r#type: if filetype.is_file() {
                                                        ScannedItemType::File
                                                    } else if filetype.is_dir() {
                                                        ScannedItemType::Directory
                                                    } else {
                                                        ScannedItemType::Other
                                                    },
                                                })));
                                        }
                                        Err(error) => {
                                            let _ = tx.send(Err(error.into()));
                                        }
                                    }
                                }
                            } else {
                                let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                                    path: to.clone(),
                                    r#type: if metadata.is_file() {
                                        ScannedItemType::File
                                    } else {
                                        ScannedItemType::Other
                                    },
                                })));
                            }
                        }
                        Err(error) => {
                            let _ = tx.send(Err(error.into()));
                        }
                    }
                }
                Err(error) => {
                    let _ = tx.send(Err(error.into()));
                }
            }
        });

        if self.finished {
            self.action_handle = Some(handle);
        }
    }
    pub fn copy_item(&mut self, item: PathBuf, to: PathBuf, fail_if_exists: bool) {
        let from = self.path.join(item);
        let to = self.path.join(to);
        let tx = self.channel.0.clone();
        let stop_scanning = self.stop_scanning.clone();

        let handle = self.runtime.spawn_blocking(move || match from.metadata() {
            Ok(metadata) => {
                if fail_if_exists {
                    match to.try_exists() {
                        Ok(exists) => {
                            if exists {
                                let _ = tx.send(Err(anyhow::Error::msg("Path already exists")));
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = tx.send(Err(error.into()));
                        }
                    }
                }

                if metadata.is_dir() {
                    match copy_dir_all(&from, &to) {
                        Ok(_) => {
                            for entry in WalkDir::new(&to) {
                                if stop_scanning.load(Ordering::SeqCst) {
                                    break;
                                }

                                match entry {
                                    Ok(entry) => {
                                        let filetype = entry.file_type();

                                        let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                                            path: entry.path().to_path_buf(),
                                            r#type: if filetype.is_file() {
                                                ScannedItemType::File
                                            } else if filetype.is_dir() {
                                                ScannedItemType::Directory
                                            } else {
                                                ScannedItemType::Other
                                            },
                                        })));
                                    }
                                    Err(error) => {
                                        let _ = tx.send(Err(error.into()));
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            let _ = tx.send(Err(error.into()));
                        }
                    }
                } else {
                    match fs::copy(&from, &to) {
                        Ok(_) => {
                            let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                                path: to.clone(),
                                r#type: if metadata.is_file() {
                                    ScannedItemType::File
                                } else {
                                    ScannedItemType::Other
                                },
                            })));
                        }
                        Err(error) => {
                            let _ = tx.send(Err(error.into()));
                        }
                    }
                }
            }
            Err(error) => {
                let _ = tx.send(Err(error.into()));
            }
        });

        if self.finished {
            self.action_handle = Some(handle);
        }
    }
    pub fn remove_item(&mut self, item: PathBuf) {
        let path = self.path.join(item);
        let tx = self.channel.0.clone();

        let handle = self
            .runtime
            .spawn_blocking(move || match trash::delete(&path) {
                Ok(_) => {
                    let _ = tx.send(Ok(ItemScanEvent::Delete(path)));
                }
                Err(error) => {
                    let _ = tx.send(Err(error.into()));
                    match path.metadata() {
                        Ok(metadata) => {
                            if metadata.is_dir() {
                                match fs::remove_dir_all(&path) {
                                    Ok(_) => {
                                        let _ = tx.send(Ok(ItemScanEvent::Delete(path)));
                                    }
                                    Err(error) => {
                                        let _ = tx.send(Err(error.into()));
                                    }
                                }
                            } else {
                                match fs::remove_file(&path) {
                                    Ok(_) => {
                                        let _ = tx.send(Ok(ItemScanEvent::Delete(path)));
                                    }
                                    Err(error) => {
                                        let _ = tx.send(Err(error.into()));
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            let _ = tx.send(Err(error.into()));
                        }
                    }
                }
            });

        if self.finished {
            self.action_handle = Some(handle);
        }
    }
    pub fn update_items(
        &mut self,
        mut error_callback: impl FnMut(anyhow::Error),
        max_items: u32,
    ) -> bool {
        let mut has_changed = false;

        let settings = self.settings.borrow();

        if settings.documents.location != self.path {
            self.scanned = false;
            let settings_location = settings.documents.location.clone();
            drop(settings);
            self.path = settings_location;
        } else {
            drop(settings);
        }

        if !self.scanned {
            has_changed = true;
            self.items.clear();
            self.roots.clear();
            self.finished = false;
            self.file_count = 0;
            self.folder_count = 0;
            self.stop_scanning.store(true, Ordering::SeqCst);
            if let Some(handle) = mem::take(&mut self.action_handle) {
                let _guard = self.runtime.enter();
                Promise::spawn_async(handle).block_until_ready();
            }
            while self.channel.1.try_recv().is_ok() {}
            self.stop_scanning.store(false, Ordering::SeqCst);
            let tx = self.channel.0.clone();
            let path = self.path.clone();
            let stop_scanning = self.stop_scanning.clone();
            self.action_handle = Some(self.runtime.spawn_blocking(move || {
                match fs::exists(&path) {
                    Ok(exists) => {
                        if !exists && let Err(error) = fs::create_dir_all(&path) {
                            let _ = tx.send(Err(error.into()));
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(Err(error.into()));
                    }
                }

                for entry in WalkDir::new(&path) {
                    if stop_scanning.load(Ordering::SeqCst) {
                        break;
                    }

                    match entry {
                        Ok(entry) => {
                            let filetype = entry.file_type();

                            let _ = tx.send(Ok(ItemScanEvent::Insert(ScannedItem {
                                path: entry.path().to_path_buf(),
                                r#type: if filetype.is_file() {
                                    ScannedItemType::File
                                } else if filetype.is_dir() {
                                    ScannedItemType::Directory
                                } else {
                                    ScannedItemType::Other
                                },
                            })));
                        }
                        Err(error) => {
                            let _ = tx.send(Err(error.into()));
                        }
                    }
                }

                let _ = tx.send(Ok(ItemScanEvent::Finish));
            }));

            self.scanned = true;
        }

        let mut handled_messages: u32 = 0;
        while let Ok(message) = self.channel.1.try_recv() {
            match message {
                Ok(message) => match message {
                    ItemScanEvent::Insert(insert) => {
                        if insert.path.starts_with(&self.path) {
                            let path = insert
                                .path
                                .strip_prefix(&self.path)
                                .map(|p| p.to_path_buf())
                                .unwrap_or_default();

                            if let Some(parent) = path.parent() {
                                if let Some(TreeItem::Directory(_, children)) =
                                    self.items.get_mut(parent)
                                {
                                    children.insert(path.clone());
                                }
                                if parent == PathBuf::default() {
                                    self.roots.insert(path.clone());
                                }
                            } else if path != PathBuf::default() {
                                self.roots.insert(path.clone());
                            }

                            self.items.insert(
                                path.clone(),
                                match insert.r#type {
                                    ScannedItemType::Directory => {
                                        self.folder_count += 1;
                                        TreeItem::Directory(path, BTreeSet::new())
                                    }
                                    ScannedItemType::File => {
                                        self.file_count += 1;
                                        TreeItem::File(path)
                                    }
                                    ScannedItemType::Other => {
                                        self.file_count += 1;
                                        TreeItem::Other(path)
                                    }
                                },
                            );
                            has_changed = true;
                        }
                    }
                    ItemScanEvent::Delete(delete) => {
                        let delete_path = delete
                            .strip_prefix(&self.path)
                            .map(|p| p.to_path_buf())
                            .unwrap_or_default();

                        let (removed_folders, removed_files) =
                            item_count_recursive(&self.items, &delete_path);

                        if let Some(item) = self.items.remove(&delete_path) {
                            self.roots.remove(item.path());
                            if let Some(parent) = item.path().parent()
                                && let Some(TreeItem::Directory(_, children)) =
                                    self.items.get_mut(parent)
                            {
                                children.remove(item.path());
                            }

                            self.folder_count -= removed_folders;
                            self.file_count -= removed_files;
                        }
                        has_changed = true;
                    }
                    ItemScanEvent::Finish => {
                        self.finished = true;
                        self.action_handle = None;
                    }
                },
                Err(error) => {
                    error_callback(error);
                }
            }
            handled_messages += 1;
            if handled_messages > max_items {
                break;
            }
        }

        has_changed
    }
}

impl Drop for FileTreeManager {
    fn drop(&mut self) {
        self.stop_scanning.store(true, Ordering::SeqCst);
    }
}

enum ItemScanEvent {
    Insert(ScannedItem),
    Delete(PathBuf),
    Finish,
}

#[derive(Debug, Clone)]
pub struct ScannedItem {
    pub path: PathBuf,
    pub r#type: ScannedItemType,
}

impl From<TreeItem> for ScannedItem {
    fn from(value: TreeItem) -> Self {
        match value {
            TreeItem::Directory(path, _) => ScannedItem {
                path,
                r#type: ScannedItemType::Directory,
            },
            TreeItem::File(path) => ScannedItem {
                path,
                r#type: ScannedItemType::File,
            },
            TreeItem::Other(path) => ScannedItem {
                path,
                r#type: ScannedItemType::Other,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScannedItemType {
    File,
    Directory,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TreeItem {
    Directory(PathBuf, BTreeSet<PathBuf>),
    File(PathBuf),
    Other(PathBuf),
}

impl TreeItem {
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Directory(path, _) => path,
            Self::File(path) => path,
            Self::Other(path) => path,
        }
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

fn item_count_recursive(items: &BTreeMap<PathBuf, TreeItem>, path: &PathBuf) -> (usize, usize) {
    if let Some(item) = items.get(path) {
        match item {
            TreeItem::Directory(_, children) => {
                let mut folder_count = 1;
                let mut file_count = 0;

                for child in children {
                    let (child_folder_count, child_file_count) = item_count_recursive(items, child);
                    folder_count += child_folder_count;
                    file_count += child_file_count;
                }

                (folder_count, file_count)
            }
            TreeItem::File(_) => (0, 1),
            TreeItem::Other(_) => (0, 1),
        }
    } else {
        (0, 0)
    }
}
