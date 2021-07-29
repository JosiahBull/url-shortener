//! The url id share, representing a valid shortened url object, and all information related to it.
use rocket::outcome::Outcome::*;
use crate::common::*;
use crate::database::{SharesDbConn, Search, update_database};
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket_sync_db_pools::rusqlite;
use rocket::http::{Status, ContentType};
use serde::{Serialize, Deserialize};
use rand::Rng;

//// Helper Functions (mostly for url generation) ////

///A 62-char alphabet, which is used for our base conversion into the shortened url.
const ALPHABET: &[char] = &['0', '1', '2', '3', '4', '5', '6','7','8','9','a','b','c','d','e','f','h','i','j','k','l','m','n','o','p','q','r','s','t','u','v','w','x','y','z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z'];

///The minimum length of a token, in chars. We will automatically add further chars to reach this length on the url.
const TOKEN_MIN_LENGTH_CHARS: usize = 6;

///The delim char between the "real" number, and the fake.
const DELIM_CHAR: char = 'g';

///Convert a base 10 number to a base 62 number
fn base_10_to_62(mut id: i64, alphabet: &[char]) -> String {
        //Converting from base 10 (id), to base 62.
        let mut result: String = String::default();
        loop {
            result.insert(0, alphabet[(id % 62) as usize]);
            id /= 36;
            if id <= 1 {
                break;
            }
        }
        result
}

///Add extra length to a string if it doesn't meet the minimum length 
fn normalize_length(mut input: String, min_length: usize, alphabet: &[char], delim: char) -> String {
    let mut rng = rand::thread_rng();
    if input.len() < min_length {
        input.push(delim);
        for _ in 0..(min_length-input.len()) {
            input.push(alphabet[rng.gen_range(0..62)]);
        }
    }
    input
}

///// Error Structs /////

/// An enum representing the error states that can occur with UrlID
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlIDError {
    ContentType,
    TooLarge,
    ServerError(String),
    ParseFailure(String),
    IdError,
    NoToken,
    DatabaseError(String),
}

impl From<UrlIDError> for String {
    fn from(err: UrlIDError) -> String {
        match err {
            UrlIDError::ContentType => "incorrect content-type provided on request".into(),
            UrlIDError::TooLarge => "request payload too large".into(),
            UrlIDError::ServerError(e) => e,
            UrlIDError::ParseFailure(e) => e,
            UrlIDError::IdError => "attempted to access id, but was none value".into(),
            UrlIDError::NoToken => "attempted to access an inaccessible token".into(),
            UrlIDError::DatabaseError(e) => e,
        }
    }
}

impl std::error::Error for UrlIDError {
    fn description(&self) -> &str {
        match self {
            UrlIDError::ContentType => "incorrect content-type provided on request",
            UrlIDError::TooLarge => "request payload too large",
            UrlIDError::ServerError(e) => e,
            UrlIDError::ParseFailure(e) => e,
            UrlIDError::IdError => "attempted to access id, but was none value",
            UrlIDError::NoToken => "attempted to access an inaccessible token",
            UrlIDError::DatabaseError(e) => e,
        }
    }
}

impl std::fmt::Display for UrlIDError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UrlIDError::ContentType => f.write_str("incorrect content-type provided on request"),
            UrlIDError::TooLarge => f.write_str("request payload too large"),
            UrlIDError::ServerError(e) => f.write_str(e),
            UrlIDError::ParseFailure(e) => f.write_str(e),
            UrlIDError::IdError => f.write_str("attempted to access id, but was none value"),
            UrlIDError::NoToken => f.write_str("attempted to access an inaccessible token"),
            UrlIDError::DatabaseError(e) => f.write_str(e),
        }
    }
}

/////  Data Structs  /////

///A new url id before it gets transformed to a UrlId internally, only used when parsing from Json.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct NewUrlID {
    url: String,
    exp: Option<i64>
}

