use std::{
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use anyhow::Error;
use rocket::{
    State,
    fs::relative,
    futures::{SinkExt, StreamExt, lock::Mutex},
    get,
    http::Status,
    serde::json::Json,
    tokio::{
        fs::{File, read_dir, remove_file, try_exists},
        io::{AsyncReadExt, AsyncWriteExt},
        sync::RwLock,
    },
};
use tapestry_weave::{VersionedWeave, universal_weave::indexmap::IndexMap, v0::TapestryWeave};

pub struct WeaveSet {
    weaves: RwLock<HashMap<rusty_ulid::Ulid, SharedWeave>>,
    root: PathBuf,
}

pub type SharedWeave = Arc<Mutex<Option<WrappedWeave>>>;

impl Default for WeaveSet {
    fn default() -> Self {
        let root: PathBuf = relative!("weaves").into();

        if !root.exists() {
            let _ = std::fs::create_dir(&root);
        }

        Self {
            weaves: HashMap::with_capacity(1024).into(),
            root,
        }
    }
}

impl WeaveSet {
    async fn list(&self) -> Result<Vec<rusty_ulid::Ulid>, Error> {
        let mut stream = read_dir(&self.root).await?;

        let mut items = Vec::with_capacity(self.weaves.read().await.capacity());

        while let Some(v) = stream.next_entry().await? {
            if let Some(filename) = v.file_name().to_str()
                && let Ok(id) = rusty_ulid::Ulid::from_str(filename)
                && v.file_type().await?.is_file()
            {
                items.push(id);
            }
        }

        Ok(items)
    }
    async fn create(&self, id: rusty_ulid::Ulid) -> Result<Option<SharedWeave>, Error> {
        let mut weaves = self.weaves.write().await;

        match weaves.entry(id) {
            Entry::Occupied(_) => Ok(None),
            Entry::Vacant(entry) => {
                let path = self.root.join(id.to_string());
                let weave = Arc::new(Mutex::new(Some(WrappedWeave::create(&path).await?)));

                entry.insert(weave.clone());

                Ok(Some(weave))
            }
        }
    }
    async fn get(&self, id: &rusty_ulid::Ulid) -> Result<Option<SharedWeave>, Error> {
        let weaves = self.weaves.read().await;

        if let Some(weave) = weaves.get(id) {
            Ok(Some(weave.clone()))
        } else {
            let mut weaves = self.weaves.write().await;

            match weaves.entry(*id) {
                Entry::Occupied(entry) => Ok(Some(entry.get().clone())),
                Entry::Vacant(entry) => {
                    let path = self.root.join(id.to_string());

                    let exists = try_exists(&path).await?;

                    if exists {
                        let weave = Arc::new(Mutex::new(Some(WrappedWeave::load(&path).await?)));

                        entry.insert(weave.clone());

                        Ok(Some(weave))
                    } else {
                        Ok(None)
                    }
                }
            }
        }
    }
    async fn opportunistic_unload(&self, id: &rusty_ulid::Ulid) {
        let mut weaves = self.weaves.write().await;

        if let Some(weave) = weaves.remove(id) {
            match Arc::try_unwrap(weave) {
                Ok(_) => {}
                Err(weave) => {
                    weaves.insert(*id, weave);
                }
            }
        }
    }
    async fn delete(&self, id: &rusty_ulid::Ulid) -> Result<(), Error> {
        let mut weaves = self.weaves.write().await;

        if let Some(weave) = weaves.remove(id) {
            match Arc::try_unwrap(weave) {
                Ok(weave) => {
                    if let Some(weave) = weave.into_inner() {
                        weave.delete().await?;
                    }
                }
                Err(weave) => {
                    let mut weave_lock = weave.lock().await;
                    if let Some(weave) = weave_lock.as_ref() {
                        remove_file(weave.path.clone()).await?;
                    }
                    *weave_lock = None;
                }
            }

            Ok(())
        } else {
            Ok(())
        }
    }
}

pub struct WrappedWeave {
    data: TapestryWeave,
    file: File,
    path: PathBuf,
}

impl WrappedWeave {
    async fn create(path: &Path) -> Result<Self, Error> {
        let file = File::create_new(path).await?;
        let weave = TapestryWeave::with_capacity(16384, IndexMap::default());

        let mut wrapped = Self {
            file,
            path: path.to_path_buf(),
            data: weave,
        };
        wrapped.save().await?;

        Ok(wrapped)
    }
    async fn load(path: &Path) -> Result<Self, Error> {
        let mut file = File::options()
            .read(true)
            .write(true)
            .truncate(true)
            .open(path)
            .await?;
        let metadata = file.metadata().await?;

        let len = usize::try_from(metadata.len()).unwrap_or(usize::MAX);

        let mut bytes = Vec::with_capacity(len);
        file.read_to_end(&mut bytes).await?;

        if let Some(weave) = VersionedWeave::from_bytes(&bytes) {
            let mut weave = weave.map(|weave| weave.into_latest())?;

            if weave.capacity() < 16384 {
                weave.reserve(16384 - weave.capacity());
            }

            Ok(Self {
                file,
                path: path.to_path_buf(),
                data: weave,
            })
        } else {
            Err(Error::msg("Invalid file header"))
        }
    }
    async fn save(&mut self) -> Result<(), Error> {
        let data = self.data.to_versioned_bytes()?;
        self.file.write_all(&data).await?;

        Ok(())
    }
    async fn delete(self) -> Result<(), Error> {
        remove_file(self.path).await?;
        Ok(())
    }
}

#[get("/weaves")]
pub async fn list(set: &State<WeaveSet>) -> Result<Json<Vec<rusty_ulid::Ulid>>, Status> {
    Ok(Json(set.list().await.map_err(|e| {
        eprintln!("{e:#?}");
        Status::new(500)
    })?))
}

#[get("/weaves/new")]
pub async fn new(set: &State<WeaveSet>) -> Result<Json<rusty_ulid::Ulid>, Status> {
    let id = rusty_ulid::Ulid::generate();

    let is_success = set
        .create(id)
        .await
        .map_err(|e| {
            eprintln!("{e:#?}");
            Status::new(500)
        })?
        .is_some();

    if is_success {
        Ok(Json(id))
    } else {
        Err(Status::new(409))
    }
}

#[get("/weaves/<id>/ws")]
pub fn socket(
    set: &State<WeaveSet>,
    ws: ws::WebSocket,
    id: rusty_ulid::Ulid,
) -> ws::Channel<'static> {
    ws.channel(move |mut stream| {
        Box::pin(async move {
            while let Some(message) = stream.next().await {
                let _ = stream.send(message?).await;
            }

            Ok(())
        })
    })
}
