use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::{UnboundedReceiver, UnboundedSender}, oneshot};

#[derive(Debug, Clone)]
pub struct DownloaderContext {
	pub request_channel: UnboundedSender<MediaRequest>,
}

#[derive(Debug, Clone)]
pub struct YoutubeRequest {
	pub url: String,
	pub audio_only: bool,
}

#[derive(Debug)]
pub enum MediaRequest {
	Youtube(YoutubeRequest, Option<oneshot::Sender<YtdlResult>>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtdlResult {

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaResult {

}