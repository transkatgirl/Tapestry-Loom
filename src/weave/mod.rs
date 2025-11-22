use std::{
    borrow::Cow,
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use anyhow::Error;
use rocket::{
    State, delete,
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
use ws::frame::{CloseCode, CloseFrame};

mod socket;

pub struct WeaveSet {
    weaves: RwLock<HashMap<rusty_ulid::Ulid, SharedWeave>>,
    root: PathBuf,
}

pub type SharedWeave = Arc<Mutex<Option<(WrappedWeave, usize)>>>;

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
                let weave = Arc::new(Mutex::new(Some((WrappedWeave::create(&path).await?, 0))));

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
            drop(weaves);
            let mut weaves = self.weaves.write().await;

            match weaves.entry(*id) {
                Entry::Occupied(entry) => Ok(Some(entry.get().clone())),
                Entry::Vacant(entry) => {
                    let path = self.root.join(id.to_string());

                    let exists = try_exists(&path).await?;

                    if exists {
                        let weave =
                            Arc::new(Mutex::new(Some((WrappedWeave::load(&path).await?, 0))));

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
    async fn delete(&self, id: &rusty_ulid::Ulid) -> Result<bool, Error> {
        let mut weaves = self.weaves.write().await;

        if let Some(weave) = weaves.remove(id) {
            match Arc::try_unwrap(weave) {
                Ok(weave) => {
                    if let Some(weave) = weave.into_inner() {
                        weave.0.delete().await?;
                    }
                }
                Err(weave) => {
                    let mut weave_lock = weave.lock().await;
                    if let Some(weave) = weave_lock.as_ref() {
                        remove_file(weave.0.path.clone()).await?;
                    }
                    *weave_lock = None;
                }
            }

            Ok(true)
        } else {
            let path = self.root.join(id.to_string());
            let exists = try_exists(&path).await?;

            if exists {
                remove_file(path).await?;
            }

            Ok(exists)
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
        let mut file = File::options().read(true).write(true).open(path).await?;
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
        self.file.flush().await?;

        Ok(())
    }
    async fn delete(self) -> Result<(), Error> {
        remove_file(self.path).await?;
        Ok(())
    }
}

#[get("/weaves")]
pub async fn list(set: &State<Arc<WeaveSet>>) -> Result<Json<Vec<rusty_ulid::Ulid>>, Status> {
    Ok(Json(set.list().await.map_err(|e| {
        eprintln!("{e:#?}");
        Status::new(500)
    })?))
}

#[get("/weaves/new")]
pub async fn create(set: &State<Arc<WeaveSet>>) -> Result<Json<rusty_ulid::Ulid>, Status> {
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
        set.opportunistic_unload(&id).await;
        Ok(Json(id))
    } else {
        eprintln!("Generated duplicate ULID: {}", id);
        Err(Status::new(500))
    }
}

#[get("/weaves/<id>")]
pub async fn download(set: &State<Arc<WeaveSet>>, id: rusty_ulid::Ulid) -> Result<Vec<u8>, Status> {
    let weave = set.get(&id).await.map_err(|e| {
        eprintln!("{e:#?}");
        Status::new(500)
    })?;

    let result = match weave {
        Some(weave) => {
            let lock = weave.lock().await;
            match lock.as_ref() {
                Some(weave) => weave.0.data.to_versioned_bytes().map_err(|e| {
                    eprintln!("{e:#?}");
                    Status::new(500)
                }),
                None => Err(Status::new(404)),
            }
        }
        None => Err(Status::new(404)),
    };

    set.opportunistic_unload(&id).await;

    result
}

#[delete("/weaves/<id>")]
pub async fn delete(set: &State<Arc<WeaveSet>>, id: rusty_ulid::Ulid) -> Status {
    match set.delete(&id).await {
        Ok(exists) => {
            if exists {
                Status::new(200)
            } else {
                Status::new(404)
            }
        }
        Err(e) => {
            eprintln!("{e:#?}");
            Status::new(500)
        }
    }
}

#[get("/weaves/<id>/ws")]
pub async fn websocket(
    set: &State<Arc<WeaveSet>>,
    ws: ws::WebSocket,
    id: rusty_ulid::Ulid,
) -> ws::Channel<'static> {
    let weave = set.get(&id).await;

    match weave {
        Ok(weave) => match weave {
            Some(weave) => {
                let identifier = id;
                let set = set.inner().clone();

                ws.channel(move |mut stream| {
                    Box::pin(async move {
                        let mut last_value = None;

                        while let Some(message) = stream.next().await {
                            let mut weave = weave.lock().await;
                            if let Some(weave) = weave.as_mut() {
                                let (responses, has_changed) = socket::handle_message(
                                    &mut weave.0.data,
                                    weave.1 != last_value.unwrap_or(weave.1),
                                    message?,
                                );

                                for message in responses {
                                    stream.send(message).await?;
                                }
                                if has_changed {
                                    weave.1 = weave.1.wrapping_add(1);
                                }
                                last_value = Some(weave.1);
                            } else {
                                stream
                                    .send(ws::Message::Close(Some(CloseFrame {
                                        code: CloseCode::Away,
                                        reason: Cow::Borrowed("Item has been deleted"),
                                    })))
                                    .await?;
                                set.opportunistic_unload(&identifier).await;
                                return Ok(());
                            }
                        }

                        set.opportunistic_unload(&identifier).await;

                        Ok(())
                    })
                })
            }
            None => ws.channel(move |mut stream| {
                Box::pin(async move {
                    stream
                        .send(ws::Message::Close(Some(CloseFrame {
                            code: CloseCode::Policy,
                            reason: Cow::Borrowed("Not found"),
                        })))
                        .await?;
                    Ok(())
                })
            }),
        },
        Err(err) => ws.channel(move |mut stream| {
            Box::pin(async move {
                eprintln!("{err:#?}");
                stream
                    .send(ws::Message::Close(Some(CloseFrame {
                        code: CloseCode::Error,
                        reason: Cow::Borrowed("Internal error"),
                    })))
                    .await?;
                Ok(())
            })
        }),
    }
}
