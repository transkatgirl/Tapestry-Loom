use std::path::PathBuf;

use tapestry_weave::universal_weave::indexmap::{IndexMap, IndexSet};
use ulid::Ulid;

use crate::files::background::BackgroundFsManager;

#[derive(Default, Debug)]
pub struct FileTree {
    last_id: Ulid,
    last_root: Option<PathBuf>,

    root_changed: bool,
    updated: bool,

    empty_count: usize,
    file_count: usize,
    directory_count: usize,

    roots: IndexSet<PathBuf>,
    items: IndexMap<PathBuf, TreeItem>,
}

#[derive(Debug)]
pub struct FileTreeState<'a> {
    pub root_changed: bool,
    pub updated: bool,

    pub file_count: usize,
    pub directory_count: usize,

    pub relative_to: &'a PathBuf,

    pub roots: &'a IndexSet<PathBuf>,
    pub items: &'a IndexMap<PathBuf, TreeItem>,
}

#[derive(Debug)]
pub enum TreeItem {
    Directory(Vec<PathBuf>),
    File,
    Symlink,
    Unknown,
}

impl FileTree {
    fn reset(&mut self, id: Ulid, root: Option<PathBuf>) {
        self.last_id = id;
        self.last_root = root;
        self.updated = true;
        self.empty_count = 0;
        self.file_count = 0;
        self.directory_count = 0;
        self.roots.clear();
        self.items.clear();
    }
    pub fn update(&mut self, background: &mut BackgroundFsManager) -> bool {
        self.updated = false;

        background.read_cached(|id, root, paths, finished| {
            if self.last_root.as_deref() != root {
                self.reset(id, root.map(|r| r.to_owned()));
                self.root_changed = true;
            } else {
                if self.last_id != id {
                    self.reset(id, root.map(|r| r.to_owned()));
                }
                self.root_changed = false;
            }

            if root.is_none() {
                return true;
            }

            let root = root.unwrap();

            let count = self.directory_count + self.file_count + self.empty_count;

            debug_assert!(paths.len() >= count);

            if paths.len() > count {
                self.updated = true;

                for (path, filetype) in &paths[count..] {
                    let path = path.strip_prefix(root).unwrap().to_owned();

                    if path.as_os_str().is_empty() {
                        self.empty_count += 1;
                        continue;
                    }

                    if let Some(parent) = path.parent()
                        && !parent.as_os_str().is_empty()
                    {
                        if let Some(TreeItem::Directory(children)) = self.items.get_mut(parent) {
                            children.push(path.to_owned());
                        }
                    } else {
                        self.roots.insert(path.to_owned());
                    }

                    assert!(!self.items.contains_key(&path));

                    self.items.insert(
                        path,
                        if filetype.is_symlink() {
                            TreeItem::Symlink
                        } else if filetype.is_dir() {
                            TreeItem::Directory(Vec::new())
                        } else if filetype.is_file() {
                            TreeItem::File
                        } else {
                            TreeItem::Unknown
                        },
                    );

                    if filetype.is_dir() {
                        self.directory_count += 1;
                    } else {
                        self.file_count += 1;
                    }
                }
            }

            !finished
        })
    }
    pub fn view<'s>(&'s self) -> FileTreeState<'s> {
        FileTreeState {
            root_changed: self.root_changed,
            updated: self.updated,
            file_count: self.file_count,
            directory_count: self.directory_count,
            relative_to: self.last_root.as_ref().unwrap(),
            roots: &self.roots,
            items: &self.items,
        }
    }
}
