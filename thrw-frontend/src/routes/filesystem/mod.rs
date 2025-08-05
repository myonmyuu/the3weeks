use crate::prelude::*;

#[component]
pub fn vfs_path_handler() -> impl IntoView {
	let location = leptos_router::hooks::use_location();
    let full_path = location.pathname.get_untracked(); // e.g., "/docs/file.md"

    let path_segments: Vec<&str> = full_path
        .trim_start_matches('/')
        .split('/')
        .collect();

	view! {
		<p>{path_segments.join(", ")}</p>
	}
}

#[component(transparent)]
pub fn FilesystemRoutes() -> impl MatchNestedRoutes + Clone {
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