pub mod prelude {
	pub use leptos::prelude::*;
	pub use leptos::reactive::spawn_local;
	pub use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
	pub use leptos_router::{
		components::{Outlet, ParentRoute, Route, Router, Routes, A, ProtectedRoute, ProtectedParentRoute}, MatchNestedRoutes, path, StaticSegment
	};
	pub use leptos_router::params::Params;
	pub use crate::routes::helpers::*;
	pub use crate::util::*;
	pub use crate::util::consts::*;
}

mod components;
pub(crate) mod routes;
pub(crate) mod storage;
mod api;
mod util;

pub use routes::App;
pub use routes::shell;