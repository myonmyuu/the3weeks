pub mod api;

#[cfg(feature = "server")]
pub mod util;

pub mod shared {
	#[derive(Debug)]
	pub enum VFSError {
		Io(std::io::Error),
		#[cfg(feature = "server")]
		Sql(sqlx::Error),
	}
}

#[allow(unused)]
pub mod prelude {
	pub use super::api::*;
	pub use super::shared::*;
	#[cfg(feature = "server")]
	pub use super::util::*;
}