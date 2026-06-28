use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use log::{debug, error};
use parking_lot::Mutex;
use tapestry_weave::VersionedWeave;
use tokio::task::{self, JoinHandle};

use crate::common::task::AbortableBlockingTaskHandle;

pub(super) enum DiskTask {
    None,
    Read(AbortableBlockingTaskHandle<Result<VersionedWeave, String>>),
    Write(JoinHandle<Result<(), String>>),
}

impl DiskTask {
    pub(super) fn read(path: PathBuf, data: Arc<Mutex<DiskTaskData>>) -> Self {
        Self::Read(AbortableBlockingTaskHandle::new(move |abort| {
            debug!("Started read task for {:?}", &path);

            let mut data = data.try_lock().unwrap();

            if let Err(error) = data.open(&path, false, &abort) {
                if abort.load(Ordering::Relaxed) {
                    debug!("Aborted opening {:?}", &path);
                } else {
                    error!("Failed to open {:?}: {:?}", path, error);
                }

                return Err(format!("Failed to open {:?}", path));
            }

            if let Err(error) = data.read(&abort) {
                if abort.load(Ordering::Relaxed) {
                    debug!("Aborted reading {:?}", &path);
                } else {
                    error!("Failed to read {:?}: {:?}", path, error);
                }

                return Err(format!("Failed to read {:?}", path));
            }

            match VersionedWeave::from_bytes(&data.buffer) {
                Some(Ok(weave)) => {
                    debug!("Finished reading {:?}", &path);
                    Ok(weave)
                }
                Some(Err(error)) => {
                    error!("Failed to deserialize {:?}: {:?}", path, error);
                    Err(format!("Failed to deserialize {:?}", path))
                }
                None => {
                    error!("Failed to parse {:?} header", path);
                    Err(format!("Failed to deserialize {:?}", path))
                }
            }
        }))
    }
    pub(super) fn write(
        path: PathBuf,
        data: Arc<Mutex<DiskTaskData>>,
        weave: &VersionedWeave,
    ) -> Self {
        let serialization_result = {
            let mut data = data.try_lock().unwrap();
            data.buffer.clear();
            weave.write_bytes(&mut data.buffer)
            // TODO: benchmark this on commonly used platforms; rkyv serialization might be faster than cloning
        };

        Self::Write(task::spawn_blocking(move || {
            debug!("Started write task for {:?}", &path);

            if let Err(error) = serialization_result {
                error!("Failed to serialize {:?}: {:?}", path, error);
                return Err(format!("Failed to serialize {:?}", path));
            }

            let mut data = data.try_lock().unwrap();

            if data.file.is_none()
                && let Err(error) = data.open_unabortable(&path, true)
            {
                error!("Failed to open {:?}: {:?}", path, error);
                return Err(format!("Failed to open {:?}", path));
            }

            if let Err(error) = data.write() {
                error!("Failed to write {:?}: {:?}", path, error);
                return Err(format!("Failed to write {:?}", path));
            }

            debug!("Finished writing {:?}", &path);

            Ok(())
        }))
    }
}

pub(super) struct DiskPreloadTask {
    handle: AbortableBlockingTaskHandle<Result<VersionedWeave, String>>,
}

impl From<DiskPreloadTask> for DiskTask {
    fn from(value: DiskPreloadTask) -> Self {
        Self::Read(value.handle)
    }
}

