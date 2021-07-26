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
#[get("/<id>")]
async fn get_page(id: String, conn: SharesDbConn) -> Option<Redirect> {    
    // struct Search {

    // }
    // impl database::Searchable for Search {
    //     fn select(&self) -> String {
    //         "url = \"www.google.com\"".into()
    //     }
    // }
    // let search = Search {};
    // let result = search_database(&conn, search).await.unwrap();
    // println!("{:?}", result);
    // if let Ok(url_id) = UrlID::from_token(&id) {
        // return Some(Redirect::to(url_id.get_dest_url().to_owned()));
    // }
    None
    // Some(Redirect::to("Good things!"))
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