//! Shorten a URL by POSTing to an endpoint, then GETs to that url will be forwarded on.

#[macro_use] extern crate rocket;
#[macro_use] extern crate diesel;
use diesel::prelude::*;
mod structs;
mod common;
mod schema;
use structs::*;
use rocket_sync_db_pools::database;
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
async fn create_shortened_url(url_id: UrlID, conn: SharesDbConn) -> Result<String, String> {
    //TODO Log new shortened URL to the db.
    use schema::shares::dsl::*;
    // diesel::insert_into(shares::table)
    //     .values(&url_id)
    //     .execute(conn)
    //     .expect("Error saving new post");

    Ok(url_id.get_shorten_url()?.to_owned())
}

///Redirect the user to a shared url
#[get("/<id>")]
fn get_page(id: String, conn: SharesDbConn) -> Redirect {
    use schema::shares::dsl::*;

    // let url_id: UrlID = UrlID::from_token(&id, conn);

    let results = shares
        .limit(5)
        .load::<structs::UrlID>(&conn)
        .expect("Failed to contact db");


    // Redirect::to(url_id.get_dest_url().to_owned())
    Redirect::to("google.com")
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