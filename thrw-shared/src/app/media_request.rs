use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::{UnboundedReceiver, UnboundedSender}, oneshot};
use youtube_dl::{SingleVideo, YoutubeDlOutput};

use crate::{downloader::shared::DownloaderError, vfs::util::FileRef};

#[derive(Debug)]
pub enum MediaRequestError {
	Dl(DownloaderError)
}

#[derive(Debug, Clone)]
pub struct DownloaderContext {
	pub request_channel: UnboundedSender<MediaRequest>,
}

#[derive(Debug, Clone)]
pub struct YtdlRequest {
	pub url: String,
	pub audio_only: bool,
}

#[derive(Debug)]
pub enum MediaRequest {
	Ytdl(YtdlRequest, oneshot::Sender<Result<YtdlResult, MediaRequestError>>),
}

#[derive(Debug, Clone)]
pub struct YtdlResult {
	pub output: SingleVideo,
	pub file: FileRef,
}