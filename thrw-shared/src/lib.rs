#![feature(bool_to_result)]
#![feature(path_add_extension)]

pub mod prelude {
	pub mod client {
		pub use leptos::prelude::*;
	}
}

pub mod util;
pub mod app;
pub mod user;
pub mod macros;
pub mod ws;
pub mod vfs;