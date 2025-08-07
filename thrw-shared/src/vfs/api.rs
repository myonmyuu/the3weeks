use super::prelude::*;

use crate::prelude::*;
use crate::user::prelude::*;
#[cfg(feature = "server")]
use super::util::*;

#[server]
pub async fn get_vfs_nodes(
	at: VfsTarget,
) -> Result<Vec<PubVfsNode>, ServerFnError> {
	let (id, _) = require_auth().await?;
	let db = extract_db()?;
	
	let (id, path) = match at {
		VfsTarget::Node(id) => (
			id,
			get_vfs_path_to(&db, id)
				.await
				.map_err(make_server_err)?
		),
		VfsTarget::Path(path) => {
			(
				traverse_vfs_path(&db, path.clone())
					.await
					.map_err(make_server_err)?,
				path
			)
		},
	};

	let vals = sqlx::query!("
		SELECT * FROM vfs_nodes
		WHERE parent_id = $1
		;",
		id
	)
		.fetch_all(&db)
		.await?
		.iter()
		.map(|rec| PubVfsNode {
			id: rec.id,
			name: rec.node_name.clone(),
			path: path.clone().join(rec.node_name.clone()),
			node_type: rec.vfs_file
				.map(|file_id| PubVfsNodeType::Audio)
				.unwrap_or(PubVfsNodeType::Folder)
		})
		.collect()
	;
	Ok(vals)
}

#[server]
pub async fn create_vfs_node(
	at: VfsTarget,
	name: String,
) -> Result<PubVfsNode, ServerFnError> {
	let (id, _) = require_auth().await?;
	let db = extract_db()?;

	let parent = match at {
		VfsTarget::Node(id) => id,
		VfsTarget::Path(path) => traverse_vfs_path(&db, path)
			.await
			.map_err(make_server_err)?,
	};

	let id = create_vfs_node_internal(
		&db,
		name,
		Some(parent)
	)
		.await
		.map_err(make_server_err)?
	;

	get_pub_vfs_node(&db, id)
		.await
		.map_err(make_server_err)
}