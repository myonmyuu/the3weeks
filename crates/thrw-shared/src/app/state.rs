pub mod prelude {
	#[cfg(not(feature = "server"))]
	pub use super::client::*;
	pub use super::shared::*;
	#[cfg(feature = "server")]
	pub use super::server::*;
}

pub mod client {
    use leptos::prelude::RwSignal;

	#[derive(Debug, Clone, Default)]
	pub struct LoginContext {
		pub login_state: RwSignal<super::shared::LoginState>,
	}
}

pub mod shared { 
	#[derive(Debug, Clone, Default)]
	pub enum LoginState {
		#[default]
		Unverified,
		LoggedOut,
		LoggedIn(i32, String, AccountLevel),
	}

	#[derive(Debug, Clone, Default)]
	pub enum AccountLevel {
		#[default]
		User,
		Admin
	}

	impl From<String> for AccountLevel {
		fn from(value: String) -> Self {
			match value.as_str() {
				"user" => Self::User,
				"admin" => Self::Admin,
				_ => Self::User
			}
		}
	}
}

#[cfg(feature = "server")]
pub mod server {
	use std::sync::Arc;
	use tokio::sync::Mutex;

	use leptos::{config::LeptosOptions, prelude::{use_context, ServerFnError}};
	use sqlx::{Pool, Postgres};

	use crate::app::media_request::DownloaderContext;

	#[derive(Debug, Clone, Default)]
	pub struct UserData {
		pub name_inspection_lock: Arc<Mutex<()>>,
		pub admin_level: Arc<Mutex<Option<i32>>>,
	}

	#[derive(axum::extract::FromRef, Debug, Clone)]
	pub struct SharedAppState {
		pub db_pool: Pool<Postgres>,
		pub leptos_options: LeptosOptions,
		pub user_data: UserData,
		pub dl_context: DownloaderContext,
	}

	#[derive(Debug)]
	pub enum LocalExtractError {
		NotAvailable(String),
	}

	crate::make_error_type!{
		pub ExtractError {
			Local(LocalExtractError),
		}
	}

	impl From<ExtractError> for ServerFnError {
		fn from(value: ExtractError) -> Self {
			Self::ServerError(format!("{value:#?}"))
		}
	}

	pub fn extract_state() -> Result<SharedAppState, ExtractError> {
		let state = use_context::<SharedAppState>()
			.ok_or(LocalExtractError::NotAvailable("App state missing".into()))?;

		Ok(state)
	}

	pub fn extract_db() -> Result<Pool<Postgres>, ExtractError> {
		let state = extract_state()?;

		Ok(state.db_pool)
	}
}
