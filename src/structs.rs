use rocket::outcome::Outcome::*;
use crate::common::*;
use rocket::request::FromParam;
use rocket::data::{self, Data, FromData, ToByteUnit};

#[derive(Debug)]
pub enum UrlParseError {

}

impl std::error::Error for UrlParseError {
    fn description(&self) -> &str {
        "Failed to parse url correctly."
    }
}

impl std::fmt::Display for UrlParseError {
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
            url: "".into(),
        }
    }
}

impl UrlID {
    fn new(url: String) -> Self {
        let mut def = Self::default();
        def.url = url;
        def
    }

    fn set_url(mut self, url: &str) -> Self {
        self.url = url.to_owned();
        self
    }

    fn set_exp(mut self, exp: &u64) -> Self {
        self.exp = exp.to_owned();
        self
    }

    fn get_shorten_url(self) -> String {
        "Not yet implemented".into()
    }
}

impl<'r> FromParam<'r> for UrlID {
    type Error = UrlParseError;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        Ok(UrlID::default())
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for UrlID {
    type Error = UrlParseError;
    async fn from_data(req: &'r rocket::request::Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        //This is expected to be a conversion from a post request as a new friend
        //Convert from serde


        //Attempt to find the requested file
        Success(UrlID {
            exp: std::u64::MAX,
            crt: std::u64::MAX,
            url: "www.google.com".into(),
        })
    }
}