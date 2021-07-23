use rocket::outcome::Outcome::*;
use crate::common::*;
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket_sync_db_pools::{rusqlite, database};
use rocket::http::{Status, ContentType};
use serde::{Serialize, Deserialize};

///// Error Structs /////
pub fn get_token(mut id: i64) -> String {
    //Converting from base 10 (id), to base 36.
    let mut result: String = String::default();
    loop {
        result.insert(0, "0123456789abcdefghijklmnopqrstuvwxyz".chars().nth((id % 36) as usize).unwrap());
        id = id/36;
        if id <= 1 {
            break;
        }
    }
    result
}

#[derive(Debug)]
pub enum ShareError {
    ContentType,
    TooLarge,
    ServerError(String),
    ParseFailure(String),
}

impl From<ShareError> for String {
    fn from(err: ShareError) -> String {
        match err {
            ShareError::ContentType => "incorrect content-type provided on request".into(),
            ShareError::TooLarge => "request payload too large".into(),
            ShareError::ServerError(e) => e,
            ShareError::ParseFailure(e) => e,
        }
    }
}

impl std::error::Error for ShareError {
    fn description(&self) -> &str {
        match &self {
            ShareError::ContentType => "incorrect content-type provided on request",
            ShareError::TooLarge => "request payload too large",
            ShareError::ServerError(e) => &e,
            ShareError::ParseFailure(e) => &e,
        }
    }
}

impl std::fmt::Display for ShareError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

#[derive(Debug)]
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
    #[serde(default)]
    exp: i64,
    /// When this url was created
    #[serde(default)]
    crt: i64,
    /// Url this redirects to
    url: String,
    /// Has this token expired
    #[serde(default)]
    expired: bool,
    ///The token that the custom url uses
    token: String,
}

impl Default for UrlID {
    fn default() -> Self {
        UrlID {
            id: None,
            exp: std::i64::MAX,
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

    pub fn set_exp(mut self, exp: &i64) -> Self {
        self.exp = exp.to_owned();
        self
    }

    pub fn set_dest_url(mut self, url: &str) -> Self {
        self.url = url.to_owned();
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

    pub fn get_token(&self) -> &str {
        &self.token
    }

    pub fn get_id(&self) -> Option<i64> {
        self.id
    }

    pub fn get_shorten_url(&self) -> String {
        //TODO make this return a proper URL, just not the token.
        format!("http://127.0.0.1/{}", self.token)
    }
}

impl crate::database::FromDatabase for UrlID {
    type Error = rusqlite::Error;
    fn from_database(row: &rusqlite::Row<'_> ) -> Result<UrlID, rusqlite::Error> {
        //SAFTEY: These should be safe, as the types with unwraps are disallowed from being null in the schema of the db.
        Ok(UrlID {
            id: row.get(0).unwrap_or(None),
            exp: row.get(1).unwrap(),
            crt: row.get(2).unwrap(),
            url: row.get(3).unwrap(),
            expired: row.get(4).unwrap(),
            token: row.get(5).unwrap(),
        })
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for UrlID {
    type Error = ShareError;
    async fn from_data(req: &'r rocket::request::Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        //Ensure correct content type
        let share_ct = ContentType::new("application", "json");
        if req.content_type() != Some(&share_ct) {
            return Failure((Status::UnsupportedMediaType, ShareError::ContentType));
        }

        let limit = 1024.bytes(); //Set the maximum size we'll unwrap
        //Read the data
        let string = match data.open(limit).into_string().await {
            Ok(string) if string.is_complete() => string.into_inner(),
            Ok(_) => return Failure((Status::PayloadTooLarge, ShareError::TooLarge)),
            Err(e) => return Failure((Status::InternalServerError, ShareError::ServerError(e.to_string()))),
        };
        
        let string = rocket::request::local_cache!(req, string);

        // Attempt to parse the string with serde into our struct
        let share: UrlID = match serde_json::from_str(string) {
            Ok(share) => share,
            Err(e) => return Failure((Status::BadRequest, ShareError::ParseFailure(e.to_string()))),
        };

        Success(share)
    }
}

