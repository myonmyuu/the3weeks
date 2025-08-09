use std::{ffi::OsStr, fs::DirEntry, path::{Path, PathBuf}, str::FromStr};

use sqlx::{Pool, Postgres};
use tokio::try_join;

use crate::media::util::{FFProbeMediaOutput};

use super::prelude::*;

mod consts {
	pub const HIERARCHIAL_PATH_PARTS_COUNT: usize = 2;
	pub const VFS_DIR_PATH: &str = "vfsfiles";
}

pub struct VfsNode {
	pub id: uuid::Uuid,
	pub parent_id: Option<uuid::Uuid>,
	pub node_name: String,
	pub vfs_file: Option<uuid::Uuid>,
	pub created_at: chrono::DateTime<chrono::Utc>,
	pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
	pub hide: bool,
}

pub struct  VfsNodeCreateArgs {
	pub name: String,
	pub hide: bool,
}

fn get_site_folder() -> PathBuf {
	if cfg!(debug_assertions) {
		PathBuf::from("./site")
	} else {
		std::env::current_exe()
			.map(|mut p| {
				p.pop();
				p.pop();
				p.join("site")
			})
			.unwrap_or(PathBuf::from("."))	
	}
}

pub fn get_vfs_dir() -> PathBuf {
	PathBuf::from(consts::VFS_DIR_PATH)
}

#[derive(Debug, Clone)]
pub enum VFSFileType {
	Multimedia(FFProbeMediaOutput),
	Image(FFProbeMediaOutput),
	Text,
}
impl ToString for VFSFileType {
	fn to_string(&self) -> String {
		match self {
			VFSFileType::Multimedia(ffo) => {
				ffo.streams.iter()
					.find(|stream| matches!(stream.codec_type, crate::media::util::MediaCodecType::Video))
					.map(|stream| stream.codec_type.to_string())
					.unwrap_or(ffo.streams.first()
						.map(|stream| stream.codec_type.to_string())
						.unwrap_or("audio".to_string())
					)
			},
			VFSFileType::Image(_) => "image".to_string(),
			VFSFileType::Text => "text".to_string(),
		}
	}
}


#[derive(Debug, Clone)]
pub struct VfsFileData {
	pub name: String,
	pub file: FileRef,
	pub file_type: VFSFileType,
	pub hide: bool,
}
impl VfsFileData {
	pub fn move_file(&mut self, path: impl AsRef<std::path::Path>) -> Result<&FileRef, VFSError> {
		let new_path = self.file.clone().move_file(path)?;
		self.file = new_path.clone();
		Ok(&self.file)
	}

	pub fn join_name(&self, add: impl Into<String>) -> Result<Self, VFSError> {
		Ok(Self {
			name: self.name.clone() + add.into().as_str(),
			..self.clone()
		})
	}
}

#[derive(Debug, Clone)]
pub struct FileRef {
	pub path: PathBuf,
	pub file_size: i64,
}
impl From<DirEntry> for FileRef {
	fn from(value: DirEntry) -> Self {
		Self {
			path: value.path(),
			file_size: value.metadata().map(|md| md.len() as i64).unwrap_or(0)
		}
	}
}
impl From<&DirEntry> for FileRef {
	fn from(value: &DirEntry) -> Self {
		Self {
			path: value.path(),
			file_size: value.metadata().map(|md| md.len() as i64 ).unwrap_or(0)
		}
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
			path: path.to_path_buf(),
			..self
		})
	}
	pub fn delete_file(self) -> Result<(), VFSError> {
		std::fs::remove_file(self.path).map_err(VFSError::Io)
	}
}

fn try_get_field<T: Clone>(content: &Option<T>, name: &str) -> Result<T, VFSError> {
	content.clone().ok_or(VFSError::MediaMissingMetadata(name.to_string()))
}

pub fn get_hierarchial_hash_path(uuid: impl Into<String>) -> PathBuf {
	let uuid: String = uuid.into();
	let mut current = uuid.clone();
	let mut path = PathBuf::new();

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
	create_vfs_node_internal(
		db_pool,
		VfsNodeCreateArgs {
			name: "root".to_string(),
			hide: false
		},
		None
	).await
}

