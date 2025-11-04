use super::prelude::*;
use crate::prelude::*;
use crate::user::prelude::*;
use crate::vfs::prelude::*;
use crate::media::prelude::*;
use crate::vfs::shared::VfsTarget;


#[server]
pub async fn download_media(
	url: String,
	audio_only: bool,
	vfs_target: Option<VfsTarget>,
) -> Result<uuid::Uuid, ServerFnError> {
	use crate::app::state::server::extract_state;
	use tokio::sync::oneshot;
	let _ = require_auth().await?;
	let state = extract_state()?;

	let (res_send, res_recv) = oneshot::channel();

	// TODO: check if url has been downloaded prior 

	state.dl_context.request_channel.send(crate::app::media_request::MediaRequest::Ytdl(
		crate::app::media_request::YtdlRequest{
			url,
			audio_only
		},
		res_send
	))?;

	let ytdl_res = match res_recv.await {
		Ok(Ok(res)) => res,
		Ok(Err(dl_err)) => return Err(make_server_err(dl_err)),
		Err(send_err) => return Err(make_server_err(send_err)),
	};
	// println!("dl okay: {ytdl_res:?}");

	let name = ytdl_res.output.title.unwrap_or("UNKNOWN TITLE??".to_string());
	
	let get_ftype = async |(file, infer_type): (FileRef, infer::MatcherType)| {
		match infer_type {
			  infer::MatcherType::Video
			| infer::MatcherType::Audio => Ok(VFSFileType::Multimedia(
				get_media_file_metadata(file.path.clone()).await?
			)),
			infer::MatcherType::Image => Ok(VFSFileType::Image(
				get_media_file_metadata(file.path.clone()).await?
			)),
			_ => {
				println!("invalid file type received from ytdl, removing");
				let _ = file.delete_file();
				return Err(MediaError::InvalidType);
			},
		}
	};

	let media_file_id = {
		let file_data = VfsFileData {
			name: name.clone(),
			file: ytdl_res.media.clone(),
			file_type: get_ftype((ytdl_res.media.clone(), infer::MatcherType::Audio))
				.await
				.map_err(make_server_err)?,
			hide: false,
		};

		let (file_id, _) = commit_file_to_vfs(
			file_data,
			&state.db_pool,
			vfs_target.clone()
		)
			.await
			.map_err(make_server_err)?
		;

		file_id
	};

	if let Some(thumb_file) = ytdl_res.thumbnail.clone() {
		let file_data = VfsFileData {
			name: name.clone() + " Thumbnail",
			file: thumb_file.clone(),
			file_type: get_ftype((thumb_file, infer::MatcherType::Image))
				.await
				.map_err(make_server_err)?,
			hide: true,
		};

		// println!("adding thumbnail; {file_data:?}");

		let (file_id, _) = commit_file_to_vfs(
			file_data,
			&state.db_pool,
			vfs_target.clone()
		)
			.await
			.map_err(make_server_err)?
		;

		set_thumbnail(&state.db_pool, media_file_id, file_id)
			.await
			.map_err(make_server_err)?
		;
	};

	#[cfg(debug_assertions)]
	{
		let _ = crate::util::copy_dir_all(
			std::path::PathBuf::from("./site"),
			std::path::PathBuf::from("./target/site")
		);
	}

	Ok(media_file_id)
}