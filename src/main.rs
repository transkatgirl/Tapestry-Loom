use std::sync::Arc;

use rocket::{
    fs::{FileServer, relative},
    routes,
};

mod weave;

#[rocket::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), rocket::Error> {
    let _rocket = rocket::build()
        .manage(Arc::new(weave::WeaveSet::default()))
        .mount("/", FileServer::from(relative!("static")))
        .mount(
            "/api",
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

    Ok(())
}
