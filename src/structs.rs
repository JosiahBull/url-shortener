use rocket::outcome::Outcome::*;
use crate::common::*;
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket_sync_db_pools::{rusqlite, database};
use rocket::http::{Status, ContentType, Header};
use serde::{Serialize, Deserialize};
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
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct UrlID {
    ///ID
    id: Option<i64>,
    /// When this url expires
    exp: i64,
    /// When this url was created
    crt: i64,
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
            exp: std::i64::MAX,
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

    pub fn set_exp(mut self, exp: &i64) -> Self {
        self.exp = exp.to_owned();
        self
    }

    pub fn get_dest_url(&self) -> &str {
        &self.url
    }
    
    pub fn get_exp(&self) -> &i64 {
        &self.exp
    }

    pub fn get_crt(&self) -> &i64 {
        &self.crt
    }

    pub fn is_expired(&self) -> &bool {
        &self.expired
    }

    pub fn get_token(&self) -> Result<&str, ShareError> {
        if let Some(token) = &self.token {
            return Ok(token);
        }
        Err(ShareError::A("No Token!".into()))
    }

    pub fn generate_token(mut self) -> Result<Self, ShareError> {
        //Todo generate token here
        self.token=Some("TemporaryToken".into());
        
        Ok(self)
    }

    pub fn get_shorten_url(&self) -> Result<&str, ShareError> {
        //TODO make this return a proper URL, just not the token.
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
        //Ensure correct content type
        let share_ct = ContentType::new("application", "json");
        if req.content_type() != Some(&share_ct) {
            return Failure((Status::UnsupportedMediaType, ShareError::A("Content Type Failure".into())));
        }

        let limit = 1024.bytes(); //Set the maximum size we'll unwrap
        //Read the data
        let string = match data.open(limit).into_string().await {
            Ok(string) if string.is_complete() => string.into_inner(),
            Ok(_) => return Failure((Status::PayloadTooLarge, ShareError::A("Too large".into()))),
            Err(e) => return Failure((Status::InternalServerError, ShareError::A(e.to_string()))),
        };
        
        let string = rocket::request::local_cache!(req, string);

        // Attempt to parse the string with serde into our struct
        let mut share: UrlID = match serde_json::from_str(string) {
            Ok(share) => share,
            Err(e) => return Failure((Status::BadRequest, ShareError::A(format!("Unable to parse string with serde: {}", e.to_string())))),
        };

        Success(share)
    }
}

