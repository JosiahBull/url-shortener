use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use rocket::{http::Status, outcome::Outcome, request::FromRequest, Request};
use rocket_sync_db_pools::rusqlite;

use crate::database::{self, FromDatabase};

pub struct NewUser<'a> {
    pub username: Cow<'a, str>,
    pub password: Cow<'a, str>,
    pub is_admin: bool,
}

pub struct User<'a> {
    id: i32,
    username: Cow<'a, str>,
    password: Cow<'a, str>,
    is_admin: bool,
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

    pub fn is_admin(&self) -> bool {
        self.is_admin
    }

    pub fn new<'a>(id: i32, username: &'a str, password: &'a str, is_admin: bool) -> User<'a> {
        User {
            id,
            username: Cow::Borrowed(username),
            password: Cow::Borrowed(password),
            is_admin,
        }
    }

    pub fn to_owned(self) -> User<'static> {
        User {
            id: self.id,
            username: Cow::Owned(self.username.into_owned()),
            password: Cow::Owned(self.password.into_owned()),
            is_admin: self.is_admin,
        }
    }
}

impl<'a> FromDatabase<'a> for User<'static> {
    type Error = rusqlite::Error;
    fn from_database(row: &'a rusqlite::Row<'a>) -> Result<Self, Self::Error> {
        //SAFETY: These should be safe, as the types with unwraps are disallowed from being null in the schema of the db.
        Ok(User {
            id: row.get(0).unwrap(),
            // username: Cow::Borrowed(row.get_ref_unwrap(1).as_str().unwrap()),
            // password: Cow::Borrowed(row.get_ref_unwrap(2).as_str().unwrap()),
            username: Cow::Owned(row.get(1).unwrap()),
            password: Cow::Owned(row.get(2).unwrap()),
            is_admin: row.get(3).unwrap(),
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

pub struct AdminUser<'a>(User<'a>);

impl AdminUser<'_> {
    pub fn get_id(&self) -> i32 {
        self.0.get_id()
    }

    pub fn get_username(&self) -> &str {
        self.0.get_username()
    }

    pub fn get_password(&self) -> &str {
        self.0.get_password()
    }

    pub fn is_admin(&self) -> bool {
        self.0.is_admin()
    }

    pub fn new<'a>(id: i32, username: &'a str, password: &'a str, is_admin: bool) -> AdminUser<'a> {
        AdminUser(User::new(id, username, password, is_admin))
    }

    pub fn to_owned(self) -> AdminUser<'static> {
        AdminUser(self.0.to_owned())
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for AdminUser<'a> {
    type Error = ();

    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, (Status, Self::Error), ()> {
        let user = request.guard::<User>().await.unwrap();
        if user.is_admin() {
            Outcome::Success(AdminUser(user))
        } else {
            Outcome::Forward(())
        }
    }
}

impl<'a> Deref for AdminUser<'a> {
    type Target = User<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for AdminUser<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
