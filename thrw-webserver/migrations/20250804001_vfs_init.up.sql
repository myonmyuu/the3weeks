CREATE TABLE IF NOT EXISTS vfs_nodes(
	id			UUID PRIMARY KEY DEFAULT gen_random_uuid()
,	parent_id	UUID REFERENCES vfs_nodes(id) ON DELETE CASCADE
,	node_name	TEXT NOT NULL
,	is_folder	BOOLEAN NOT NULL
,	created_at	TIMESTAMPTZ DEFAULT now()
,	updated_at	TIMESTAMPTZ

,	CONSTRAINT	unique_name_per_folder UNIQUE(parent_id, node_name)
);
