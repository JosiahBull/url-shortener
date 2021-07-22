use crate::structs::{DatabaseError, SharesDbConn, UrlID};
use rocket_sync_db_pools::rusqlite::params;

pub trait Searchable {
    fn select(&self) -> String; 
}

pub async fn setup(conn: &SharesDbConn) -> Result<(), DatabaseError> {
    conn.run(|c| {
        c.execute("CREATE TABLE IF NOT EXISTS shares (
            id INTEGER PRIMARY KEY,
            exp BIGINT NOT NULL,
            crt BIGINT INT NOT NULL,
            url TEXT NOT NULL,
            expired BOOLEAN NOT NULL DEFAULT 'f',
            token TEXT
        );", [])
    }).await?;

    Ok(())
}

pub async fn add_to_database(conn: &SharesDbConn, data: UrlID) -> Result<(), DatabaseError> {
    let token = data.get_token().unwrap().to_owned();
    let result = conn.run(move |c| {
        c.execute("
        INSERT INTO shares (exp, crt, url, token)
        VALUES (?1, ?2, ?3, ?4)
        ;", params![data.get_exp(), data.get_crt(), data.get_dest_url(), token])
    }).await;
    println!("{:?}", result);
    Ok(())
}

pub async fn search_database<T: Searchable>(conn: &SharesDbConn, search: T) -> Result<Option<UrlID>, DatabaseError> {
    //TODO
    Ok(None)
}

pub async fn edit_share<T: Searchable>(conn: &SharesDbConn, search: T, new_item: UrlID) -> Result<(), DatabaseError> {
    //TODO
    Ok(())
}

pub async fn remove_share<T: Searchable>(conn: &SharesDbConn, search: T) -> Result<(), DatabaseError> {
    //TODO
    Ok(())
}