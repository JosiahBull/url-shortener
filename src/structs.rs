use rocket::outcome::Outcome::*;
use crate::common::*;
use rocket::data::{self, Data, FromData};
use rocket_sync_db_pools::{rusqlite, database};

///// Error Structs /////

#[derive(Debug)]
pub enum ShareError {
    A(String)
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

pub enum DatabaseError {
    A(String)
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(error: rusqlite::Error) -> DatabaseError {
        DatabaseError::A(error.to_string())
    }
}

impl From<DatabaseError> for String {
    fn from(err: DatabaseError) -> String {
        "A database error occured!".into()
    }
}

/////  Data Structs  /////

#[database("sqlite_shares")]
pub struct SharesDbConn(rusqlite::Connection);

/// This struct represents a valid url ID
#[derive(Debug)]
pub struct UrlID {
    ///ID
    id: Option<u64>,
    /// When this url expires
    exp: u64,
    /// When this url was created
    crt: u64,
    /// Url this redirects to
    url: String,
    /// Has this token expired
    expired: bool,
    ///The token that the custom url uses
    token: Option<String>,
}


impl Default for UrlID {
    fn default() -> Self {
        UrlID {
            id: None,
            exp: std::u64::MAX,
            crt: get_time_seconds(),
            url: String::default(),
            expired: bool::default(),
            token: None,
        }
    }
}

impl UrlID {
    pub fn new(url: &str) -> Self {
        let mut def = Self::default();
        def.url = url.to_owned();
        def
    }

    pub fn from_token(id: &str) -> Result<Self, ShareError> {
        //TODO
        Ok(Self::default())
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
        self.token=Some("TemporaryToken".into());
        
        Ok(self)
    }

    pub fn get_shorten_url(&self) -> Result<&str, ShareError> {
        if let Some(token) = &self.token {
            return Ok(token);
        }
        Err(ShareError::A("No Token!".into()))
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
            token: Some("Temporary Token".into()),
        })
    }
}

