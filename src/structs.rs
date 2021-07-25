use rocket::outcome::Outcome::*;
use crate::common::*;
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket_sync_db_pools::{rusqlite, database};
use rocket::http::{Status, ContentType};
use serde::{Serialize, Deserialize};
use rand::Rng;

//// Helper Functions (mostly for url generation) ////

///A 36-char alphabet, which is used for our base conversion into the shortened url. Note that this could be improved to base 62 by adding extra upper case chars if needed!
const ALPHABET: &[char] = &['0', '1', '2', '3', '4', '5', '6','7','8','9','a','b','c','d','e','f','g','h','i','j','k','l','m','n','o','p','q','r','s','t','u','v','w','x','y','z'];

fn get_token(id: i64) -> String {
    let mut token = base_10_to_36(id, ALPHABET);
    
    token = apply_shift(token, ALPHABET, 5, 50);
    token
}   

fn base_10_to_36(mut id: i64, alphabet: &[char]) -> String {
        //Converting from base 10 (id), to base 36.
        let mut result: String = String::default();
        loop {
            result.insert(0, alphabet[(id % 36) as usize]);
            id = id/36;
            if id <= 1 {
                break;
            }
        }
        result
}

fn get_char_position(letter: char, alphabet: &[char]) -> i64 {
    let mut counter: i64 = 0;
    for comp in alphabet.iter() {
        if letter == *comp {
            return counter;
        }
        counter += 1;
    }
    panic!("Finding char position failed!");
}
///When passed to the add_with_overflow function, determines what action it should take in the event of an overflow.
#[derive(PartialEq, Eq)]
enum OverflowStatus {
    OverFlowToZero,
    OverFlowToMin,
}
///Attempts to add two numbers together, with a limit. In the event the new number overflows the limit, carrys out some action as defined by the overflow status
fn add_with_overflow(num: usize, num2: usize, lim: usize, overflow_to_zero: OverflowStatus) -> usize {
    let full_addition = num + num2;
    if full_addition <= lim {
        return full_addition;
    }
    if overflow_to_zero == OverflowStatus::OverFlowToZero {
        return full_addition - lim;
    }
    return std::usize::MIN + full_addition - lim;
}
///Apply a shift in each char of a generated url, creating some randomness.
fn apply_shift(input: String, alphabet: &[char], shift_min: usize, shift_max: usize) -> String {
    let mut result = String::default();
    let mut counter = 0;
    let mut rng = rand::thread_rng();
    for curr in input.chars() {
        let pos = get_char_position(curr, alphabet);
        let new_pos = add_with_overflow(pos as usize, rng.gen_range(shift_min..shift_max+1), 36, OverflowStatus::OverFlowToZero);
        counter += 1;
        result.push(alphabet[new_pos as usize])
    }
    result
}


///// Error Structs /////

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

