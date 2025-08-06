use std::{ffi::{OsStr, OsString}, future::{join, Future}};

use sqlx::{Pool, Postgres};
use thrw_shared::{app::media_request::{DownloaderContext, MediaRequest, YoutubeRequest, YtdlResult}, make_error_type, vfs::{shared::{VFSFileType}, util::commit_file_to_vfs}};
use tokio::{sync::{mpsc::{unbounded_channel, UnboundedReceiver}, oneshot}, try_join};

mod consts {
	pub const YT_DL_DIR: &str = "./ytdl";
	#[cfg(target_family = "windows")]
	pub const YT_DL_BIN: &str = "yt-dlp.exe";
	#[cfg(target_family = "unix")]
	pub const YT_DL_BIN: &str = "yt-dlp_linux";
	pub const YT_DL_PATH: &str = const_format::concatcp!(YT_DL_DIR, "/", YT_DL_BIN);

	pub const YT_DL_TEMP_FILENAME: &str = "TEMP";
}

#[derive(Debug, Clone)]
enum LocalDowloadError {
	YtdlInitError,
	YtdlNotSingle,
	NoTempFile,
}

make_error_type!{
	DownloaderError {
		Local(LocalDowloadError),
		Ytdl(youtube_dl::Error),
		Io(std::io::Error),
	}
}

async fn handle_yt_dl(
	req: YoutubeRequest,
	ret: Option<oneshot::Sender<YtdlResult>>,
	db_pool: Pool<Postgres>,
) -> Result<(), DownloaderError> {
	if let Err(err) = dl_ytdl_if_needed().await {
		println!("error downloading ytdl: {err:?}");
		return Err(LocalDowloadError::YtdlInitError.into());
	}

	println!("downloading media at '{}'", req.url);
	let path = std::path::Path::new(consts::YT_DL_DIR);
	let mut output_b = youtube_dl::YoutubeDl::new(req.url.clone());
	let output_b = output_b
		.socket_timeout("15")
		.youtube_dl_path(consts::YT_DL_PATH)
	;

	let mut dl_b = youtube_dl::YoutubeDl::new(req.url);
	let dl_b = dl_b
		.socket_timeout("15")
		.youtube_dl_path(consts::YT_DL_PATH)
		.extract_audio(req.audio_only)
		.output_template(format!("{}.%(ext)s", consts::YT_DL_TEMP_FILENAME))
	;

	let (output, _) = try_join!(
		output_b.run_async(),
		dl_b.download_to_async(path)
	)?;
	let output = output.into_single_video()
		.ok_or(LocalDowloadError::YtdlNotSingle)?
	;

	let files: Result<Vec<_>, std::io::Error> = std::fs::read_dir(path)?
		.collect()
	;
	let files = files?;
	let entry = files
		.iter()
		.find(|en| en.path().with_extension("").file_name().map(OsStr::to_str).unwrap_or(Some("")).unwrap_or("") == consts::YT_DL_TEMP_FILENAME)
		.ok_or(LocalDowloadError::NoTempFile)?
	;

	let file_data = thrw_shared::vfs::util::VfsFileData {
		name: output.title.unwrap_or("UNKNOWN TITLE??".to_string()),
		file: entry.into(),
		file_type: if req.audio_only { VFSFileType::Audio } else { VFSFileType::Video }
	};

	let _ = commit_file_to_vfs(file_data.clone(), &db_pool, None).await;

	if ret.map_or_default(|a| a.send(YtdlResult {  }).is_err()) {
		// TODO: log ? 
	}

	println!("download of '{file_data:?}' completed");
	// println!("file format: {:?}", output.);
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

fn make_handler(
	db_pool: Pool<Postgres>,
	mut message_recv: UnboundedReceiver<MediaRequest>
) -> impl Future<Output = ()> {
	async move {
		while let Some(message) = message_recv.recv().await {
			match message {
				MediaRequest::Youtube(req, ret) => {
					if let Err(err) = handle_yt_dl(req, ret, db_pool.clone()).await {
						println!("error with youtube request: {err:?}");
					}
				},
			}
		}
	}
}

pub fn init_downloader(
	db_pool: &Pool<Postgres>,
) -> DownloaderContext {
	let (send, receive) = unbounded_channel();

	tokio::spawn(make_handler(db_pool.clone(),receive));

	return DownloaderContext { request_channel: send }
}