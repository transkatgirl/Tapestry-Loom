use rocket::{
    futures::{SinkExt, StreamExt},
    get,
};

#[get("/weave/<id>")]
pub(super) fn handler(ws: ws::WebSocket, id: u128) -> ws::Channel<'static> {
    ws.channel(move |mut stream| {
        Box::pin(async move {
            while let Some(message) = stream.next().await {
                let _ = stream.send(message?).await;
            }

            Ok(())
        })
    })
}
