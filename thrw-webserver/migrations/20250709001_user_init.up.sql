CREATE TABLE IF NOT EXISTS user_levels(
	level_id	SMALLINT PRIMARY KEY NOT NULL
,	level_name	TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users(
    id      	SERIAL PRIMARY KEY
,	email   	TEXT NOT NULL
,	pwhash  	TEXT NOT NULL
,	user_level	SMALLINT REFERENCES user_levels(level_id) NOT NULL
);

CREATE TABLE IF NOT EXISTS usersettings(
	id			INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE
,	strict_ip	BOOLEAN
);

INSERT INTO user_levels
	(level_id, level_name)
VALUES
	(3, 'user'),
	(5, 'admin')
;