//! Shorten a URL by POSTing to an endpoint, then GETs to that url will be forwarded on.

#[macro_use] extern crate rocket;
mod structs;
mod common;
mod database;
use structs::*;
use database::*;
use rocket::response::Redirect;

//Configuration
const SERVER_DOMAIN: &str = "127.0.0.1:8000";

/// Create a new shortened URL
/// ```JSON
/// POST
/// {
///     url: String
/// }
/// ```
#[post("/shorten", data = "<url_id>")]
async fn create_shortened_url(url_id: UrlID, conn: SharesDbConn) -> Result<String, String> {
    let inserted: UrlID = add_to_database(&conn, url_id).await?;
    let inserted: UrlID = inserted.generate_token(&conn).await?;
    match inserted.get_shortened_link() {
        Ok(s) => return Ok(s),
        Err(e) => return Err(e.to_string()),
    }
}

///Initally Setup the Db
#[get("/setup")]
async fn setup_db(conn: SharesDbConn) -> Result<String, String> {
    database::setup(&conn).await?;
    Ok("Success".into())
}

///Redirect the user to a shared url
#[get("/<token>")]
async fn get_page(token: String, conn: SharesDbConn) -> Result<Option<Redirect>, String> { 
    let search_result = Search::Token(token).find_share(&conn).await?;
    if search_result.is_none() {
        return Ok(None)
    }
    Ok(Some(Redirect::to(search_result.unwrap().get_dest_url().to_owned()))) //SAFETY: This unwrap is fine as we have checked it is non-null above!
}

#[catch(404)]
fn not_found(req: &rocket::Request) -> String {
    format!("Sorry, '{}' is not a valid path.", req.uri())
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![create_shortened_url, get_page, setup_db])
        .register("/", catchers![not_found])
        .attach(SharesDbConn::fairing())
}