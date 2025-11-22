#[macro_use]
extern crate rocket;

use rocket::fs::{relative, FileServer};

#[get("/hello")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", FileServer::from(relative!("static")))
        .mount("/hello", routes![index])
}
