use crate::structs::{DatabaseError, SharesDbConn, UrlID};
use rocket_sync_db_pools::rusqlite::{self, params};
//Only use tokio in dev mode to run async tests

pub enum Search {
    Id(i64),
    Url(String),
    Token(String),
}

impl Search {
    fn get_search_term(self) -> String {
        match self {
            Search::Id(s) => format!("{} = {}", "id", s),
            Search::Url(s) => format!("{} = {}", "url", s),
            Search::Token(s) => format!("{} = {}", "token", s),
        }
    }

    async fn find_share(self, conn: &SharesDbConn) -> Result<UrlID, DatabaseError> {
        let search_result = search_database(conn, self).await?;
        if search_result.is_empty() {
            return Err(DatabaseError::A("Unable to find share to edit".into()));
        }
        Ok(search_result[0].clone()) //Assume first result is correct, user will use search::id() variant if exactness is important.
    }
}

pub trait FromDatabase: Sized {
    type Error: Send + std::fmt::Debug + Into<rocket_sync_db_pools::rusqlite::Error>;
    fn from_database(data: &rocket_sync_db_pools::rusqlite::Row<'_> ) -> Result<Self, Self::Error>;
}

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
                Ok(UrlID::from_database(row)?)
            }).unwrap().collect()
        }).unwrap();
        tx.commit().unwrap();
        //TODO Implement error handling here, lots of unwrap statements which could panic. At min should be exchanged for expect statements, or preferrably proper handling.

        Ok(result_data[0].clone())
    }).await?;

    Ok(response)
}

pub async fn search_database(conn: &SharesDbConn, search: Search) -> Result<Vec<UrlID>, DatabaseError> {
    let result = conn.run(move |c| {
        c.prepare(&format!("Select * FROM shares WHERE {};", search.get_search_term()))
        .and_then(|mut res: rusqlite::Statement| -> std::result::Result<Vec<UrlID>, rusqlite::Error> {
            res.query_map([], |row| {
                Ok(UrlID::from_database(row)?)
            }).unwrap().collect()
        })
    }).await?;
    Ok(result)
}

pub async fn update_database(conn: &SharesDbConn, search: Search, new_share: UrlID) -> Result<(), DatabaseError> {
    let search_result: UrlID = search.find_share(conn).await?;
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
            LIMIT 1;
        ", params![
            new_share.get_exp(), 
            new_share.get_crt(), 
            new_share.get_dest_url(), 
            new_share.is_expired(), 
            new_share.get_token().unwrap(), 
            search_result.get_id().unwrap()
        ])
    }).await?;
    Ok(())
}

pub async fn remove_from_database(conn: &SharesDbConn, search: Search) -> Result<(), DatabaseError> {
    let search_result = search.find_share(conn).await?;
    conn.run(move |c| {
        c.execute("
        DELETE FROM shares
        WHERE
            id = ?1
        ", params![search_result.get_id()])
    }).await?;
    Ok(())
}