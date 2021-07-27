//! Shorten a URL by POSTing to an endpoint, then GETs to that url will be forwarded on.

#[macro_use] extern crate rocket;
mod url_id;
mod common;
mod database;
use url_id::*;
use database::*;
use rocket::response::Redirect;

//Configuration
///The IP address of this server, should be set to your domain or IP.
const SERVER_DOMAIN: &str = "127.0.0.1:8000";

/// Create a new shortened URL
/// ```JSON
/// POST
/// {
///     url: String,
///     exp: Integer (optional, if excluded will default to forever)
/// }
/// ```
#[post("/shorten", data = "<url_id>")]
async fn create_shortened_url(url_id: UrlID, conn: SharesDbConn) -> Result<String, String> {
    let inserted: UrlID = add_to_database(&conn, url_id).await?;
    let inserted: UrlID = inserted.generate_token(&conn).await?;
    match inserted.get_shortened_link() {
        Ok(s) => Ok(s),
        Err(e) => Err(e.to_string()),
    }
}

///Making a GET request to this endpoint will create the table in the database automatically if it hasn't been created already.
#[get("/setup")]
async fn setup_db(conn: SharesDbConn) -> Result<String, String> {
    database::setup(&conn).await?;
    Ok("Success".into())
}

///This should be the most commonly used endpoint, and will redirect a user to the correct page the url shortens to!
#[get("/<token>")]
async fn get_page(token: String, conn: SharesDbConn) -> Result<Option<Redirect>, String> { 
    let search_result = Search::Token(token).find_share(&conn).await?;
    if search_result.is_none() {
        return Ok(None)
    }
    Ok(Some(Redirect::to(search_result.unwrap().get_dest_url().to_owned()))) //SAFETY: This unwrap is fine as we have checked it is non-null above!
}

///Automatically catch 404 errors and server a slightly more interesting response.
#[catch(404)]
#[doc(hidden)]
fn not_found(req: &rocket::Request) -> String {
    format!("Sorry, '{}' is not a valid path.", req.uri())
}

#[doc(hidden)]
#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![create_shortened_url, get_page, setup_db])
        .register("/", catchers![not_found])
        .attach(SharesDbConn::fairing())
}