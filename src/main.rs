//! Shorten a URL by POSTing to an endpoint, then GETs to that url will be forwarded on.

#[macro_use] extern crate rocket;
#[macro_use] extern crate diesel;
mod structs;
mod common;
mod database;
use structs::*;
use database::*;
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
    conn.run(|c| {
        c.execute("CREATE TABLE shares (
            id MEDIUMINT(255) PRIMARY KEY,
            exp INTEGER(255) NOT NULL,
            crt INTEGER(255) NOT NULL,
            url TEXT NOT NULL,
            expired BOOLEAN NOT NULL DEFAULT 'f',
            token TEXT NOT NULL
        )", []).unwrap();
    }).await;

    Ok(url_id.get_shorten_url()?.to_owned())
}

///Initally Setup the Db
#[get("/setup")]
async fn setup_db(conn: SharesDbConn) -> Result<String, String> {
    database::setup(&conn).await?;
    Ok("Success".into())
}

///Redirect the user to a shared url
#[get("/<id>")]
fn get_page(id: String, conn: SharesDbConn) -> Redirect {    
    
    //TODO Implement result type on this, on error forward to 404 handler!
    let url_id: UrlID = UrlID::from_token(&id);

    Redirect::to(url_id.get_dest_url().to_owned())
}

#[catch(404)]
fn not_found(req: &rocket::Request) -> String {
    format!("Sorry, '{}' is not a valid path.", req.uri())
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![get_page, create_shortened_url, setup_db])
        .register("/", catchers![not_found])
        .attach(SharesDbConn::fairing())
}