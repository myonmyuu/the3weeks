#[cfg(feature = "server")]
pub mod util;

pub mod prelude {
	pub use super::shared::*;
}

pub mod shared {
	#[derive(Debug)]
	pub enum MediaError {
		Io(std::io::Error),
		InvalidPath,
		Json(serde_json::Error),
		#[cfg(feature = "server")]
		Ffmpeg(anyhow::Error),
	}
}
