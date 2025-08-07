#![feature(bool_to_result)]
#![feature(path_add_extension)]

pub mod prelude {
	pub use leptos::prelude::*;
	#[cfg(feature = "server")]
	pub use server::*;

	#[cfg(feature = "server")]
	pub mod server {
		pub use crate::app::state::prelude::*;
	}
}

pub mod util;
pub mod app;
pub mod user;
pub mod macros;
pub mod ws;
pub mod vfs;
pub mod media;
pub mod downloader;