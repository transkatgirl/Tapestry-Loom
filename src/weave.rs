use std::{
    collections::HashMap,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use rocket::{
    State,
    fs::relative,
    futures::{SinkExt, StreamExt},
    get,
    http::Status,
    serde::json::Json,
    tokio::{
        fs::{File, read_dir},
        io,
    },
};
use tapestry_weave::v0::TapestryWeave;

pub struct WeaveSet {
    weaves: HashMap<rusty_ulid::Ulid, WrappedWeave>,
    root: PathBuf,
}

impl Default for WeaveSet {
    fn default() -> Self {
        Self {
            weaves: HashMap::with_capacity(1024),
            root: relative!("weaves").into(),
        }
    }
}

impl WeaveSet {
    async fn list(&self) -> Result<Vec<rusty_ulid::Ulid>, io::Error> {
        let mut stream = read_dir(&self.root).await?;

        let mut items = Vec::with_capacity(self.weaves.capacity());

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
        todo!()
    }
}

pub struct WrappedWeave {
    data: TapestryWeave,
    file: File,
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
