use std::{ffi::OsStr, fs::DirEntry, path::{Path, PathBuf}};

use sqlx::{Pool, Postgres};

use super::prelude::*;

mod consts {
	pub const HIERARCHIAL_PATH_PARTS_COUNT: usize = 2;
	pub const VFS_PATH: &str = "./vfs";
	pub const VFS_PATH_ROOT: &str = const_format::concatcp!(VFS_PATH, "/root");
}

#[derive(Debug, Clone)]
pub struct FileRef {
	pub path: PathBuf,
}
impl From<DirEntry> for FileRef {
	fn from(value: DirEntry) -> Self {
		Self { path: value.path() }
	}
}
impl From<&DirEntry> for FileRef {
	fn from(value: &DirEntry) -> Self {
		Self { path: value.path() }
	}
}
impl FileRef {
	pub fn move_file(self, path: impl AsRef<Path>) -> Result<Self, VFSError> {
		let path = path.as_ref();
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent).map_err(VFSError::Io)?;
		}
		std::fs::rename(self.path, path).map_err(VFSError::Io)?;
		Ok(Self {
			path: path.to_path_buf()
		})
	}
}

pub fn get_hierarchial_hash_path(uuid: impl Into<String>) -> PathBuf {
	let uuid: String = uuid.into();
	let mut current = uuid.clone();
	let mut path = PathBuf::new();
	path.push(consts::VFS_PATH_ROOT);

	for _ in 0..consts::HIERARCHIAL_PATH_PARTS_COUNT {
		let remaining = current.split_off(2);
		path.push(current.clone());
		current = remaining;
	}

	path.push(uuid);
	path
}

async fn ensure_vfs_root(
	db_pool: &Pool<Postgres>,
) -> Result<uuid::Uuid, VFSError> {
	create_vfs_node(db_pool, "root".to_string(), None).await
}

async fn create_vfs_closures(
	db_pool: &Pool<Postgres>,
	id: uuid::Uuid,
) -> Result<(), VFSError> {
	let mut tx = db_pool.begin()
		.await
		.map_err(VFSError::Sql)?;
	
	let node = sqlx::query!("
		SELECT *
		FROM vfs_nodes
		WHERE id = $1
		;",
		id
	)
		.fetch_one(&mut *tx)
		.await
		.map_err(VFSError::Sql)?
	;
	sqlx::query!("
		-- delete previous closures (except self reference)
		DELETE FROM node_closures
		WHERE descendant = $1
		  AND ancestor != $1
		;",
		node.id
	)
		.execute(&mut *tx)
		.await
		.map_err(VFSError::Sql)?
	;
	sqlx::query!("
		--ensure self reference exists
		INSERT INTO node_closures (ancestor, descendant, depth)
		VALUES ($1, $1, 0)
		ON CONFLICT DO NOTHING
		;",
		node.id
	)
		.execute(&mut *tx)
		.await
		.map_err(VFSError::Sql)?
	;
	sqlx::query!("
		-- Insert all inherited paths from its ancestors
		INSERT INTO node_closures (ancestor, descendant, depth)
		SELECT
			ancestor,	-- ancestor of P
			$1,			-- the new node
			depth + 1	-- one level deeper
		FROM node_closures
		WHERE descendant = $2; -- 'p'
		;",
		node.id,
		node.parent_id
	)
		.execute(&mut *tx)
		.await
		.map_err(VFSError::Sql)?
	;

	tx.commit()
		.await
		.map_err(VFSError::Sql)
}

async fn create_vfs_node(
	db_pool: &Pool<Postgres>,
	name: String,
	parent: Option<uuid::Uuid>,
) -> Result<uuid::Uuid, VFSError> {
	println!("creating vfs node '{name}', parent: {parent:?}");
	let existing_node = sqlx::query!("
		SELECT id
		FROM vfs_nodes
		WHERE (
			node_name = $1
		  	AND (
		  		parent_id = $2
			 OR parent_id IS NULL
			)
		)
		;",
		name,
		parent
	)
		.fetch_optional(db_pool)
		.await
		.map_err(VFSError::Sql)?
	;

	let Some(node) = existing_node else {
		let node = sqlx::query!("
			INSERT INTO vfs_nodes
				(parent_id, node_name)
			VALUES
				($1, $2)
			RETURNING
				id
			;",
			parent,
			name
		)
			.fetch_one(db_pool)
			.await
			.map_err(VFSError::Sql)?
		;

		create_vfs_closures(db_pool, node.id).await?;
		
		return Ok(node.id);
	};

	Ok(node.id)
}

/// create the needed vfs path, returning the last node's id
pub async fn ensure_vfs_path(
	db_pool: &Pool<Postgres>,
	vfs_path: PathBuf,
) -> Result<uuid::Uuid, VFSError> {
	println!("creating vfs path '{vfs_path:?}'");
	let mut current = ensure_vfs_root(db_pool).await?;
	
	for part in vfs_path.iter().map(OsStr::to_str) {
		let Some(part) = part else {
			println!("invalid osstr encountered in vfs path: '{vfs_path:?}'");
			continue;
		};
		if part.to_ascii_lowercase() == "root" {
			continue;
		}

		current = create_vfs_node(db_pool, part.to_string(), Some(current)).await?;
	}

	print!("path '{vfs_path:?}' created");
	Ok(current)
}

pub async fn commit_file_to_vfs(
	file: FileRef,
	db_pool: &Pool<Postgres>,
	vfs_path: Option<PathBuf>,
) -> Result<uuid::Uuid, VFSError> {
	// println!("committing file '{:?}'", file.path);
	let file_uuid = uuid::Uuid::new_v4();

	let path = get_hierarchial_hash_path(file_uuid);
	// path.add_extension(file.path.extension().unwrap_or(OsStr::new("")));
	let file = file.move_file(path)?;
	// println!("file moved to '{:?}'", file.path);

	let parent = if let Some(vfs_path) = vfs_path {
		ensure_vfs_path(db_pool, vfs_path).await?
	} else {
		ensure_vfs_root(db_pool).await?
	};

	

	todo!()
}

pub async fn init_vfs(
	db_pool: &Pool<Postgres>,
) -> Result<(), VFSError> {
	let _ = ensure_vfs_root(db_pool).await?;
	// ensure_vfs_path(db_pool, "a/b/c/d".into()).await?;
	Ok(())
}