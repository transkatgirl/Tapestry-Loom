mod weave;

use rocket::{
    fs::{FileServer, relative},
    routes,
};

#[rocket::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), rocket::Error> {
    let _rocket = rocket::build()
        .manage(weave::WeaveSet::default())
        .mount("/", FileServer::from(relative!("static")))
        .mount(
            "/api",
            routes![
                weave::list,
                weave::create,
                weave::download,
                weave::delete,
                weave::socket
            ],
        )
        .launch()
        .await?;

    Ok(())
}
