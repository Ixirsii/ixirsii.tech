-- Exactly one shared signing key, initialized by the application at startup.
-- Anonymous login forms never allocate database sessions.
CREATE TABLE login_csrf_key (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    key BYTEA NOT NULL CHECK (octet_length(key) = 64)
);
