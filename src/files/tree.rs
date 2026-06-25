use log::debug;
use ulid::Ulid;

use crate::files::background::BackgroundFsManager;

#[derive(Debug)]
pub struct FileTree {
    last_id: Ulid,
    directory_count: usize,
    file_count: usize,
}

impl Default for FileTree {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTree {
    pub fn new() -> Self {
        Self {
            last_id: Ulid(0),
            directory_count: 0,
            file_count: 0,
        }
    }
    fn reset(&mut self, id: Ulid) {
        self.last_id = id;
        self.directory_count = 0;
        self.file_count = 0;
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
                }
            }

            // TODO

            !finished
        })
    }
}
