use std::{ffi::OsStr, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};
use thrw_shared::{downloader::api::download_media, vfs::{api::{create_vfs_node, get_vfs_nodes}, shared::{PubVfsNode, VfsTarget}}};

use crate::prelude::*;

pub mod consts {
	pub const NODE_LIST_ID: i32 = crate::prelude::VFS_IDS + 1;

	pub const VFS_URL: &str = "vfs";
	pub const VFS_ROOT: &str = "root";
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VfsRoute {
	Invalid,
	Root,
	Route(PathBuf)
}
impl From<VfsRoute> for VfsTarget {
	fn from(value: VfsRoute) -> Self {
		Self::Path(value.into())
	}
}
impl From<VfsRoute> for PathBuf {
	fn from(value: VfsRoute) -> Self {
		match value {
			VfsRoute::Root | VfsRoute::Invalid => PathBuf::from(consts::VFS_ROOT),
			VfsRoute::Route(path) => path,
		}
	}
}

#[component]
pub fn vfs_entry(
	node: PubVfsNode,
) -> impl IntoView {
	let path_href = PathBuf::from("/")
		.join(consts::VFS_URL)
		.join(consts::VFS_ROOT)
		.join(node.path.clone())
		.to_string_lossy()
		.into_owned()
	;
	let node_for_when = node.clone();
	let name = node.name.clone();
	let name_link = name.clone();
	let href = move||path_href.clone();
	let when = move||matches!(node_for_when.node_type, thrw_shared::vfs::shared::PubVfsNodeType::Folder);
	let thumb_src = move||{
		let node = node.clone();
		node.thumbnail.unwrap_or(match node.node_type {
			thrw_shared::vfs::shared::PubVfsNodeType::Folder => "/icons/folder.png",
			thrw_shared::vfs::shared::PubVfsNodeType::Video => "/icons/video-file.png",
			thrw_shared::vfs::shared::PubVfsNodeType::Audio => "/icons/music-file.png",
			thrw_shared::vfs::shared::PubVfsNodeType::Image => "/icons/image.png",
			thrw_shared::vfs::shared::PubVfsNodeType::Text => "/icons/document.png",
		}.to_string())
	};

	let fallback = move|| view! {
		<p
			class="vfs_link"
		>
			{name.clone()}
		</p>
	}; 
	let main_view = move || view! {
		<A
			attr:class="vfs_link"
			href=href
		>
			<p>{name_link.clone()}</p>
		</A>
	};

	view! {
		<div
			class="vfs_node"
		>
			<img src=thumb_src />
			<Show
				when=when
				fallback=fallback
			>
			{main_view.clone()()}
			</Show>
		</div>
	}
}

#[component]
pub fn vfs_path_handler() -> impl IntoView {
	let location = leptos_router::hooks::use_location();
	let vfs_node_review = ReviewEvent::<{VFS_IDS}>::use_provided();
	let path_signal = RwSignal::new(VfsRoute::Invalid);
	let node_text = RwSignal::new("".to_string());
	let vid_url = RwSignal::new("".to_string());
	let video_check = RwSignal::new(false);

	let node_res = Resource::new(path_signal, async |path| {
		// log::debug!("nodes refreshing...");
		let path = match path {
			VfsRoute::Invalid => return vec![],
			VfsRoute::Root => PathBuf::from(consts::VFS_ROOT),
			VfsRoute::Route(path) => path,
		};

		let args = thrw_shared::vfs::api::VfsGetNodeArgs {
			show_hidden: false,
		};

		let Ok(nodes) = get_vfs_nodes(VfsTarget::Path(path), Some(args)).await else {
			return vec![]
		};

		nodes
	});

	let url_segment_res = Resource::new(path_signal, async |path| {
		match path {
			VfsRoute::Root | VfsRoute::Invalid => vec![(consts::VFS_ROOT.to_string(), PathBuf::from(consts::VFS_ROOT))],
			VfsRoute::Route(path) => {
				let mut cur_path = PathBuf::from("/").join(consts::VFS_URL);
				path.iter().map(|seg| {
					let owned = seg.to_str()
						.unwrap_or("PATH ERROR")
						.to_string()
						.replace("%20", " ")
					;
					cur_path.push(seg);
					(owned, cur_path.clone())
				}).collect()
			},
		}
	});

	let refresh_path_parts = move || {
		node_res.refetch();
		url_segment_res.refetch();
		vfs_node_review.invalidate();
	};
	    
	Effect::new(move |_| {
		// log::debug!("path refreshing...");
		let path = location.pathname.get();
		// log::debug!("path: {path}");
		let path = PathBuf::from(path);
		let Ok(path) = path.strip_prefix(format!("/{}/", consts::VFS_URL)).map(Path::to_path_buf) else {
			// log::debug!("path is invalid");
			path_signal(VfsRoute::Invalid);
			return;
		};

		// log::debug!("stripped path: {path:?}");
		if path == PathBuf::from(consts::VFS_ROOT) {
			// log::debug!("path is root");
			path_signal(VfsRoute::Root);
			return;
		}

		// log::debug!("new path: {path:?}");
		path_signal(VfsRoute::Route(path));
		refresh_path_parts();
	});

	let get_path_string = Memo::new(move |_prev| {
		match path_signal() {
			VfsRoute::Invalid => "path invalid!".to_string(),
			VfsRoute::Root => consts::VFS_ROOT.to_string(),
			VfsRoute::Route(path) => path.to_string_lossy().into_owned(),
		}
	});
    
	view! {
		// path visibliity
		<div>
			<Transition fallback=move || view! {  }>
			{move || url_segment_res.get().map(|path| {
				let path = path.clone();
				view! {
					<p>
					{path.iter().map(|seg| {
						let seg = seg.clone();
						view! {
							<span>
								<A
									href=move|| {
										(
											PathBuf::from("/")
												.join(consts::VFS_URL)
												.join(seg.1.clone())
										)
											.to_string_lossy()
											.into_owned()
									}
								>
									{seg.0.clone()}
								</A>
							</span>
							<span>/</span>
						}
					}).collect_view()}
					</p>
				}
			})}
			</Transition>
		</div>

		// nodes
		<div
		>
			// return
			<Show
				when=move|| { !matches!(path_signal(), VfsRoute::Root)}
				fallback=move|| view! {
					<div
						class="vfs_node"
					>
					</div>
				}
			>
				<div
					class="vfs_node"
				>
					<A
						attr:class="vfs_link"
						href=move|| {
							let mut path = PathBuf::from("/")
								.join(consts::VFS_URL)
								.join(Into::<PathBuf>::into(path_signal()))
							;
							path.pop();
							path
								.to_string_lossy()
								.into_owned()
						}
					>
						<p>./..</p>
					</A>
				</div>
			</Show>
			// nodes in path
			<Transition fallback=move||view! { <p>Loading...</p>} >
			{move|| {
				node_res.get().map(|nodes| {
					view! {
						{nodes.iter().map(|node| {
							let node = node.clone();
							view! { <VfsEntry node /> }
						}).collect_view()}
					}
				})
			}}
			</Transition>
		</div>
		// add node
		<div>
			<input bind:value=node_text />
			<button
				on:click=move|_|{
					spawn_local(async move {
						if let Ok(_node) = create_vfs_node(VfsTarget::Path(path_signal.get_untracked().into()), node_text.get_untracked()).await {
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
		</div>
		// download
		<div>
			<input bind:value=vid_url />
			<input bind:value=video_check type="checkbox" />
			<button
				on:click=move|_|{
					spawn_local(async move {
						let url = vid_url.get_untracked();
						log::debug!("downloading media at {}...", url);
						let dl_res = download_media(
							url,
							!video_check.get_untracked(),
							Some(path_signal.get_untracked().into())
						)
							.await
						;
						
						match dl_res {
							Ok(_id) => {
								log::debug!("media downloaded, refreshing...");
								vfs_node_review.invalidate();
								node_res.refetch();
							},
							Err(err) => log::debug!("media download failed: {err:?}"),
						};
					});
				}
			>
				download
			</button>
		</div>
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