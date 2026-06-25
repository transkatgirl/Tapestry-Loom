use std::path::PathBuf;

use tapestry_weave::universal_weave::indexmap::{IndexMap, IndexSet};
use ulid::Ulid;

use crate::files::background::BackgroundFsManager;

#[derive(Default, Debug)]
pub struct FileTree {
    last_id: Ulid,

    file_count: usize,
    directory_count: usize,

    roots: IndexSet<PathBuf>,
    items: IndexMap<PathBuf, (FileType, Vec<PathBuf>)>,
}

#[derive(Debug)]
pub enum FileType {
    Directory,
    File,
    Symlink,
}

impl FileTree {
    fn reset(&mut self, id: Ulid) {
        self.last_id = id;
        self.file_count = 0;
        self.directory_count = 0;
        self.roots.clear();
        self.items.clear();
    }
    pub fn update(&mut self, background: &mut BackgroundFsManager) -> bool {
        background.read_cached(|id, root, paths, finished| {
            if self.last_id != id {
                self.reset(id);
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
