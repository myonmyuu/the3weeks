#[cfg(feature = "server")]
pub mod auth;
pub mod api;

pub mod prelude {
	#[cfg(feature = "server")]
	pub use super::auth::*;
	pub use crate::util::*;
}