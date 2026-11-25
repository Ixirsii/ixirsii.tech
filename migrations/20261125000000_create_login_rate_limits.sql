CREATE TABLE login_rate_limits (
    scope           TEXT NOT NULL CHECK (scope IN ('client', 'account_client')),
    identity_hash   BYTEA NOT NULL CHECK (octet_length(identity_hash) = 32),
    attempts        TIMESTAMPTZ[] NOT NULL CHECK (cardinality(attempts) > 0),
    last_attempt_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (scope, identity_hash)
);
CREATE INDEX login_rate_limits_expiry ON login_rate_limits (scope, last_attempt_at);

-- There are no application reservations before the Unix epoch. Clamp very
-- large windows before constructing an interval so it cannot overflow.
CREATE FUNCTION login_window_start(as_of TIMESTAMPTZ, window_seconds BIGINT)
RETURNS TIMESTAMPTZ LANGUAGE SQL STABLE STRICT
RETURN as_of - make_interval(secs => least(window_seconds, ceil(extract(epoch FROM as_of))::BIGINT));

-- One call is one transaction. Existing identities lock only their own row.
-- New identities also serialize admission within their scope to enforce the cap.
CREATE FUNCTION reserve_login_attempt(
    p_scope TEXT, p_key BYTEA, p_limit BIGINT, p_window_seconds BIGINT, p_capacity BIGINT
) RETURNS BOOLEAN LANGUAGE plpgsql VOLATILE STRICT AS $$
DECLARE
    history TIMESTAMPTZ[];
    observed_at TIMESTAMPTZ;
    cutoff TIMESTAMPTZ;
BEGIN
    SELECT attempts INTO history FROM login_rate_limits
    WHERE scope = p_scope AND identity_hash = p_key FOR UPDATE;

    IF NOT FOUND THEN
        PERFORM pg_advisory_xact_lock(hashtext('ixirsii.tech/login-limits'), hashtext(p_scope));
        -- Another replica may have admitted this key while we waited.
        SELECT attempts INTO history FROM login_rate_limits
        WHERE scope = p_scope AND identity_hash = p_key FOR UPDATE;
    END IF;

    observed_at := clock_timestamp();
    cutoff := login_window_start(observed_at, p_window_seconds);
    IF history IS NULL THEN
        DELETE FROM login_rate_limits WHERE scope = p_scope AND last_attempt_at <= cutoff;
        IF (SELECT count(*) FROM login_rate_limits WHERE scope = p_scope) >= p_capacity THEN
            RETURN FALSE;
        END IF;
        INSERT INTO login_rate_limits (scope, identity_hash, attempts, last_attempt_at)
        VALUES (p_scope, p_key, ARRAY[observed_at], observed_at);
    ELSE
        history := ARRAY(SELECT t FROM unnest(history) AS t WHERE t > cutoff);
        IF cardinality(history) >= p_limit THEN
            RETURN FALSE;
        END IF;
        UPDATE login_rate_limits
        SET attempts = array_append(history, observed_at),
            last_attempt_at = greatest(last_attempt_at, observed_at)
        WHERE scope = p_scope AND identity_hash = p_key;
    END IF;
    RETURN TRUE;
END;
$$;
