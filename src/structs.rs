use rocket::outcome::Outcome::*;
use crate::common::*;
use rocket::data::{self, Data, FromData};
use rocket_sync_db_pools::{diesel, database};

#[database("sqlite_shares")]
pub struct SharesDbConn(diesel::SqliteConnection);

#[derive(Debug)]
pub enum ShareError {

}

impl From<ShareError> for String {
    fn from(err: ShareError) -> String {
        "A share error occcured!".into()
    }
}

impl std::error::Error for ShareError {
    fn description(&self) -> &str {
        "Failed to parse url correctly."
    }
}

impl std::fmt::Display for ShareError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // match &*self {

        //   }
        f.write_str("Failed to parse the url correctly.")
    }
}

/// This struct represents a valid url ID
pub struct UrlID {
    /// When this url expires
    exp: u64,
    /// When this url was created
    crt: u64,
    /// Url this redirects to
    url: String,
}



impl Default for UrlID {
    fn default() -> Self {
        UrlID {
            exp: std::u64::MAX,
            crt: get_time_seconds(),
            url: String::default(),
        }
    }
}

impl UrlID {
    pub fn new(url: &str) -> Self {
        let mut def = Self::default();
        def.url = url.to_owned();
        def
    }

    pub fn from_id(id: &str, conn: SharesDbConn) -> Self {
        //TODO
        Self::default()
    }

    pub fn set_url(mut self, url: &str) -> Self {
        self.url = url.to_owned();
        self
    }

    pub fn set_exp(mut self, exp: &u64) -> Self {
        self.exp = exp.to_owned();
        self
    }

    pub fn get_shorten_url(&self) -> Result<String, ShareError> {
        Ok("Not yet implemented".into())
    }

    pub fn get_dest_url(self) -> String {
        self.url
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for UrlID {
    type Error = ShareError;
    async fn from_data(req: &'r rocket::request::Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        //This is expected to be a conversion from a post request as a new url for shortening
        //Convert from serde based on the incoming data

        Success(UrlID {
            exp: std::u64::MAX,
            crt: std::u64::MAX,
            url: "www.google.com".into(),
        })
    }
}