async fn update_vfs_closures(
	db_pool: &Pool<Postgres>,
	node_id: uuid::Uuid,
) -> Result<(), VFSError> {
	let mut tx = db_pool.begin()
		.await
		.map_err(VFSError::Sql)?;
	
	let node = sqlx::query!("
		SELECT *
		FROM vfs_nodes
		WHERE id = $1
		;",
		node_id
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

async fn get_vfs_node(
	db_pool: &Pool<Postgres>,
	name: String,
	parent: Option<uuid::Uuid>,
) -> Result<uuid::Uuid, VFSError> {
	sqlx::query!("
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
		.map(|rec| rec.id)
		.ok_or(VFSError::NotFound)
}

pub async fn set_thumbnail(
	db_pool: &Pool<Postgres>,
	file_id: uuid::Uuid,
	image_id: uuid::Uuid,
) -> Result<(), VFSError> {
	sqlx::query!("
		INSERT INTO vfs_thumbs
			(id, thumbnail)
		VALUES
			($1, $2)
		;",
		file_id,
		image_id
	)
		.execute(db_pool)	
		.await
		.map_err(VFSError::Sql)
		.map(|_| ())
}

pub async fn get_thumbnail(
	db_pool: &Pool<Postgres>,
	id: uuid::Uuid,
) -> Result<Option<(uuid::Uuid, String)>, VFSError> {
	let file_node = sqlx::query!("
		SELECT
			file.id				AS file_id,
			node.id				AS node_id,
			thumb.id			AS thumbnail_id,
			thumb_img.file_path	AS thumbnail_path
		FROM vfs_nodes	AS node
		JOIN vfs_files	AS file			ON file.id = node.vfs_file
		JOIN vfs_thumbs	AS thumb		ON thumb.id = file.id
		JOIN vfs_files	AS thumb_img	ON thumb_img.id = thumb.thumbnail
		WHERE node.id = $1
		;",
		id
	)
		.fetch_optional(db_pool)
		.await
		.map_err(VFSError::Sql)?
	;

	Ok(file_node.map(|data| (data.thumbnail_id, data.thumbnail_path)))
}

pub async fn get_vfs_node_data(
	db_pool: &Pool<Postgres>,
	id: uuid::Uuid,
) -> Result<VfsNode, VFSError> {
	sqlx::query_as!(
		VfsNode,
		"SELECT * FROM vfs_nodes
		WHERE id = $1
		;",
		id
	)
		.fetch_one(db_pool)
		.await
		.map_err(VFSError::Sql)
}

pub async fn get_pub_vfs_node(
	db_pool: &Pool<Postgres>,
	id: uuid::Uuid,
) -> Result<PubVfsNode, VFSError> {
	let node_data = get_vfs_node_data(db_pool, id).await?;
	Ok(PubVfsNode {
		id: node_data.id,
		name: node_data.node_name.clone(),
		path: get_vfs_path_to(db_pool, node_data.id).await?,
		node_type: node_data.vfs_file
			.map(|file_id| PubVfsNodeType::Audio)
			.unwrap_or(PubVfsNodeType::Folder),
		thumbnail: get_thumbnail(db_pool, id).await?.map(|th| th.1)
	})
}

pub async fn create_vfs_node_internal(
	db_pool: &Pool<Postgres>,
	args: VfsNodeCreateArgs,
	parent: Option<uuid::Uuid>,
) -> Result<uuid::Uuid, VFSError> {
	let Ok(node) = get_vfs_node(db_pool, args.name.clone(), parent).await else {
		println!("creating vfs node '{}', parent: {parent:?}", args.name);
		let node = sqlx::query!("
			INSERT INTO vfs_nodes
				(parent_id, node_name, hide)
			VALUES
				($1, $2, $3)
			RETURNING
				id
			;",
			parent,
			args.name,
			args.hide
		)
			.fetch_one(db_pool)
			.await
			.map_err(VFSError::Sql)?
		;

		update_vfs_closures(db_pool, node.id).await?;
		
		return Ok(node.id);
	};

	Ok(node)
}

/// create a cfs file (and the inner file type), returning the vfs_files.id
async fn create_vfs_file(
	db_pool: &Pool<Postgres>,
	file_data: VfsFileData,
) -> Result<uuid::Uuid, VFSError> {
	let path = file_data.file.path;
	let path = PathBuf::from("/")
		.join(path
			.strip_prefix(get_site_folder())
			.unwrap_or(&path)
		)
		.to_string_lossy()
		.into_owned()
	;

	let new_file = sqlx::query!("
		INSERT INTO vfs_files
			(file_path, file_size, file_type)
		VALUES
			($1, $2, $3)
		RETURNING
			id
		;",
		path, file_data.file.file_size, file_data.file_type.to_string()
	)
		.fetch_one(db_pool)
		.await
		.map_err(VFSError::Sql)?
	;

	match file_data.file_type {
		VFSFileType::Image(ffprobe_output) => {
			// TODO: create image thumbnail 
			let Some(stream) = ffprobe_output.streams.first() else {
				return Err(VFSError::MediaStreamMissing);
			};

			sqlx::query!("
				INSERT INTO image_files
					(
						id,
						width, height,
						codec_name, pix_fmt
					)
				VALUES
					($1, $2, $3, $4, $5)
				;",
				new_file.id,
				stream.width, stream. height,
				stream.codec_name, stream.pix_fmt
			)
				.execute(db_pool)
				.await
				.map_err(VFSError::Sql)?
			;
		},
		VFSFileType::Text => {
				
		},
		VFSFileType::Multimedia(ffprobe_output) => {
			let duration = ffprobe_output.format.duration;
			let bitrate = ffprobe_output.format.bit_rate;
			let video_stream = ffprobe_output.streams.iter().find(|stream| matches!(stream.codec_type, crate::media::util::MediaCodecType::Video));
			let audio_stream = ffprobe_output.streams.iter().find(|stream| matches!(stream.codec_type, crate::media::util::MediaCodecType::Audio));
			if let Some(video) = video_stream {
				let audio_codec = audio_stream.map(|stream| stream.codec_name.clone()).flatten();
				sqlx::query!("
					INSERT INTO video_files
						(
							id,
							duration, width, height,
							r_frame_rate, avg_frame_rate,
							video_codec, audio_codec
						)
					VALUES
						($1, $2, $3, $4, $5, $6, $7, $8)
					;",
					new_file.id,
					duration, video.width, video.height,
					video.r_frame_rate, video.avg_frame_rate,
					video.codec_name, audio_codec
				)
					.execute(db_pool)
					.await
					.map_err(VFSError::Sql)?
				;
			} else if let Some(audio) = audio_stream {
				sqlx::query!("
					INSERT INTO audio_files
						(
							id,
							duration, codec_name, bitrate,
							sample_format, sample_rate, channels
						)
					VALUES
						($1, $2, $3, $4, $5, $6, $7)
					;",
					new_file.id,
					duration, audio.codec_name, bitrate,
					audio.sample_fmt, audio.sample_rate, audio.channels
				)
					.execute(db_pool)
					.await
					.map_err(VFSError::Sql)?
				;
			} else {
				return Err(VFSError::MediaStreamMissing);
			}
		},
			
	}

	Ok(new_file.id)
}

async fn set_vfs_file_to_node(
	db_pool: &Pool<Postgres>,
	file_id: uuid::Uuid,
	node_id: uuid::Uuid,
) -> Result<(), VFSError> {
	sqlx::query!("
		UPDATE vfs_nodes
		SET vfs_file = $2
		WHERE id = $1
		;",
		node_id,
		file_id
	)
		.execute(db_pool)
		.await
		.map_err(VFSError::Sql)
		.map(|_| ())
}

pub async fn traverse_vfs_path(
	db_pool: &Pool<Postgres>,
	vfs_path: PathBuf,
) -> Result<uuid::Uuid, VFSError> {
	let mut current = ensure_vfs_root(db_pool).await?;

	for part in vfs_path.iter().map(OsStr::to_str) {
		let Some(part) = part else {
			println!("invalid osstr encountered in vfs path: '{vfs_path:?}'");
			continue;
		};
		if part.to_ascii_lowercase() == "root" {
			continue;
		}

		current = get_vfs_node(
			db_pool,
			part
				.to_string()
				.replace("%20", " "),
			Some(current)
		).await?;
	}

	Ok(current)
}

pub async fn get_vfs_path_to(
	db_pool: &Pool<Postgres>,
	to: uuid::Uuid,
) -> Result<PathBuf, VFSError> {
	struct CrawlNode {
		id: uuid::Uuid,
		node_name: String,
		parent_id: Option<uuid::Uuid>
	}

	let get_node_raw = async |node_id: uuid::Uuid| {
		sqlx::query_as!(
			CrawlNode,
			"SELECT node_name, id, parent_id
			FROM vfs_nodes
			WHERE id = $1
			;",
			node_id
		)
			.fetch_one(db_pool)
			.await
			.map_err(VFSError::Sql)
	};

	let mut current = get_node_raw(to).await?;

	let mut path = PathBuf::new();

	while let Some(parent) = current.parent_id {
		path = PathBuf::from(current.node_name.clone()).join(path);
		current = get_node_raw(parent).await?;
	}

	Ok(path)
}

/// create the needed vfs path, returning the last node's id
pub async fn ensure_vfs_path(
	db_pool: &Pool<Postgres>,
	vfs_path: PathBuf,
) -> Result<uuid::Uuid, VFSError> {
	// println!("creating vfs path '{vfs_path:?}'");
	let mut current = ensure_vfs_root(db_pool).await?;
	
	for part in vfs_path.iter().map(OsStr::to_str) {
		let Some(part) = part else {
			println!("invalid osstr encountered in vfs path: '{vfs_path:?}'");
			continue;
		};
		if part.to_ascii_lowercase() == "root" {
			continue;
		}

		current = create_vfs_node_internal(
			db_pool,
			VfsNodeCreateArgs {
				name: part.to_string(),
				hide: false
			},
			Some(current)
		).await?;
	}

	// println!("path '{vfs_path:?}' created");
	Ok(current)
}

/// commit file to vfs, returning the vfs file id and the node id
pub async fn commit_file_to_vfs(
	mut data: VfsFileData,
	db_pool: &Pool<Postgres>,
	vfs_target: Option<VfsTarget>,
) -> Result<(uuid::Uuid, uuid::Uuid), VFSError> {
	println!("committing file '{:?}'", data.file.path);

	// the uuid for the file on disk
	let file_uuid = uuid::Uuid::new_v4();
	let hier_path = get_hierarchial_hash_path(file_uuid);
	let abs_path = get_site_folder()
		.join(get_vfs_dir())
		.join(&hier_path)
	;
	// println!("new file path: {path:?}");
	// println!("moving file...");
	// path.add_extension(file.path.extension().unwrap_or(OsStr::new("")));
	let _ = data.move_file(abs_path.with_extension(data.file.path.extension().unwrap_or(OsStr::new(""))))?;

	// println!("file moved to '{:?}'", file.path);
	

	let parent = match vfs_target {
		Some(VfsTarget::Node(node)) => node,
		Some(VfsTarget::Path(vfs_path)) => ensure_vfs_path(db_pool, vfs_path).await?,
		None => ensure_vfs_root(db_pool).await?,
	};

	let file_t = create_vfs_file(db_pool, data.clone());
	let node_t = create_vfs_node_internal(
		db_pool,
		VfsNodeCreateArgs {
			name: data.name.clone(),
			hide: data.hide,
		},
		Some(parent)
	);

	let (file_id, node_id) = try_join!(
		file_t,
		node_t
	)?;

	set_vfs_file_to_node(db_pool, file_id, node_id).await?;
	Ok((file_id, node_id))
}

async fn mark_vfs_node_updated(
	db_pool: &Pool<Postgres>,
	node_id: uuid::Uuid,
) -> Result<(), VFSError> {
	sqlx::query!("
		UPDATE vfs_nodes
		SET updated_at = now()
		WHERE id = $1
		;",
		node_id
	)
		.execute(db_pool)
		.await
		.map_err(VFSError::Sql)
		.map(|_| ())
}

pub async fn move_vfs_file(
	db_pool: &Pool<Postgres>,
	node_id: uuid::Uuid,
	target: VfsTarget,
) -> Result<(), VFSError> {
	let parent_id = match target {
		VfsTarget::Node(parent_id) => parent_id,
		VfsTarget::Path(vfs_path) => {
			ensure_vfs_path(db_pool, vfs_path).await?
		},
	};

	sqlx::query!("
		UPDATE vfs_nodes
		SET parent_id = $1
		WHERE id = $2
		;",
		parent_id,
		node_id
	)
		.execute(db_pool)
		.await
		.map_err(VFSError::Sql)?
	;

	update_vfs_closures(db_pool, node_id).await?;
	mark_vfs_node_updated(db_pool, node_id).await
}

pub async fn init_vfs(
	db_pool: &Pool<Postgres>,
) -> Result<(), VFSError> {
	println!("initializing vfs...");
	let _ = ensure_vfs_root(db_pool).await?;
	// ensure_vfs_path(db_pool, "a/b/c/d".into()).await?;
	Ok(())
}