impl DiskPreloadTask {
    pub(super) fn new(path: PathBuf, data: Arc<Mutex<DiskTaskData>>) -> Self {
        Self {
            handle: AbortableBlockingTaskHandle::new(move |abort| {
                debug!("Started read task for {:?}", &path);

                let mut data = data.try_lock().unwrap();

                if let Err(error) = data.open(&path, false, &abort) {
                    if abort.load(Ordering::Relaxed) {
                        debug!("Aborted opening {:?}", &path);
                    } else {
                        error!("Failed to open {:?}: {:?}", path, error);
                    }

                    return Err(format!("Failed to open {:?}", path));
                }

                if let Err(error) = data.read_chunked(&abort) {
                    if abort.load(Ordering::Relaxed) {
                        debug!("Aborted reading {:?}", &path);
                    } else {
                        error!("Failed to read {:?}: {:?}", path, error);
                    }

                    return Err(format!("Failed to read {:?}", path));
                }

                match VersionedWeave::from_bytes(&data.buffer) {
                    Some(Ok(weave)) => {
                        debug!("Finished reading {:?}", &path);
                        Ok(weave)
                    }
                    Some(Err(error)) => {
                        error!("Failed to deserialize {:?}: {:?}", path, error);
                        Err(format!("Failed to deserialize {:?}", path))
                    }
                    None => {
                        error!("Failed to parse {:?} header", path);
                        Err(format!("Failed to deserialize {:?}", path))
                    }
                }
            }),
        }
    }
}

pub(super) struct DiskTaskData {
    file: Option<File>,
    buffer: Vec<u8>,
}

impl DiskTaskData {
    pub(super) fn new() -> Self {
        Self {
            file: None,
            buffer: Vec::with_capacity(16384),
        }
    }
    fn open(&mut self, path: &Path, create: bool, abort: &AtomicBool) -> Result<(), io::Error> {
        assert!(self.file.is_none());

        let file = File::options()
            .create(create)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;

        if abort.load(Ordering::Relaxed) {
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        };

        file.try_lock()?;

        if abort.load(Ordering::Relaxed) {
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        };

        self.file = Some(file);

        Ok(())
    }
    fn open_unabortable(&mut self, path: &Path, create: bool) -> Result<(), io::Error> {
        assert!(self.file.is_none());

        let file = File::options()
            .create(create)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;

        file.try_lock()?;

        self.file = Some(file);

        Ok(())
    }
    fn read(&mut self, abort: &AtomicBool) -> Result<(), io::Error> {
        if let Some(file) = &mut self.file {
            file.seek(SeekFrom::Start(0))?;

            if abort.load(Ordering::Relaxed) {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            };

            let size = usize::try_from(file.metadata()?.len()).unwrap_or(usize::MAX);

            if abort.load(Ordering::Relaxed) {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            };

            if size > self.buffer.capacity() {
                self.buffer.reserve(size - self.buffer.len());
            }

            self.buffer.clear();
            file.read_to_end(&mut self.buffer)?;

            if abort.load(Ordering::Relaxed) {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            };

            Ok(())
        } else {
            Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }
    fn read_chunked(&mut self, abort: &AtomicBool) -> Result<(), io::Error> {
        if let Some(file) = &mut self.file {
            file.seek(SeekFrom::Start(0))?;

            if abort.load(Ordering::Relaxed) {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            };

            let size = usize::try_from(file.metadata()?.len()).unwrap_or(usize::MAX);

            if abort.load(Ordering::Relaxed) {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            };

            if size > self.buffer.capacity() {
                self.buffer.reserve(size - self.buffer.len());
            }

            self.buffer.clear();

            let mut buffer = [0u8; 8192]; // Based on BufReader's default buffer size

            loop {
                let len = file.read(&mut buffer)?;

                if len == 0 {
                    if abort.load(Ordering::Relaxed) {
                        return Err(io::Error::from(io::ErrorKind::Interrupted));
                    };

                    break;
                } else {
                    self.buffer.extend(&buffer[..len]);

                    if abort.load(Ordering::Relaxed) {
                        return Err(io::Error::from(io::ErrorKind::Interrupted));
                    };
                };
            }

            Ok(())
        } else {
            Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }
    fn write(&mut self) -> Result<(), io::Error> {
        if let Some(file) = &mut self.file {
            file.set_len(self.buffer.len() as u64)?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&self.buffer)?;
            file.flush()?;
            file.sync_data()?;

            Ok(())
        } else {
            Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }
}
