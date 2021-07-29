//! The url id share, representing a valid shortened url object, and all information related to it.
use rocket::outcome::Outcome::*;
use crate::common::*;
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket_sync_db_pools::rusqlite;
use rocket::http::{Status, ContentType};
use serde::{Serialize, Deserialize};
use rand::Rng;

//// Helper Functions (mostly for url generation) ////

///A 62-char alphabet, which is used for our base conversion into the shortened url.
pub const ALPHABET: &[char] = &['0', '1', '2', '3', '4', '5', '6','7','8','9','a','b','c','d','e','f','h','i','j','k','l','m','n','o','p','q','r','s','t','u','v','w','x','y','z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z'];

///The minimum length of a token, in chars. We will automatically add further chars to reach this length on the url.
const TOKEN_MIN_LENGTH_CHARS: usize = 6;

///The delim char between the "real" number, and the fake.
pub const DELIM_CHAR: char = 'g';

///The base being used for conversion
const BASE: usize = 61;

///Convert a base 10 number to a base 61 number
fn base_10_to_61(mut id: i64, alphabet: &[char]) -> String {
    //Converting from base 10 (id), to base 61.
    let mut result: String = String::default();
    loop {
        result.insert(0, alphabet[(id % BASE as i64) as usize]);
        id /= BASE as i64;
        if id < 1 {
            break;
        }
    }
    result
}

///Find position of a char in a string, panics upon failure to find the given char.
fn get_char_position(letter: char, alphabet: &[char]) -> i64 {
    for (counter, comp) in alphabet.iter().enumerate() {
        if letter == *comp {
            return counter as i64;
        }
    }
    panic!("Finding char position failed!");
}

///Convert a base 61 number to a base 10 number
pub fn base_61_to_10(token: String, alpha: &[char]) -> i64 {
    let mut result = 0;
    let mut counter = 0;
    loop {
        result = BASE as i64 * result + get_char_position(token.chars().nth(counter).unwrap(), alpha);
        counter += 1;
        if counter >= token.len() {
            break;
        }
    }
    result
}

#[test]
fn test_base_conversion() {
    let input_max = 1000;
    for i in 0..input_max {
        let token: String = base_10_to_61(i, ALPHABET);
        let id: i64 = base_61_to_10(token.clone(), ALPHABET);
        assert_eq!(i, id);
    }
}

///Add extra length to a string if it doesn't meet the minimum length 
fn normalize_length(mut input: String, min_length: usize, alphabet: &[char], delim: char) -> String {
    let mut rng = rand::thread_rng();
    if input.len() < min_length {
        input.push(delim);
        for _ in 0..(min_length-input.len()) {
            input.push(alphabet[rng.gen_range(0..BASE)]);
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

///A UrlId before it has been committed to the database.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct UncommittedUrlID {
    url: String,
    exp: Option<i64>,
    crt: Option<i64>
}

impl UncommittedUrlID {
    ///Get the destination url of this shortened link. 
    pub fn get_dest_url(&self) -> &str {
        &self.url
    }
    
    ///Get the time that this shortened link will expire.
    pub fn get_exp(&self) -> i64 {
        self.exp.unwrap()
    }

    ///Get the time this shortened link was created.
    pub fn get_crt(&self) -> i64 {
        self.crt.unwrap()
    }
}

/// This struct represents a valid url ID
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UrlID {
    ///ID
    id: i64,
    /// When this url expires
    exp: i64,
    /// When this url was created
    crt: i64,
    /// Url this redirects to
    url: String,
}

impl Default for UrlID {
    fn default() -> Self {
        UrlID {
            id: std::i64::MAX,
            exp: std::i64::MAX,
            crt: get_time_seconds(),
            url: String::default(),
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

    #[allow(dead_code)]
    ///Get the destination url of this shortened link. 
    pub fn get_dest_url(&self) -> &str {
        &self.url
    }

    #[allow(dead_code)]
    ///Get the time that this shortened link will expire.
    pub fn get_exp(&self) -> &i64 {
        &self.exp
    }

    #[allow(dead_code)]
    ///Get the time this shortened link was created.
    pub fn get_crt(&self) -> &i64 {
        &self.crt
    }

    ///Generates the unique identifier representing this shortened url. Can be chained with other requests, though this borrows mutably unlike others. 
    pub fn generate_token(&self) -> String {
        let token = base_10_to_61(self.id, ALPHABET);
        normalize_length(token, TOKEN_MIN_LENGTH_CHARS, ALPHABET, DELIM_CHAR)
    }

    ///Get the shortened link associated with this URL. May create an error if the token has not been generated yet.
    pub fn get_shortened_link(&self) -> String {
        format!("http://{}/{}", crate::SERVER_DOMAIN, self.generate_token())
    }

    ///Get the ID associated with this shortened url.
    pub fn get_id(&self) -> &i64 {
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
            id: row.get(0).unwrap(),
            exp: row.get(1).unwrap(),
            crt: row.get(2).unwrap(),
            url: row.get(3).unwrap(),
        })
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for UncommittedUrlID {
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
        let mut share: UncommittedUrlID = match serde_json::from_str(string) {
            Ok(share) => share,
            Err(e) => return Failure((Status::BadRequest, UrlIDError::ParseFailure(e.to_string()))),
        };
        if share.exp.is_none() {
            share.exp = Some(std::i64::MAX) //Note it's not very idomatic to have this defined in multiple places (both here and default), might pay to wrap in enum then reuse?
        }
        share.crt = Some(get_time_seconds());
        Success(share)
    }
}