use std::future::{join, Future};

use thrw_shared::{app::media_request::{DownloaderContext, MediaRequest, YoutubeRequest, YoutubeResult}, make_error_type};
use tokio::{sync::{mpsc::{unbounded_channel, UnboundedReceiver}, oneshot}, try_join};

mod consts {
	pub const YT_DL_DIR: &str = "./ytdl";
	#[cfg(target_family = "windows")]
	pub const YT_DL_BIN: &str = "yt-dlp.exe";
	#[cfg(target_family = "unix")]
	pub const YT_DL_BIN: &str = "yt-dlp_linux";
	pub const YT_DL_PATH: &str = const_format::concatcp!(YT_DL_DIR, "/", YT_DL_BIN);
}

#[derive(Debug, Clone)]
enum LocalDowloadError {
	YtdlInitError,
}

make_error_type!{
	DownloaderError {
		Local(LocalDowloadError),
		Ytdl(youtube_dl::Error),
	}
}

async fn handle_yt_dl(req: YoutubeRequest, ret: Option<oneshot::Sender<YoutubeResult>>) -> Result<(), DownloaderError> {
	println!("handling youtube download...");
	let path = std::path::Path::new("ytdl");
	let mut output_b = youtube_dl::YoutubeDl::new(req.url.clone());
	let output_b = output_b
		.socket_timeout("15")
		.youtube_dl_path(consts::YT_DL_PATH)
	;
	let output = output_b.run_async();
	let mut dl_b = youtube_dl::YoutubeDl::new(req.url);
	let dl_b = dl_b
		.socket_timeout("15")
		.youtube_dl_path(consts::YT_DL_PATH)
		.extract_audio(req.audio_only)
		.extra_arg("-o \"TEMP.mp3\"")
	;
	let dl = dl_b.download_to_async(path);

	let (output, _) = try_join!(output, dl)?;

	if ret.map_or_default(|a| a.send(YoutubeResult {  }).is_err()) {
		// TODO: log ? 
	}

	println!("youtube dl okay");
	Ok(())
}

async fn dl_ytdl_if_needed() -> Result<(), DownloaderError> {
	let path = std::path::Path::new(consts::YT_DL_PATH);
	if path.exists() {
		return Ok(());
	}

	println!("downloading ytdl...");
	youtube_dl::download_yt_dlp(path)
		.await
		.map(|_| ())
		.map_err(Into::into)
}

fn make_handler(mut message_recv: UnboundedReceiver<MediaRequest>) -> impl Future<Output = ()> {
	async move {
		if let Err(err) = dl_ytdl_if_needed().await {
			println!("error downloading ytdl: {err:?}");
		}

		while let Some(message) = message_recv.recv().await {
			match message {
				MediaRequest::Youtube(req, ret) => {
					if let Err(err) = handle_yt_dl(req, ret).await {
						println!("error with youtube request: {err:?}");
					}
				},
			}
		}
	}
}

pub fn init_downloader() -> DownloaderContext {
	let (send, receive) = unbounded_channel();

	tokio::spawn(make_handler(receive));

	return DownloaderContext { request_channel: send }
}