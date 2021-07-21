use rocket::outcome::Outcome::*;
use crate::common::*;
use rocket::data::{self, Data, FromData};
use rocket_sync_db_pools::database;
use crate::schema::shares;
use serde::{Serialize, Deserialize};

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
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Queryable)]
pub struct UrlID {
    ///ID
    pub id: Option<u64>,
    /// When this url expires
    pub exp: u64,
    /// When this url was created
    pub crt: u64,
    /// Url this redirects to
    pub url: String,
    /// Has this token expired
    pub expired: bool,
    ///The token that the custom url uses
    pub token: String,
}


impl Default for UrlID {
    fn default() -> Self {
        UrlID {
            id: None,
            exp: std::u64::MAX,
            crt: get_time_seconds(),
            url: String::default(),
            expired: bool::default(),
            token: String::default(),
        }
    }
}

impl UrlID {
    pub fn new(url: &str) -> Self {
        let mut def = Self::default();
        def.url = url.to_owned();
        def
    }

    pub fn from_token(id: &str, conn: SharesDbConn) -> Self {
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

    pub fn get_dest_url(&self) -> &str {
        &self.url
    }

    pub fn generate_token(mut self) -> Result<Self, ShareError> {
        //Todo generate token here
        self.token="TemporaryToken".into();
        
        Ok(self)
    }

    pub fn get_shorten_url(&self) -> Result<&str, ShareError> {
        Ok(&self.token)
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
            id: None,
            expired: false,
            token: "TemporaryToken".into(),
        })
    }
}