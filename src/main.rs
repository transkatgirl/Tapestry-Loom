mod weave;

use rocket::{
    fs::{FileServer, relative},
    get, routes,
};

#[get("/hello")]
fn index() -> &'static str {
    "Hello, world!"
}

#[rocket::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), rocket::Error> {
    let _rocket = rocket::build()
        .mount("/", FileServer::from(relative!("static")))
        .mount("/v0", routes![index])
        .mount("/v0", routes![weave::handler])
        .launch()
        .await?;

    Ok(())
}
