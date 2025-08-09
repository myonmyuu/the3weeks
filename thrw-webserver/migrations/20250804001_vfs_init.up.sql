CREATE TABLE IF NOT EXISTS vfs_files(
	id			UUID PRIMARY KEY DEFAULT gen_random_uuid()
,	file_path	TEXT NOT NULL
,	file_size	BIGINT NOT NULL
,	file_type	TEXT NOT NULL CHECK (file_type IN ('audio', 'video', 'image', 'text'))
,	mime_type	TEXT
,	created_at	TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS image_files(
	id			UUID PRIMARY KEY REFERENCES vfs_files(id) ON DELETE CASCADE
,	width		SMALLINT NOT NULL
,	height		SMALLINT NOT NULL
,	codec_name	TEXT
,	pix_fmt		TEXT
);

CREATE TABLE IF NOT EXISTS audio_files(
	id				UUID PRIMARY KEY REFERENCES vfs_files(id) ON DELETE CASCADE
,	duration		FLOAT NOT NULL
,	codec_name		TEXT
,	bitrate			INTEGER
,	sample_format	TEXT
,	sample_rate		INTEGER
,	channels		INTEGER
);

CREATE TABLE IF NOT EXISTS video_files(
	id				UUID PRIMARY KEY REFERENCES vfs_files(id) ON DELETE CASCADE
,	duration		FLOAT NOT NULL
,	width			SMALLINT
,	height			SMALLINT
,	r_frame_rate	TEXT
,	avg_frame_rate	TEXT
,	video_codec		TEXT
,	audio_codec		TEXT
);

CREATE TABLE IF NOT EXISTS vfs_thumbs(
	id			UUID PRIMARY KEY REFERENCES vfs_files(id) ON DELETE CASCADE
,	thumbnail	UUID REFERENCES image_files(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS vfs_nodes(
	id			UUID PRIMARY KEY DEFAULT gen_random_uuid()
,	parent_id	UUID REFERENCES vfs_nodes(id) ON DELETE CASCADE
,	node_name	TEXT NOT NULL
,	vfs_file	UUID REFERENCES vfs_files(id) ON DELETE CASCADE
,	hide		BOOLEAN NOT NULL
,	created_at	TIMESTAMPTZ NOT NULL DEFAULT now()
,	updated_at	TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS node_closures(
	ancestor	UUID NOT NULL REFERENCES vfs_nodes(id) ON DELETE CASCADE
,	descendant	UUID NOT NULL REFERENCES vfs_nodes(id) ON DELETE CASCADE
,	depth		INTEGER NOT NULL
,	PRIMARY KEY	(ancestor, descendant)
);

