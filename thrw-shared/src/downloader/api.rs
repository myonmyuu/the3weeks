use super::prelude::*;
use crate::prelude::*;
use crate::user::prelude::*;
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

	let file_data = crate::vfs::util::VfsFileData {
		name: ytdl_res.output.title.unwrap_or("UNKNOWN TITLE??".to_string()),
		file: ytdl_res.file.clone(),
		file_type: crate::vfs::util::VFSFileType::Multimedia(
			crate::media::util::get_media_file_metadata(ytdl_res.file.path).await
				.map_err(make_server_err)?
		)
	};

	crate::vfs::util::commit_file_to_vfs(
		file_data.clone(),
		&state.db_pool,
		vfs_target
	)
		.await
		.map_err(make_server_err)
}