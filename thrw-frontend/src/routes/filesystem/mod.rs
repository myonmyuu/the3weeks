use std::{ffi::OsStr, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};
use thrw_shared::vfs::{api::{create_vfs_node, get_vfs_nodes}, shared::VfsTarget};

use crate::prelude::*;

pub mod consts {
	pub const NODE_LIST_ID: i32 = crate::prelude::VFS_IDS + 1;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VfsRoute {
	Invalid,
	Root,
	Route(PathBuf)
}
impl From<VfsRoute> for PathBuf {
	fn from(value: VfsRoute) -> Self {
		match value {
			VfsRoute::Root | VfsRoute::Invalid => PathBuf::from("root"),
			VfsRoute::Route(path) => path,
		}
	}
}

#[component]
pub fn vfs_path_handler() -> impl IntoView {
	let location = leptos_router::hooks::use_location();
	let vfs_node_review = ReviewEvent::<{VFS_IDS}>::use_provided();
	let path_signal = RwSignal::new(VfsRoute::Invalid);
	let node_text = RwSignal::new("".to_string());
    
	Effect::new(move |_| {
		// log::debug!("path refreshing...");
		let path = location.pathname.get();
		let path = PathBuf::from(path);
		let Ok(path) = path.strip_prefix("/vfs/").map(Path::to_path_buf) else {
			// log::debug!("path is invalid");
			path_signal(VfsRoute::Invalid);
			return;
		};

		// log::debug!("stripped path: {path:?}");
		if path == PathBuf::from("root") {
			// log::debug!("path is root");
			path_signal(VfsRoute::Root);
			return;
		}

		// log::debug!("new path: {path:?}");
		path_signal(VfsRoute::Route(path));
		vfs_node_review.invalidate();
	});

	let node_res = Resource::new(path_signal, async |path| {
		// log::debug!("nodes refreshing...");
		let path = match path {
			VfsRoute::Invalid => return vec![],
			VfsRoute::Root => PathBuf::from("root"),
			VfsRoute::Route(path) => path,
		};

		let Ok(nodes) = get_vfs_nodes(VfsTarget::Path(path)).await else {
			return vec![]
		};

		nodes
	});

	let get_path_string = Memo::new(move |_prev| {
		match path_signal() {
			VfsRoute::Invalid => "path invalid!".to_string(),
			VfsRoute::Root => "root".to_string(),
			VfsRoute::Route(path) => path.to_str().unwrap().to_string(),
		}
	});
    
	view! {
		<p>{move||get_path_string()}</p>
		<Show when=move|| { !matches!(path_signal(), VfsRoute::Root)}>
			<A
				href=move|| {
					let mut path = PathBuf::from("/vfs")
						.join(Into::<PathBuf>::into(path_signal()))
					;
					path.pop();
					path
						.to_str()
						.unwrap()
						.to_string()
				}
			>
			return (..)
			</A>
		</Show>
		<Transition fallback=move||view! { <p>Loading...</p>} >
		{move|| {
			node_res.get().map(|nodes| {
				view! {
					{nodes.iter().map(|node| {
						let node = node.clone();
						let path_node = node.clone();
						view! {
							<A
								href=move|| {
									PathBuf::from("/vfs")
										.join(Into::<PathBuf>::into(path_signal.get_untracked()))
										.join(path_node.name.clone())
										.to_str()
										.unwrap()
										.to_string()
								}
							>
								<p>{node.name.clone()}</p>
							</A>
						}
					}).collect_view()}
				}
			})
		}}
		</Transition>
		<input bind:value=node_text />
		<button
			on:click=move|_|{
				spawn_local(async move {
					if let Ok(node) = create_vfs_node(VfsTarget::Path(path_signal.get_untracked().into()), node_text.get_untracked()).await {
						log::debug!("node created, refreshing...");
						vfs_node_review.invalidate();
						node_res.refetch();
					} else {
						log::debug!("node creation failed");
					}
				});
			}
		>
			add new
		</button>
	}
}

#[component(transparent)]
pub fn FilesystemRoutes() -> impl MatchNestedRoutes + Clone {
	ReviewEvent::<{VFS_IDS}>::provide_new();
	
	view! {
		<ProtectedParentRoute
			path=path!("/vfs")
			view=EmptyParent
			condition=check_login_raw
			redirect_path=||"/"
		>
			<Route path=path!("/*path") view=VfsPathHandler />
		</ProtectedParentRoute>
	}
	.into_inner()
}