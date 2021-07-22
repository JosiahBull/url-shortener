use crate::structs::{DatabaseError, SharesDbConn, UrlID};
use rocket_sync_db_pools::rusqlite::{self, params};
//Only use tokio in dev mode to run async tests

pub trait Searchable {
    fn select(&self) -> String; 
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

pub async fn add_to_database(conn: &SharesDbConn, data: UrlID) -> Result<(), DatabaseError> {
    let token = data.get_token().unwrap().to_owned();
    conn.run(move |c| {
        c.execute("
        INSERT INTO shares (exp, crt, url, expired, token)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ;", params![data.get_exp(), data.get_crt(), data.get_dest_url(), data.is_expired(), token])
    }).await?;
    Ok(())
}

pub async fn search_database<T>(conn: &SharesDbConn, search: T) -> Result<Option<Vec<UrlID>>, DatabaseError> 
where T: Searchable + Send + Sync + 'static {
    let result = conn.run(move |c| {
        c.prepare(&format!("Select * FROM shares WHERE {}", search.select()))
        .and_then(|mut res: rusqlite::Statement| -> std::result::Result<Vec<UrlID>, rusqlite::Error> {
            res.query_map([], |row| {
                Ok(UrlID::from_database(row)?)
            }).unwrap().collect()
        })
    }).await?;
    if result.is_empty() {
        return Ok(None);
    }
    Ok(Some(result))
}

pub async fn edit_share<T: Searchable>(conn: &SharesDbConn, search: T, new_item: UrlID) -> Result<(), DatabaseError> {
    //TODO
    Ok(())
}

pub async fn remove_share<T: Searchable>(conn: &SharesDbConn, search: T) -> Result<(), DatabaseError> {
    //TODO
    Ok(())
}