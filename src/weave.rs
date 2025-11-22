use std::{
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

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
        fs::{File, read_dir},
        io,
    },
};
use tapestry_weave::v0::TapestryWeave;

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
    async fn list(&self) -> Result<Vec<rusty_ulid::Ulid>, io::Error> {
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
    async fn get_or_create_weave(
        &self,
        id: rusty_ulid::Ulid,
    ) -> Result<Arc<Mutex<WrappedWeave>>, String> {
        let weaves = self.weaves.read().await;

        if let Some(weave) = weaves.get(&id) {
            Ok(weave.clone())
        } else {
            let mut weaves = self.weaves.write().await;

            match weaves.entry(id) {
                Entry::Occupied(entry) => Ok(entry.get().clone()),
                Entry::Vacant(entry) => {
                    let path = self.root.join(id.to_string());
                    let weave = Arc::new(Mutex::new(WrappedWeave::load_or_create(&path).await?));

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
    async fn load_or_create(path: &Path) -> Result<Self, String> {
        todo!()
    }
    //async fn save(&mut self) ->
}

#[get("/weaves")]
pub async fn list(set: &State<WeaveSet>) -> Result<Json<Vec<rusty_ulid::Ulid>>, Status> {
    Ok(Json(set.list().await.map_err(|_| Status::new(500))?))
}

#[get("/weaves/<id>")]
pub fn handler(
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
