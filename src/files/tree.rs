use std::path::PathBuf;

use tapestry_weave::universal_weave::indexmap::{IndexMap, IndexSet};
use ulid::Ulid;

use crate::files::background::BackgroundFsManager;

#[derive(Default, Debug)]
pub struct FileTree {
    last_id: Ulid,
    last_root: Option<PathBuf>,

    file_count: usize,
    directory_count: usize,

    roots: IndexSet<PathBuf>,
    items: IndexMap<PathBuf, (FileType, Vec<PathBuf>)>,
}

pub struct FileTreeData<'a> {
    pub file_count: usize,
    pub directory_count: usize,

    pub roots: &'a IndexSet<PathBuf>,
    pub items: &'a IndexMap<PathBuf, (FileType, Vec<PathBuf>)>,
}

#[derive(Debug)]
pub enum FileType {
    Directory,
    File,
    Symlink,
}

impl FileTree {
    fn reset(&mut self, id: Ulid, root: Option<PathBuf>) {
        self.last_id = id;
        self.last_root = root;
        self.file_count = 0;
        self.directory_count = 0;
        self.roots.clear();
        self.items.clear();
    }
    pub fn update(&mut self, background: &mut BackgroundFsManager) -> bool {
        background.read_cached(|id, root, paths, finished| {
            if self.last_id != id || self.last_root.as_deref() != root {
                self.reset(id, root.map(|r| r.to_owned()));
            }

            debug_assert!(paths.len() >= self.directory_count + self.file_count);

            if paths.len() > self.directory_count + self.file_count {
                for (path, filetype) in &paths[(self.directory_count + self.file_count)..] {
                    if filetype.is_dir() {
                        self.directory_count += 1;
                    } else {
                        self.file_count += 1;
                    }

                    // TODO
                }
            }

            !finished
        })
    }
}
