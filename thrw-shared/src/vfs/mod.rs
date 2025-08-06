pub mod api;

#[cfg(feature = "server")]
pub mod util;

pub mod shared {
    use std::path::PathBuf;

	#[derive(Debug)]
	pub enum VFSError {
		Io(std::io::Error),
		#[cfg(feature = "server")]
		Sql(sqlx::Error),
	}

	#[derive(Debug, Clone)]
	pub enum VFSFileType {
		Audio,
		Video,
		Image,
		Text,
	}
	impl ToString for VFSFileType {
		fn to_string(&self) -> String {
			match self {
				VFSFileType::Audio => "audio",
				VFSFileType::Video => "video",
				VFSFileType::Image => "image",
				VFSFileType::Text => "text",
			}.to_string()
		}
	}
}

#[allow(unused)]
pub mod prelude {
	pub use super::api::*;
	pub use super::shared::*;
	#[cfg(feature = "server")]
	pub use super::util::*;
}