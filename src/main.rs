//! Shorten a URL by POSTing to an endpoint, then GETs to that url will be forwarded on.

#[macro_use]
extern crate rocket;
mod common;
mod database;
mod url_id;
mod users;

use std::borrow::Cow;

use database::*;
use rocket::http::{CookieJar, Status};
use rocket::response::{Flash, Redirect};
use rocket::{fairing::AdHoc, form::Form};
use rocket_dyn_templates::{context, Template};
use url_id::*;
use users::User;

use crate::users::NewUser;

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
    auth: User<'_>,
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

    database::increment_clicks(&conn, *search_result.as_ref().unwrap().get_id()).await?;

    Ok(Some(Redirect::to(
        search_result.unwrap().get_dest_url().to_owned(),
    ))) //SAFETY: This unwrap is fine as we have checked it is non-null above!
}

#[delete("/r/<token>")]
async fn delete_page(
    auth: User<'_>,
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
#[get("/dashboard?<page>")]
async fn dashboard(auth: User<'_>, conn: SharesDbConn, page: u32) -> Template {
    // get total count of shares
    let total_count = get_total_share_count(&conn).await.unwrap();
    let shares = Search::Page {
        page: page.into(),
        per_page: 10,
    }
    .find_shares(&conn)
    .await
    .unwrap();

    Template::render(
        "admin",
        context! {
            shares: shares,
            total: total_count,
            page: page,
        },
    )
}

#[get("/dashboard", rank = 2)]
async fn dashboard_no_auth(conn: SharesDbConn) -> Result<String, (Status, &'static str)> {
    let res = "Unauthorized, consider logging in at <a href=\"/login\">/login</a>";
    Err((Status::Unauthorized, res))
}

#[get("/login")]
async fn login() -> Template {
    Template::render("login", context! {})
}

#[post("/login", data = "<login>")]
async fn login_form_submission(
    conn: SharesDbConn,
    login: Form<NewUser<'_>>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    let user = login.into_inner();

    // try to find user in db
    let db_user = database::get_user_by_username(&conn, Cow::Borrowed(&user.username))
        .await
        .unwrap();
    if db_user.is_none() {
        return Flash::error(Redirect::to("/login"), "Invalid username or password");
    }
    let db_user = db_user.unwrap();

    // validate login
    if !user.compare_against(db_user.get_password()) {
        return Flash::error(Redirect::to("/login"), "Invalid username or password");
    }

    // good login - generate cookie and redirect to dashboard
    let cookie = user.username; //TODO: Expiry
    cookies.add_private(rocket::http::Cookie::new("user", cookie.to_string()));
    Flash::success(Redirect::to("/dashboard"), "Successfully logged in!")
}

/// Automatically catch 404 errors and serve a slightly more interesting response.
#[catch(404)]
#[doc(hidden)]
fn not_found(req: &rocket::Request) -> String {
    format!("Sorry, '{}' is not a valid path.", req.uri())
}

#[doc(hidden)]
#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/r", routes![get_page, delete_page])
        .mount("/r", routes![delete_page_no_auth])
        .mount(
            "/",
            routes![
                create_shortened_url,
                dashboard,
                login,
                login_form_submission
            ],
        )
        .mount(
            "/",
            routes![create_shortened_url_no_auth, dashboard_no_auth],
        )
        .register("/", catchers![not_found])
        .attach(Template::fairing())
        .attach(SharesDbConn::fairing())
        .attach(AdHoc::try_on_ignite("Database Init", |rocket| async {
            let conn = SharesDbConn::get_one(&rocket)
                .await
                .expect("database connection");
            database::setup(&conn).await.unwrap(); //XXX return error
            Ok(rocket)
        }))
}
