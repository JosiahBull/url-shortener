//! Shorten a URL by POSTing to an endpoint, then GETs to that url will be forwarded on.

#[macro_use]
extern crate rocket;
mod common;
mod database;
mod url_id;
mod users;

use database::*;
use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::response::Redirect;
use url_id::*;
use users::AdminUser;

//Configuration
/// The IP address of this server, should be set to your domain or IP.
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
async fn create_shortened_url(
    auth: AdminUser<'_>,
    url_id: UncommittedUrlID,
    conn: SharesDbConn,
) -> Result<String, (Status, String)> {
    let inserted: UrlID = add_share_to_database(&conn, url_id).await?;
    Ok(inserted.get_shortened_link())
}

#[post("/shorten", data = "<url_id>", rank = 2)]
async fn create_shortened_url_no_auth(
    url_id: UncommittedUrlID,
    conn: SharesDbConn,
) -> Result<String, (Status, String)> {
    Err((Status::Unauthorized, "Unauthorized".to_owned()))
}

/// This should be the most commonly used endpoint, and will redirect a user to the correct page the url shortens to!
#[get("/r/<token>")]
async fn get_page(token: String, conn: SharesDbConn) -> Result<Option<Redirect>, (Status, String)> {
    let converted_id: &str = token.split(url_id::DELIM_CHAR).collect::<Vec<&str>>()[0];
    let search_result = Search::Id(base_61_to_10(converted_id, url_id::ALPHABET))
        .find_share(&conn)
        .await?;
    if search_result.is_none() {
        return Ok(None);
    }
    Ok(Some(Redirect::to(
        search_result.unwrap().get_dest_url().to_owned(),
    ))) //SAFETY: This unwrap is fine as we have checked it is non-null above!
}

#[delete("/r/<token>")]
async fn delete_page(
    auth: AdminUser<'_>,
    token: String,
    conn: SharesDbConn,
) -> Result<(), (Status, String)> {
    let converted_id: &str = token.split(url_id::DELIM_CHAR).collect::<Vec<&str>>()[0];
    let search_result = Search::Id(base_61_to_10(converted_id, url_id::ALPHABET))
        .find_share(&conn)
        .await?;
    if search_result.is_none() {
        return Err((Status::NotFound, "Not found".to_owned()));
    }
    delete_share_from_database(&conn, *search_result.unwrap().get_id())
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    Ok(())
}

#[delete("/r/<token>", rank = 2)]
async fn delete_page_no_auth(token: String, conn: SharesDbConn) -> Result<(), (Status, String)> {
    Err((Status::Unauthorized, "Unauthorized".to_owned()))
}

/// Load the admin page, which will allow you to view all shortened URLs, and delete them (if logged in)
#[get("/admin")]
async fn admin_page(auth: AdminUser<'_>, conn: SharesDbConn) -> Result<String, (Status, String)> {
    // let all_shares = get_all_shares(&conn).await?;
    // let mut html = String::from("<html><body><h1>Admin Page</h1><ul>");
    // for share in all_shares {
    //     html.push_str(&format!(
    //         "<li><a href=\"{}\">{}</a> - <a href=\"{}\">Delete</a></li>",
    //         share.get_shortened_link(),
    //         share.get_shortened_link(),
    //         format!("/delete/{}", share.get_id())
    //     ));
    // }
    // html.push_str("</ul></body></html>");
    // Ok(html)

    Ok("Admin page".to_owned())
}

#[get("/admin", rank = 2)]
async fn admin_page_no_auth(conn: SharesDbConn) -> Result<String, (Status, &'static str)> {
    let res = "Unauthorized, consider logging in at <a href=\"/login\">/login</a>";
    Err((Status::Unauthorized, res))
}

#[get("/login")]
async fn login_page() -> String {
    "Login page".to_owned()
}

/// Automatically catch 404 errors and server a slightly more interesting response.
#[catch(404)]
#[doc(hidden)]
fn not_found(req: &rocket::Request) -> String {
    format!("Sorry, '{}' is not a valid path.", req.uri())
}

#[doc(hidden)]
#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![create_shortened_url])
        .mount("/r", routes![get_page])
        .register("/", catchers![not_found])
        .attach(SharesDbConn::fairing())
        .attach(AdHoc::try_on_ignite("Database Init", |rocket| async {
            let conn = SharesDbConn::get_one(&rocket)
                .await
                .expect("database connection");
            database::setup(&conn);
            Ok(rocket)
        }))
}
