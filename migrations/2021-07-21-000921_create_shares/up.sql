-- Your SQL goes here
CREATE TABLE shares (
    id MEDIUMINT(255) PRIMARY KEY,
    exp INTEGER(255) NOT NULL,
    crt INTEGER(255) NOT NULL,
    url TEXT NOT NULL,
    expired BOOLEAN NOT NULL DEFAULT 'f',
    token TEXT NOT NULL
)