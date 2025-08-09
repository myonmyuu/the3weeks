use serde::{Deserialize, Serialize};

use super::prelude::*;

use crate::prelude::*;
use crate::user::prelude::*;
#[cfg(feature = "server")]
use super::util::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfsGetNodeArgs {
	pub show_hidden: bool,
}

#[server]
pub async fn get_vfs_nodes(
	at: VfsTarget,
	args: Option<VfsGetNodeArgs>,
) -> Result<Vec<PubVfsNode>, ServerFnError> {
	let (id, _) = require_auth().await?;
	let db = extract_db()?;
	
	// println!("getting nodes at {at:?}");

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

	let show_hidden = args.map(|args| args.show_hidden).unwrap_or(false);

	let vals = sqlx::query!("
		SELECT * FROM vfs_nodes
		WHERE parent_id = $1
		  AND (hide = false OR hide = $2)
		;",
		id,
		show_hidden
	)
		.fetch_all(&db)
		.await?
	;
	futures::future::join_all(vals
		.iter()
		.map(async |rec| get_pub_vfs_node(&db, rec.id)
			.await
			.map_err(make_server_err)
		))
		.await
		.into_iter()
		.collect()
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
		VfsNodeCreateArgs {
			name: name,
			hide: false
		},
		Some(parent)
	)
		.await
		.map_err(make_server_err)?
	;

	get_pub_vfs_node(&db, id)
		.await
		.map_err(make_server_err)
}