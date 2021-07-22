use crate::structs::{DatabaseError, SharesDbConn};

pub async fn setup(conn: &SharesDbConn) -> Result<bool, DatabaseError>{
    conn.run(|c| {
        c.execute("CREATE TABLE IF NOT EXIST shares (
            id MEDIUMINT(255) PRIMARY KEY,
            exp INTEGER(255) NOT NULL,
            crt INTEGER(255) NOT NULL,
            url TEXT NOT NULL,
            expired BOOLEAN NOT NULL DEFAULT 'f',
            token TEXT NOT NULL
        )", [])
    }).await?;

    Ok(true)
}