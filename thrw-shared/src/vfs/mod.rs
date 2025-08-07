pub mod api;

#[cfg(feature = "server")]
pub mod util;

pub mod shared {
    use std::path::PathBuf;

    use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize)]
	pub enum VfsTarget {
		Node(uuid::Uuid),
		Path(std::path::PathBuf),
	}

	#[derive(Debug)]
	pub enum VFSError {
		NotFound,
		InvalidPath,
		Io(std::io::Error),
		#[cfg(feature = "server")]
		Sql(sqlx::Error),
		MediaMissingMetadata(String),
		MediaStreamMissing,
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	pub enum PubVfsNodeType {
		Folder,
		Video,
		Audio,
		Image,
		Text,
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	pub struct PubVfsNode {
		pub id: uuid::Uuid,
		pub name: String,
		pub path: PathBuf,
		pub node_type: PubVfsNodeType,
	}
}

#[allow(unused)]
pub mod prelude {
	pub use super::api::*;
	pub use super::shared::*;
	#[cfg(feature = "server")]
	pub use super::util::*;
}