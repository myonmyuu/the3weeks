CREATE TABLE IF NOT EXISTS sessions (
    session_id	UUID PRIMARY KEY,
    user_id		INTEGER REFERENCES users(id),
    expires_at	TIMESTAMPTZ,
    created_at	TIMESTAMPTZ DEFAULT now()
);
