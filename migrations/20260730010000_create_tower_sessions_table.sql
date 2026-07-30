CREATE TABLE IF NOT EXISTS tower_sessions
(
    id          CHAR(22) CHARACTER SET ascii COLLATE ascii_bin PRIMARY KEY NOT NULL,
    data        BLOB                                                     NOT NULL,
    expiry_date TIMESTAMP(6)                                             NOT NULL,
    INDEX idx_tower_sessions_expiry_date (expiry_date)
);
