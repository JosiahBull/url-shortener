//! Functions and structs needed for interaction with the sqlite database.
use crate::url_id::{UrlIDError, SharesDbConn, UrlID};
use rocket_sync_db_pools::rusqlite::{self, params};

///An enum representing the database error states
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
        return match err {
            // DatabaseError::A(s) => return format!("Database Error: {}", s)
            DatabaseError::DoesNotExist => "not found in database".to_string(),
            DatabaseError::UrlIDError(s) => format!("a problem occured with a share when interacting with the database: {}", s.to_string()),
            DatabaseError::UnableToContact => "failed to connect to the database".to_string(),
            DatabaseError::SqlError(s) => format!("an sql error occured when interfacing with the database: {}", s),
            DatabaseError::InsertError(s) => format!("an error occured attempting to add a new share to the databse: {}", s),
        } 
    }
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            // DatabaseError::A(s) => return format!("Database Error: {}", s)
            DatabaseError::DoesNotExist => f.write_str("not found in database"),
            DatabaseError::UrlIDError(s) => f.write_str(&format!("a problem occured with a share when interacting with the database: {}", s.to_string())),
            DatabaseError::UnableToContact => f.write_str("failed to connect to the database"),
            DatabaseError::SqlError(s) => f.write_str(&format!("an sql error occured when interfacing with the database: {}", s)),
            DatabaseError::InsertError(s) => f.write_str(&format!("an error occured attempting to add a new share to the databse: {}", s)),
        } 
    }
}

impl From<DatabaseError> for UrlIDError {
    fn from(err: DatabaseError) -> UrlIDError {
        UrlIDError::DatabaseError(err.to_string())
    }
}

///Specifies how to search for items in the database, with three options.
pub enum Search {
    Id(i64),
    #[allow(dead_code)]
    Url(String),
    Token(String),
}

impl Search {
    ///Based on which variant of the enum you are using, generates the search term required to interface with the sqlite database.
    // Note, if adding to this function in the future, ensure to add '' around strings.
    fn get_search_term(self) -> String {
        match self {
            Search::Id(s) => format!("{} = {}", "id", s),
            Search::Url(s) => format!("{} = '{}'", "url", s),
            Search::Token(s) => format!("{} = '{}'", "token", s),
        }
    }
    ///Run a search, returns the first result it finds in the database, or a DatabaseError if something goes wrong.
    pub async fn find_share(self, conn: &SharesDbConn) -> Result<Option<UrlID>, DatabaseError> {
        let search_result = search_database(conn, self).await?;
        if search_result.is_empty() {
            return Ok(None);
        }
        Ok(Some(search_result[0].clone())) //Assume first result is correct, user will use search::id() variant if exactness is important.
    }
}

///Implementing a trait means that your struct can be parsed from a database row, or return an error.
pub trait FromDatabase: Sized {
    type Error: Send + std::fmt::Debug + Into<rocket_sync_db_pools::rusqlite::Error>;
    fn from_database(data: &rocket_sync_db_pools::rusqlite::Row<'_> ) -> Result<Self, Self::Error>;
}

///Setup the database. Creates the table(s) required if they do not already exist in the database.db file.
pub async fn setup(conn: &SharesDbConn) -> Result<(), DatabaseError> {
    conn.run(|c| {
        c.execute("CREATE TABLE IF NOT EXISTS shares (
            id INTEGER PRIMARY KEY,
            exp BIGINT NOT NULL,
            crt BIGINT INT NOT NULL,
            url TEXT NOT NULL,
            expired BOOLEAN NOT NULL,
            token TEXT
        );", [])
    }).await?;
    Ok(())
}

///Attempts to add a new share to the database. If successful, will return the added share (importantly) with an ID!
pub async fn add_to_database(conn: &SharesDbConn, data: UrlID) -> Result<UrlID, DatabaseError> {
    let response: UrlID = conn.run(move |c| -> Result<UrlID, DatabaseError> {
        let tx = c.transaction().unwrap();
        tx.execute("
            INSERT INTO shares (exp, crt, url, expired, token)
            VALUES (?1, ?2, ?3, ?4, ?5);
        ", params![
            data.get_exp(), data.get_crt(), data.get_dest_url(), data.is_expired(), "".to_string()
        ]).expect("failed to create share in db!");
        let result_data: Vec<UrlID> = tx.prepare("SELECT * FROM shares ORDER BY id DESC LIMIT 1;")
        .and_then(|mut res: rusqlite::Statement| -> std::result::Result<Vec<UrlID>, rusqlite::Error> {
            res.query_map([], |row| {
                UrlID::from_database(row)
            }).unwrap().collect()
        }).unwrap();
        tx.commit().unwrap();
        //TODO Implement error handling here, lots of unwrap statements which could panic. At min should be exchanged for expect statements, or preferrably proper handling.

        Ok(result_data[0].clone())
    }).await?;

    Ok(response)
}

///This is a non-public function, utilised by Search.find_share(). It will search a database, matching against criteria. It returns a vec of possible elements which may match the query.
async fn search_database(conn: &SharesDbConn, search: Search) -> Result<Vec<UrlID>, DatabaseError> {
    let result = conn.run(move |c| {
        c.prepare(&format!("Select * FROM shares WHERE {};", search.get_search_term()))
        .and_then(|mut res: rusqlite::Statement| -> std::result::Result<Vec<UrlID>, rusqlite::Error> {
            res.query_map([], |row| {
                UrlID::from_database(row)
            }).unwrap().collect()
        })
    }).await?;
    Ok(result)
}

///Update an element in the database, replacing it with a new element.
pub async fn update_database(conn: &SharesDbConn, search: Search, new_share: UrlID) -> Result<(), DatabaseError> {
    let search_result = match search.find_share(conn).await? {
        Some(s) => s,
        None => return Err(DatabaseError::DoesNotExist),
    };
    
    conn.run(move |c| {
        //SAFTEY: As we are searching by ID to update a share, we shouldn't ever update more than one UrlID at a time.
        c.execute("
            UPDATE shares
            SET exp = ?1,
                crt = ?2,
                url = ?3,
                expired = ?4,
                token = ?5
            WHERE
                id = ?6
            ;
        ", params![
            new_share.get_exp(), 
            new_share.get_crt(), 
            new_share.get_dest_url(), 
            new_share.is_expired(), 
            new_share.get_token().unwrap(), //TODO
            search_result.get_id().unwrap() //TODO
        ])
    }).await?;
    Ok(())
}

///Remove a share from the database.
// WARNING: AS THIS FUNCTION IS UNUSED, IT IS ALSO UNTESTED.
#[allow(dead_code)]
pub async fn remove_from_database(conn: &SharesDbConn, search: Search) -> Result<(), DatabaseError> {
    let search_result = match search.find_share(conn).await? {
        Some(s) => s,
        None => return Err(DatabaseError::DoesNotExist),
    };
    conn.run(move |c| {
        c.execute("
        DELETE FROM shares
        WHERE
            id = ?1
        ", params![search_result.get_id()])
    }).await?;
    Ok(())
}