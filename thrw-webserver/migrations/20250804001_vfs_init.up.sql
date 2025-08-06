CREATE TABLE IF NOT EXISTS vfs_files(
	id			UUID PRIMARY KEY DEFAULT gen_random_uuid()
,	file_path	TEXT NOT NULL
,	file_size	BIGINT NOT NULL
,	file_type	TEXT NOT NULL CHECK (file_type IN ('audio', 'video', 'image', 'text'))
,	mime_type	TEXT
,	created_at	TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS image_files(
	id			UUID PRIMARY KEY REFERENCES vfs_files(id) ON DELETE CASCADE
,	width		INTEGER NOT NULL
,	height		INTEGER NOT NULL
,	color_depth	INTEGER
,	exif		JSONB
);

CREATE TABLE IF NOT EXISTS audio_files(
	id			UUID PRIMARY KEY REFERENCES vfs_files(id) ON DELETE CASCADE
,	thumbnail	UUID REFERENCES image_files(id) ON DELETE SET NULL
,	duration	FLOAT NOT NULL
,	bitrate		INTEGER
,	sample_rate	INTEGER
,	channels	INTEGER
);

CREATE TABLE IF NOT EXISTS video_files(
	id			UUID PRIMARY KEY REFERENCES vfs_files(id) ON DELETE CASCADE
,	thumbnail	UUID REFERENCES image_files(id) ON DELETE SET NULL
,	duration	FLOAT NOT NULL
,	resolution	TEXT
,	frame_rate	FLOAT
,	video_codec	TEXT
,	audio_codec	TEXT
);

CREATE TABLE IF NOT EXISTS vfs_nodes(
	id			UUID PRIMARY KEY DEFAULT gen_random_uuid()
,	parent_id	UUID REFERENCES vfs_nodes(id) ON DELETE CASCADE
,	node_name	TEXT NOT NULL
,	vfs_file	UUID REFERENCES vfs_files(id) ON DELETE CASCADE
,	created_at	TIMESTAMPTZ DEFAULT now()
,	updated_at	TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS node_closures(
	ancestor	UUID REFERENCES vfs_nodes(id)
,	descendant	UUID REFERENCES vfs_nodes(id)
,	depth		INTEGER
,	PRIMARY KEY	(ancestor, descendant)
);

