#![allow(clippy::unnecessary_lazy_evaluations)]

use std::borrow::Cow;

use rocket::{http::Status, outcome::Outcome, request::FromRequest, Request};
use rocket_sync_db_pools::rusqlite;

use crate::database::{self, FromDatabase};

fn hash_str(input: &str) -> String {
    let salt: [u8; 16] = rand::random();
    let config = argon2::Config::default();
    argon2::hash_encoded(input.as_bytes(), &salt, &config).unwrap()
}

#[derive(FromForm)]
pub struct NewUser<'a> {
    pub username: Cow<'a, str>,
    pub password: Cow<'a, str>,
}

impl NewUser<'_> {
    pub fn hash_internal(&mut self) {
        self.password = Cow::Owned(hash_str(&self.password));
    }

    pub fn compare_against(&self, pass: &str) -> bool {
        argon2::verify_encoded(pass, self.password.as_bytes()).unwrap()
    }
}

pub struct User<'a> {
    id: i32,
    username: Cow<'a, str>,
    password: Cow<'a, str>,
}

impl User<'_> {
    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_username(&self) -> &str {
        &self.username
    }

    pub fn get_password(&self) -> &str {
        &self.password
    }

    pub fn new<'a>(id: i32, username: &'a str, password: &'a str, is_admin: bool) -> User<'a> {
        User {
            id,
            username: Cow::Borrowed(username),
            password: Cow::Borrowed(password),
        }
    }

    pub fn to_owned(&self) -> User<'static> {
        User {
            id: self.id,
            username: Cow::Owned(self.username.clone().into_owned()),
            password: Cow::Owned(self.password.clone().into_owned()),
        }
    }
}

impl<'a> FromDatabase<'a> for User<'static> {
    type Error = rusqlite::Error;
    fn from_database(row: &'a rusqlite::Row<'a>) -> Result<Self, Self::Error> {
        //SAFETY: These should be safe, as the types with unwraps are disallowed from being null in the schema of the db.
        Ok(User {
            id: row.get(0).unwrap(),
            username: Cow::Owned(row.get(1).unwrap()),
            password: Cow::Owned(row.get(2).unwrap()),
        })
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for User<'a> {
    type Error = ();

    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, (Status, Self::Error), ()> {
        let db = request.guard::<database::SharesDbConn>().await.unwrap();
        let cookies = request.cookies();
        let cookie = cookies.get_private("user_token");
        if let Some(cookie) = cookie {
            let token = cookie.value();
            //TODO
        }
        Outcome::Forward(())
    }
}
