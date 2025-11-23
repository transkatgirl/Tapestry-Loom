use std::sync::Arc;

use rocket::{
    fs::{FileServer, relative},
    routes,
};

mod weave;

#[rocket::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), rocket::Error> {
    let weave_set = Arc::new(weave::WeaveSet::default());

    let _rocket = rocket::build()
        .manage(weave_set.clone())
        .mount("/", FileServer::from(relative!("frontend/dist")))
        .mount(
            "/api/v0",
            routes![
                weave::list,
                weave::create,
                weave::download,
                weave::delete,
                weave::websocket
            ],
        )
        .launch()
        .await?;

    weave_set.unload().await;

    Ok(())
}
