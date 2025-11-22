use std::{
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use anyhow::Error;
use parking_lot::Mutex;
use rocket::{
    State,
    fs::relative,
    futures::{SinkExt, StreamExt},
    get,
    http::Status,
    serde::json::Json,
    tokio::{
        self,
        fs::{File, metadata, read_dir},
        io::{AsyncReadExt, AsyncWriteExt},
    },
};
use tapestry_weave::{VersionedWeave, universal_weave::indexmap::IndexMap, v0::TapestryWeave};

pub struct WeaveSet {
    weaves: tokio::sync::RwLock<HashMap<rusty_ulid::Ulid, Arc<Mutex<WrappedWeave>>>>,
    root: PathBuf,
}

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
    async fn create(&self, id: rusty_ulid::Ulid) -> Result<Arc<Mutex<WrappedWeave>>, Error> {
        let mut weaves = self.weaves.write().await;

        match weaves.entry(id) {
            Entry::Occupied(_) => Err(Error::msg("Item already exists")),
            Entry::Vacant(entry) => {
                let path = self.root.join(id.to_string());
                let weave = Arc::new(Mutex::new(WrappedWeave::create(&path).await?));

                entry.insert(weave.clone());

                Ok(weave)
            }
        }
    }
    async fn get(&self, id: rusty_ulid::Ulid) -> Result<Arc<Mutex<WrappedWeave>>, Error> {
        let weaves = self.weaves.read().await;

        if let Some(weave) = weaves.get(&id) {
            Ok(weave.clone())
        } else {
            let mut weaves = self.weaves.write().await;

            match weaves.entry(id) {
                Entry::Occupied(entry) => Ok(entry.get().clone()),
                Entry::Vacant(entry) => {
                    let path = self.root.join(id.to_string());
                    let weave = Arc::new(Mutex::new(WrappedWeave::load(&path).await?));

                    entry.insert(weave.clone());

                    Ok(weave)
                }
            }
        }
    }
}

pub struct WrappedWeave {
    data: TapestryWeave,
    file: File,
}

impl WrappedWeave {
    async fn create(path: &Path) -> Result<Self, Error> {
        let file = File::create_new(path).await?;
        let weave = TapestryWeave::with_capacity(16384, IndexMap::default());

        let mut wrapped = Self { file, data: weave };
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

            Ok(Self { file, data: weave })
        } else {
            Err(Error::msg("Invalid header"))
        }
    }
    async fn save(&mut self) -> Result<(), Error> {
        let data = self.data.to_versioned_bytes()?;
        self.file.write_all(&data).await?;

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

    set.create(id).await.map_err(|e| {
        eprintln!("{e:#?}");
        Status::new(500)
    })?;

    Ok(Json(id))
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
