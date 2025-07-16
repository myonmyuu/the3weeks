use std::collections::HashMap;

use cookie::time::UtcDateTime;
use leptos::{config::LeptosOptions, prelude::{use_context, ServerFnError}};
use sqlx::{Pool, Postgres};

#[derive(Debug, Clone, Default)]
pub struct UserData {
	pub active_tokens: HashMap<String, UtcDateTime>,
}

#[derive(axum::extract::FromRef, Debug, Clone)]
pub struct AppState {
    pub db_pool: Pool<Postgres>,
    pub leptos_options: LeptosOptions,
	pub user_data: UserData,
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
		Self::ServerError(format!("{:#?}", value))
	}
}

pub fn extract_state() -> Result<AppState, ExtractError> {
	let state = use_context::<AppState>()
		.ok_or(LocalExtractError::NotAvailable("App state missing".into()))?;

	Ok(state)
}

pub fn extract_db() -> Result<Pool<Postgres>, ExtractError> {
	let state = extract_state()?;

	Ok(state.db_pool)
}