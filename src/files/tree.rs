use std::path::PathBuf;

use tapestry_weave::universal_weave::indexmap::{IndexMap, IndexSet};
use ulid::Ulid;

use crate::files::background::BackgroundFsManager;

#[derive(Default, Debug)]
pub struct FileTree {
    last_id: Ulid,
    last_root: Option<PathBuf>,

    pub(super) was_reset: bool,

    pub(super) file_count: usize,
    pub(super) directory_count: usize,

    pub(super) roots: IndexSet<PathBuf>,
    pub(super) items: IndexMap<PathBuf, TreeItem>,
}

#[derive(Debug)]
pub enum TreeItem {
    Directory(Vec<PathBuf>),
    File,
    Symlink,
}

impl FileTree {
    fn reset(&mut self, id: Ulid, root: Option<PathBuf>) {
        self.last_id = id;
        self.last_root = root;
        self.was_reset = true;
        self.file_count = 0;
        self.directory_count = 0;
        self.roots.clear();
        self.items.clear();
    }
    pub fn update(&mut self, background: &mut BackgroundFsManager) -> bool {
        background.read_cached(|id, root, paths, finished| {
            if self.last_id != id || self.last_root.as_deref() != root {
                self.reset(id, root.map(|r| r.to_owned()));
            } else {
                self.was_reset = false;
            }

            if root.is_none() {
                return true;
            }

            let root = root.unwrap();

            debug_assert!(paths.len() >= self.directory_count + self.file_count);

            if paths.len() > self.directory_count + self.file_count {
                for (path, filetype) in &paths[(self.directory_count + self.file_count)..] {
                    let path = path.strip_prefix(root).unwrap().to_path_buf();

                    if path.as_os_str().is_empty() {
                        continue;
                    }

                    if let Some(parent) = path.parent()
                        && !parent.as_os_str().is_empty()
                    {
                        if let Some(TreeItem::Directory(children)) = self.items.get_mut(parent) {
                            children.push(path.clone());
                        }
                    } else {
                        self.roots.insert(path.clone());
                    }

                    self.items.insert(
                        path,
                        if filetype.is_dir() {
                            TreeItem::Directory(Vec::new())
                        } else if filetype.is_symlink() {
                            TreeItem::Symlink
                        } else {
                            TreeItem::File
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
}
