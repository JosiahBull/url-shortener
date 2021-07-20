//! Shorten a URL by POSTing to an endpoint, then GETs to that url will be forwarded on.

#[macro_use] extern crate rocket;

mod structs;
mod common;
use structs::*;
use rocket_sync_db_pools::{diesel, database};
use rocket::response::Redirect;

//Configuration
const SERVER_DOMAIN: &str = "127.0.0.1";

/// Create a new shortened URL
/// ```JSON
/// POST
/// {
///     url: String
/// }
/// ```
#[post("/shorten", data = "<url_id>")]
async fn create_shortened_url(url_id: UrlID) -> Result<String, String> {
    //TODO Log new shortened URL to the db.
    Ok(url_id.get_shorten_url()?)
}

///Redirect the user to a shared url
#[get("/<id>")]
fn get_page(id: String, conn: SharesDbConn) -> Redirect {
    let url_id: UrlID = UrlID::from_id(&id, conn);

    Redirect::to(url_id.get_dest_url())
}

#[catch(404)]
fn not_found(req: &rocket::Request) -> String {
    format!("Sorry, '{}' is not a valid path.", req.uri())
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![get_page, create_shortened_url])
        .register("/", catchers![not_found])
        .attach(SharesDbConn::fairing())
}