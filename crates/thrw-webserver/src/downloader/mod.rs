use std::{ffi::{OsStr, OsString}, future::{join, Future}};

use sqlx::{Pool, Postgres};
use thrw_shared::{app::media_request::{DownloaderContext, MediaRequest, MediaRequestError, YtdlRequest, YtdlResult}, downloader::shared::DownloaderError, make_error_type, media::util::get_media_file_metadata, vfs::util::{commit_file_to_vfs, FileRef, VFSFileType}};
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

async fn handle_yt_dl(
	req: YtdlRequest,
	db_pool: Pool<Postgres>,
) -> Result<YtdlResult, DownloaderError> {
	if let Err(err) = dl_ytdl_if_needed().await {
		println!("error downloading ytdl: {err:?}");
		return Err(DownloaderError::YtdlInitError.into());
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
		.extra_arg("--write-thumbnail")
		.output_template(format!("{}.%(ext)s", consts::YT_DL_TEMP_FILENAME))
	;

	let (output, _) = try_join!(
		output_b.run_async(),
		dl_b.download_to_async(path)
	)
		.map_err(DownloaderError::Ytdl)?
	;
	let output = output.into_single_video()
		.ok_or(DownloaderError::YtdlNotSingle)?
	;

	let files: Result<Vec<_>, std::io::Error> = std::fs::read_dir(path)
		.map_err(DownloaderError::Io)?
		.collect()
	;
	let files = files.map_err(DownloaderError::Io)?;
	let file_refs: Vec<FileRef> = files
		.iter()
		.filter(|en| en.path().with_extension("").file_name().map(OsStr::to_str).unwrap_or(Some("")).unwrap_or("") == consts::YT_DL_TEMP_FILENAME)
		.map(|entry| entry.into())
		.collect()
	;

	let mut media = None;
	let mut thumbnail = None;

	for file in file_refs {
		let data = std::fs::read(&file.path)
			.map_err(DownloaderError::Io)?
		;
		let infer_data = infer::get(&data);
		match infer_data {
			Some(itype) => match itype.matcher_type() {
				  infer::MatcherType::Video
				| infer::MatcherType::Audio => media = Some(file),
				infer::MatcherType::Image => thumbnail = Some(file),
				_ => {
					println!("invalid file type received from ytdl, removing");
					let _ = file.delete_file();
					continue;
				},
			},
			None => {
				println!("unable to determine ytdl file type, removing");
				let _ = file.delete_file();
				continue;
			},
		};
	}
	
	let result = YtdlResult {
		media: media.ok_or(DownloaderError::NoTempFile)?,
		thumbnail,
		output: output,
	};
	Ok(result)
}

async fn dl_ytdl_if_needed() -> Result<(), DownloaderError> {
	let path = std::path::Path::new(consts::YT_DL_PATH);
	if path.exists() {
		return Ok(());
	}

	println!("downloading ytdl...");
	youtube_dl::download_yt_dlp(consts::YT_DL_DIR)
		.await
		.map(|_| ())
		.map_err(DownloaderError::Ytdl)
}

fn make_handler(
	db_pool: Pool<Postgres>,
	mut message_recv: UnboundedReceiver<MediaRequest>
) -> impl Future<Output = ()> {
	async move {
		while let Some(message) = message_recv.recv().await {
			match message {
				MediaRequest::Ytdl(req, ret) => {
					let _ = ret.send(
						handle_yt_dl(req, db_pool.clone())
							.await
							.map_err(MediaRequestError::Dl)
					);
				},
			}
		}

		println!("media downloader thread closing");
	}
}

pub fn init_downloader(
	db_pool: &Pool<Postgres>,
) -> DownloaderContext {
	let (send, receive) = unbounded_channel();

	tokio::spawn(make_handler(db_pool.clone(),receive));

	return DownloaderContext { request_channel: send }
}