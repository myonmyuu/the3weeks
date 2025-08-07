pub mod api;
#[cfg(feature = "server")]
pub mod util;

pub mod shared {
	#[derive(Debug)]
	pub enum DownloaderError {
		YtdlInitError,
		YtdlNotSingle,
		NoTempFile,
		#[cfg(feature = "server")]
		Ytdl(youtube_dl::Error),
		Io(std::io::Error),
		Media(crate::media::shared::MediaError)
	}
}

#[allow(unused)]
pub mod prelude {
	pub use super::api::*;
	pub use super::shared::*;
	#[cfg(feature = "server")]
	pub use super::util::*;
}