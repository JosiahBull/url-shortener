//! Functions and structs needed for interaction with the sqlite database.
use crate::url_id::{UncommittedUrlID, UrlID, UrlIDError};
use crate::users::{NewUser, User};
use rocket_sync_db_pools::database;
use rocket_sync_db_pools::rusqlite::{self, params};

/// A shared database
#[doc(hidden)]
#[database("sqlite_shares")]
pub struct SharesDbConn(rusqlite::Connection);

/// An enum representing the database error states
#[derive(Debug)]
pub enum DatabaseError {
    UrlIDError(UrlIDError),
    DoesNotExist,
    UnableToContact,
    SqlError(String),
    InsertError(String),
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(error: rusqlite::Error) -> DatabaseError {
        DatabaseError::SqlError(error.to_string())
    }
}

impl From<DatabaseError> for String {
    fn from(err: DatabaseError) -> String {
        match err {
            DatabaseError::DoesNotExist => "not found in database".to_string(),
            DatabaseError::UrlIDError(s) => format!(
                "a problem occurred with a share when interacting with the database: {}",
                s
            ),
            DatabaseError::UnableToContact => "failed to connect to the database".to_string(),
            DatabaseError::SqlError(s) => format!(
                "an sql error occurred when interfacing with the database: {}",
                s
            ),
            DatabaseError::InsertError(s) => format!(
                "an error occurred attempting to add a new share to the database: {}",
                s
            ),
        }
    }
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DatabaseError::DoesNotExist => f.write_str("not found in database"),
            DatabaseError::UrlIDError(s) => f.write_str(&format!(
                "a problem occurred with a share when interacting with the database: {}",
                s
            )),
            DatabaseError::UnableToContact => f.write_str("failed to connect to the database"),
            DatabaseError::SqlError(s) => f.write_str(&format!(
                "an sql error occurred when interfacing with the database: {}",
                s
            )),
            DatabaseError::InsertError(s) => f.write_str(&format!(
                "an error occurred attempting to add a new share to the database: {}",
                s
            )),
        }
    }
}

impl From<DatabaseError> for UrlIDError {
    fn from(err: DatabaseError) -> UrlIDError {
        UrlIDError::DatabaseError(err.to_string())
    }
}

impl std::convert::From<DatabaseError> for (rocket::http::Status, std::string::String) {
    fn from(err: DatabaseError) -> (rocket::http::Status, std::string::String) {
        (rocket::http::Status::new(500), err.to_string())
    }
}

/// Specifies how to search for items in the database, with three options.
pub enum Search {
    Id(i64),
    #[allow(dead_code)]
    Url(String),
}

impl Search {
    /// Based on which variant of the enum you are using, generates the search term required to interface with the sqlite database.
    // Note, if adding to this function in the future, ensure to add '' around strings.
    fn get_search_term(self) -> String {
        match self {
            Search::Id(s) => format!("{} = {}", "id", s),
            Search::Url(s) => format!("{} = '{}'", "url", s),
        }
    }
    ///Run a search, returns the first result it finds in the database, or a DatabaseError if something goes wrong.
    pub async fn find_share(self, conn: &SharesDbConn) -> Result<Option<UrlID>, DatabaseError> {
        let search_result = search_share_database(conn, self).await?;
        if search_result.is_empty() {
            return Ok(None);
        }
        Ok(Some(search_result[0].clone())) //Assume first result is correct, user will use search::id() variant if exactness is important.
    }
}

/// Implementing a trait means that your struct can be parsed from a database row, or return an error.
pub trait FromDatabase<'a>: Sized {
    type Error: Send + std::fmt::Debug + Into<rocket_sync_db_pools::rusqlite::Error>;
    fn from_database(
        data: &'a rocket_sync_db_pools::rusqlite::Row<'a>,
    ) -> Result<Self, Self::Error>;
}

/// Setup the database. Creates the table(s) required if they do not already exist in the database.db file.
pub async fn setup(conn: &SharesDbConn) -> Result<(), DatabaseError> {
    conn.run(|c| {
        c.execute(
            "CREATE TABLE IF NOT EXISTS shares (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            exp BIGINT NOT NULL,
            crt BIGINT INT NOT NULL,
            url TEXT NOT NULL
        );",
            [],
        )?;

        c.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL,
                password TEXT NOT NULL,
                is_admin BOOLEAN NOT NULL
            );",
            [],
        )
    })
    .await?;
    Ok(())
}