/// This struct represents a valid url ID
#[derive(Debug, Clone, Eq, PartialEq)]
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
    ///Creates a new UrlID with a given url.
    #[allow(dead_code)]
    pub fn new(url: String) -> Self {
        UrlID {
            url,
            .. Default::default()
        }
    }

    ///Set the expiry, can be chained.
    #[allow(dead_code)]
    pub fn set_exp(mut self, exp: &i64) -> Self {
        self.exp = exp.to_owned();
        self
    }

    ///Set the destination url of this shortened link. Can be chained with other sets.
    #[allow(dead_code)]
    pub fn set_dest_url(mut self, url: &str) -> Self {
        self.url = url.to_owned();
        self
    }

    ///Get the destination url of this shortened link. 
    pub fn get_dest_url(&self) -> &str {
        &self.url
    }
    
    ///Get the time that this shortened link will expire.
    pub fn get_exp(&self) -> &i64 {
        &self.exp
    }

    ///Get the time this shortened link was created.
    pub fn get_crt(&self) -> &i64 {
        &self.crt
    }

    ///A boolean representing whether or not this shortened link has expired.
    pub fn is_expired(&self) -> &bool {
        &self.expired
    }

    ///Generates the unique identifier representing this shortened url. Can be chained with other requests, though this borrows mutably unlike others. 
    pub async fn generate_token(mut self, conn: &SharesDbConn) -> Result<UrlID, UrlIDError> {
        if self.id.is_none() {
            return Err(UrlIDError::IdError);
        }
        let token = base_10_to_62(self.id.expect("Id was none"), ALPHABET);
        self.token = Some(normalize_length(token, TOKEN_MIN_LENGTH_CHARS, ALPHABET, DELIM_CHAR));
        update_database(&conn, Search::Id(self.id.expect("Id was none")), self.clone()).await?;
        Ok(self)
    }

    ///Get the unique token representing this url.
    pub fn get_token(&self) -> Result<String, UrlIDError> {
        if let Some(token) = &self.token {
            return Ok(token.to_owned());
        }
        Err(UrlIDError::NoToken)
    }

    ///Get the shortened link associated with this URL. May create an error if the token has not been generated yet.
    pub fn get_shortened_link(&self) -> Result<String, UrlIDError> {
        if self.token.is_none() {
            return Err(UrlIDError::NoToken);
        }
        Ok(format!("http://{}/{}", crate::SERVER_DOMAIN, self.token.as_ref().unwrap()))
    }

    ///Get the ID associated with this shortened url.
    pub fn get_id(&self) -> &Option<i64> {
        &self.id
    }
}

impl std::convert::From<UrlIDError> for (rocket::http::Status, std::string::String) {
    fn from(err: UrlIDError) -> (rocket::http::Status, std::string::String) {
        // (rocket::http::Status::new(500), err.to_string())
        match err {
            UrlIDError::ServerError(e) => (rocket::http::Status::new(500), e),
            UrlIDError::DatabaseError(e) => (rocket::http::Status::new(500), e),
            UrlIDError::ParseFailure(e) => (rocket::http::Status::new(500), e),
            _ => (rocket::http::Status::new(400), "Bad Request".into()),
        }
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
    type Error = UrlIDError;
    async fn from_data(req: &'r rocket::request::Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        //Ensure correct content type
        let share_ct = ContentType::new("application", "json");
        if req.content_type() != Some(&share_ct) {
            return Failure((Status::UnsupportedMediaType, UrlIDError::ContentType));
        }

        let limit = 1024.bytes(); //Set the maximum size we'll unwrap
        //Read the data
        let string = match data.open(limit).into_string().await {
            Ok(string) if string.is_complete() => string.into_inner(),
            Ok(_) => return Failure((Status::PayloadTooLarge, UrlIDError::TooLarge)),
            Err(e) => return Failure((Status::InternalServerError, UrlIDError::ServerError(e.to_string()))),
        };
        
        let string = rocket::request::local_cache!(req, string);

        // Attempt to parse the string with serde into our struct
        let share: NewUrlID = match serde_json::from_str(string) {
            Ok(share) => share,
            Err(e) => return Failure((Status::BadRequest, UrlIDError::ParseFailure(e.to_string()))),
        };

        Success(UrlID {
            url: share.url,
            exp: share.exp.unwrap_or(std::i64::MAX), //Note it's not very idomatic to have this defined in multiple places (both here and default), might pay to wrap in enum then reuse?
            .. Default::default()
        })
    }
}