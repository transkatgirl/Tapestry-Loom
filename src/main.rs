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
        .mount("/v0", routes![weave::list])
        .mount("/v0", routes![weave::handler])
        .launch()
        .await?;

    Ok(())
}
