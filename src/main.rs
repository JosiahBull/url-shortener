//! Shorten a URL by POSTing to an endpoint, then GETs to that url will be forwarded on.

#[macro_use] extern crate rocket;

mod structs;
mod common;
use structs::*;

//Configuration
const SERVER_DOMAIN: &str = "127.0.0.1";


#[post("/shorten", data = "<url_id>")]
fn create_shortened_url(url_id: UrlID) -> &'static str {
    "Not Implemented"
}

#[get("/<url_id>")]
fn get_page(url_id: UrlID) -> &'static str {
    "Not Implemented"
}


#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![get_page])
}