/// Attempts to add a new share to the database. If successful, will return the added share (importantly) with an ID!
pub async fn add_share_to_database(
    conn: &SharesDbConn,
    data: UncommittedUrlID,
) -> Result<UrlID, DatabaseError> {
    let response: UrlID = conn.run(move |c| -> Result<UrlID, DatabaseError> {
        let tx = c.transaction().unwrap();
        tx.execute("
            INSERT INTO shares (exp, crt, url)
            VALUES (?1, ?2, ?3);
        ", params![
            data.get_exp(), data.get_crt(), data.get_dest_url()
        ]).expect("failed to create share in db!");
        let mut result_data: Vec<UrlID> = tx.prepare("SELECT * FROM shares ORDER BY id DESC LIMIT 1;")
        .and_then(|mut res: rusqlite::Statement| -> std::result::Result<Vec<UrlID>, rusqlite::Error> {
            res.query_map([], |row| {
                UrlID::from_database(row)
            }).unwrap().collect()
        }).unwrap();
        tx.commit().unwrap();
        //TODO Implement error handling here, lots of unwrap statements which could panic. At min should be exchanged for expect statements, or preferably proper handling.

        Ok(result_data.remove(0))
    }).await?;

    Ok(response)
}

pub async fn add_user_to_database<'a>(
    conn: &SharesDbConn,
    data: NewUser<'static>,
) -> Result<User<'static>, DatabaseError> {
    let response: User = conn.run(move |c| -> Result<User, DatabaseError> {
        let tx = c.transaction().unwrap();
        tx.execute("
            INSERT INTO users (username, password, is_admin)
            VALUES (?1, ?2, ?3);
        ", params![
            data.username, data.password, data.is_admin
        ]).expect("failed to create user in db!");

        let mut result_data: Vec<User> = tx.prepare("SELECT * FROM users ORDER BY id DESC LIMIT 1;")
        .and_then(|mut res: rusqlite::Statement| -> std::result::Result<Vec<User>, rusqlite::Error> {
            res.query_map([], |row| {
                User::from_database(row)
            }).unwrap().collect()
        }).unwrap();

        tx.commit().unwrap();

        Ok(result_data.remove(0))
    }).await?;

    Ok(response)
}

/// Attempts to remove a share from the database, based on the ID of the share.
pub async fn delete_share_from_database(conn: &SharesDbConn, id: i64) -> Result<(), DatabaseError> {
    conn.run(move |c| c.execute("DELETE FROM shares WHERE id = ?1;", params![id]))
        .await?;
    Ok(())
}

/// Attempts to remove a user from the database, based on the ID of the user.
pub async fn delete_user_from_database(conn: &SharesDbConn, id: i64) -> Result<(), DatabaseError> {
    conn.run(move |c| c.execute("DELETE FROM users WHERE id = ?1;", params![id]))
        .await?;
    Ok(())
}

/// This is a non-public function, utilised by Search.find_share(). It will search a database, matching against criteria. It returns a vec of possible elements which may match the query.
async fn search_share_database(
    conn: &SharesDbConn,
    search: Search,
) -> Result<Vec<UrlID>, DatabaseError> {
    let result = conn
        .run(move |c| {
            c.prepare(&format!(
                "Select * FROM shares WHERE {};",
                search.get_search_term()
            ))
            .and_then(
                |mut res: rusqlite::Statement| -> std::result::Result<Vec<UrlID>, rusqlite::Error> {
                    res.query_map([], |row| UrlID::from_database(row))
                        .unwrap()
                        .collect()
                },
            )
        })
        .await?;
    Ok(result)
}

/// Update an element in the database, replacing it with a new element.
#[allow(dead_code)]
pub async fn update_share_database(
    conn: &SharesDbConn,
    search: Search,
    new_share: UrlID,
) -> Result<(), DatabaseError> {
    let search_result = match search.find_share(conn).await? {
        Some(s) => s,
        None => return Err(DatabaseError::DoesNotExist),
    };

    conn.run(move |c| {
        //SAFETY: As we are searching by ID to update a share, we shouldn't ever update more than one UrlID at a time.
        c.execute(
            "
            UPDATE shares
            SET exp = ?1,
                crt = ?2,
                url = ?3
            WHERE
                id = ?6
            ;
        ",
            params![
                new_share.get_exp(),
                new_share.get_crt(),
                new_share.get_dest_url(),
                search_result.get_id()
            ],
        )
    })
    .await?;
    Ok(())
}
