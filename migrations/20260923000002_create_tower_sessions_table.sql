CREATE TABLE tower_sessions (
    id          VARCHAR(22) PRIMARY KEY,
    data        BYTEA NOT NULL,
    expiry_date TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_tower_sessions_expiry_date ON tower_sessions (expiry_date);
