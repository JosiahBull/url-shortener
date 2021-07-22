use crate::structs::{DatabaseError, SharesDbConn, UrlID};

pub trait Searchable {
    fn select(&self) -> String; 
}

pub async fn setup(conn: &SharesDbConn) -> Result<(), DatabaseError> {
    conn.run(|c| {
        c.execute("CREATE TABLE IF NOT EXIST shares (
            id MEDIUMINT(255) PRIMARY KEY,
            exp INTEGER(255) NOT NULL,
            crt INTEGER(255) NOT NULL,
            url TEXT NOT NULL,
            expired BOOLEAN NOT NULL DEFAULT 'f',
            token TEXT
        )", [])
    }).await?;

    Ok(())
}

pub async fn add_to_database(data: &UrlID) -> Result<(), DatabaseError> {
    //TODO
    Ok(())
}

pub async fn search_database<T: Searchable>(search: T) -> Result<Option<UrlID>, DatabaseError> {
    //TODO
    Ok(None)
}