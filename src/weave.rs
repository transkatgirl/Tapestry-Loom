use std::{
    iter,
    sync::{Arc, Mutex},
};

use rocket::{
    State,
    futures::{SinkExt, StreamExt},
    get,
    serde::json::Json,
};
use tapestry_weave::v0::TapestryWeave;

#[derive(Default, Debug)]
pub struct WeaveSet {}

impl WeaveSet {
    async fn list(&self) -> Vec<rusty_ulid::Ulid> {
        todo!()
    }
    async fn get_or_create_weave(
        &self,
        id: rusty_ulid::Ulid,
    ) -> Result<Arc<Mutex<TapestryWeave>>, String> {
        todo!()
    }
}

#[get("/weaves")]
pub async fn list(set: &State<WeaveSet>) -> Json<Vec<rusty_ulid::Ulid>> {
    Json(set.list().await)
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
