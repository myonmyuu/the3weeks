CREATE TABLE IF NOT EXISTS sessions (
	session_id	UUID
,	user_id		INTEGER REFERENCES users(id)
,	expires_at	TIMESTAMPTZ NOT NULL
,	created_at	TIMESTAMPTZ DEFAULT now()
,	ip_address	VARCHAR(64)

,	PRIMARY KEY (session_id, user_id)
);
