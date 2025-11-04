use std::{net::SocketAddr, str::FromStr};

use axum::http::{header::{InvalidHeaderName, InvalidHeaderValue}, HeaderName, HeaderValue};
use chrono::{DateTime, Utc};
use cookie::{time::OffsetDateTime, Cookie};
use leptos::prelude::{use_context, ServerFnError};
use leptos_axum::ResponseOptions;
use sqlx::Row;
use uuid::Uuid;

use crate::app::{cookie::server::{get_cookie_jar, CookieError}, state::server::{extract_db, extract_state, ExtractError, SharedAppState}};

pub mod values {
	pub const SESSION_LENGTH: i64 = 30;
}

#[derive(Debug)]
pub enum LocalAuthError {
	NotAuthenticated,
	Forbidden,
	Invalid,
	Expired,
	AlreadyLoggedIn,
	NoKeyChain,
	InvalidUserLevel,
	AdminLevelNotInDb,
}

crate::make_error_type!{
	pub AuthError {
		Auth(LocalAuthError),
		Cookie(CookieError),
		Extract(ExtractError),
		Sql(sqlx::Error),
		Uuid(uuid::Error),
		Server(ServerFnError),
		HeaderName(InvalidHeaderName),
		HeaderValue(InvalidHeaderValue),
	}
}

impl From<AuthError> for ServerFnError {
	fn from(value: AuthError) -> Self {
		Self::ServerError(format!("{value:?}"))
	}
}

pub async fn use_keychain(keychain: String) -> Result<i16, AuthError> {
	let db_pool = extract_db()?;

	let keychain = sqlx::query!("
		SELECT *
		FROM key_chains
		WHERE key_name = $1
		;",
		keychain
	)
		.fetch_optional(&db_pool)
		.await?
		.ok_or(LocalAuthError::NoKeyChain)?
	;

	sqlx::query!("
		UPDATE key_chains
		SET uses = $1
		WHERE key_name = $2
		;",
		keychain.uses - 1,
		keychain.key_name
	)
		.execute(&db_pool)
		.await?
	;

	Ok(keychain.entry_level)
}

// only available in requests
pub async fn get_user_level_internal(id: Option<i32>) -> Result<(i32, String), AuthError> {
	let id = match id {
		Some(id) => id,
		None => require_auth().await?.0,
	};
	let db_pool = extract_db()?;

	let level_rec = sqlx::query!("
		SELECT user_levels.level_id, user_levels.level_name
		FROM users
		JOIN user_levels ON user_levels.level_id = users.user_level
		WHERE users.id = $1
		;",
		id
	)
		.fetch_one(&db_pool)
		.await?
	;

	Ok((level_rec.level_id as i32, level_rec.level_name))
}

pub async fn remove_expired_sessions() -> Result<(), AuthError> {
	let db_pool = extract_db()?;

	sqlx::query!("
		DELETE FROM sessions
		WHERE expires_at < $1
		;",
		chrono::offset::Utc::now()
	)
		.execute(&db_pool)
		.await?
	;

	Ok(())
}

fn get_next_expiry_time() -> DateTime<Utc> {
	chrono::offset::Utc::now() + chrono::TimeDelta::minutes(values::SESSION_LENGTH)
}

pub async fn end_session_with_state(token: Uuid, state: SharedAppState) -> Result<(), AuthError> {
	let db_pool = state.db_pool;

	// execution does not return on error, we want to write the cookies either way
	let _del_query_res = sqlx::query!("
		DELETE FROM sessions
		WHERE session_id = $1
		;",
		token
	)
		.execute(&db_pool)
		.await
	;
	if let Some(response_options) = use_context::<ResponseOptions>() {
		let cookie = Cookie::build(
			(crate::app::cookie::values::SESSION_TOKEN, "")
		)
			.path("/")
			.http_only(true)
			.same_site(cookie::SameSite::Lax)
			.expires(OffsetDateTime::now_utc() + cookie::time::Duration::minutes(-1))
			.build()
		;
		
		response_options.append_header(
			HeaderName::from_str(crate::app::cookie::values::SET_COOKIE)?,
			HeaderValue::from_str(&cookie.to_string())?
		);
	}
	Ok(())

}

pub async fn end_session(token: Uuid) -> Result<(), AuthError> {
	end_session_with_state(token, extract_state()?).await
}

// pub async  fn check_token_validity(token: Uuid) 
pub async fn check_token_validity_and_refresh_with_state(token: Uuid, state: SharedAppState) -> Result<(i32, Uuid), AuthError> {
	let db_pool = state.db_pool;

	let token_row = sqlx::query!("
		SELECT expires_at, user_id
		FROM sessions
		WHERE session_id = $1
		;",
		token
	)
		.fetch_optional(&db_pool)
		.await?
	;

	let Some(token_row) = token_row else {
		return Err(AuthError::Auth(LocalAuthError::NotAuthenticated));
	};

	let expires_at: DateTime<Utc> = token_row.expires_at;
	if chrono::offset::Utc::now() > expires_at {
		end_session(token).await?;
		return Err(AuthError::Auth(LocalAuthError::Expired));
	}

	refresh_token(token, Some(db_pool)).await?;
	refresh_token_cookie(token)?;

	Ok((token_row.user_id, token))
}

pub async fn check_token_validity_and_refresh(token: Uuid) -> Result<(i32, Uuid), AuthError> {
	check_token_validity_and_refresh_with_state(token, extract_state()?).await
}

pub fn refresh_token_cookie(token: Uuid) -> Result<(), AuthError> {
	if let Some(response_options) = use_context::<ResponseOptions>() {
		let cookie = Cookie::build(
			(crate::app::cookie::values::SESSION_TOKEN, token.to_string())
		)
			.path("/")
			.http_only(false)
			.same_site(cookie::SameSite::Lax)
			.expires(OffsetDateTime::now_utc() + cookie::time::Duration::minutes(crate::user::auth::values::SESSION_LENGTH))
			.build()
		;

		response_options.append_header(
			HeaderName::from_str(crate::app::cookie::values::SET_COOKIE)?,
			HeaderValue::from_str(&cookie.to_string())?
		);
	}

	Ok(())
}

pub async fn refresh_token(token: Uuid, db_pool: Option<sqlx::Pool<sqlx::Postgres>>) -> Result<(), AuthError> {
	let db_pool = match db_pool {
		Some(p) => p,
		None => extract_db()?,
	};

	sqlx::query!("
		UPDATE sessions
		SET expires_at = $1
		WHERE session_id = $2
		;",
		get_next_expiry_time(),
		token
	)
		.execute(&db_pool)
		.await?
	;

	Ok(())
}

pub async fn generate_auth_token(user_id: i32, addr: SocketAddr) -> Result<Uuid, AuthError> {
	let db_pool = extract_db()?;

	let uuid = uuid::Uuid::new_v4();
	let mut addr = addr;
	addr.set_port(0);
	sqlx::query!("
		INSERT INTO sessions
		(session_id, user_id, expires_at, ip_address)
		VALUES
		($1, $2, $3, $4)
		;",
		uuid,
		user_id,
		get_next_expiry_time(),
		addr.to_string()
	)
		.execute(&db_pool)
		.await?
	;

	Ok(uuid)
}

pub async fn extract_session_cookie() -> Result<Uuid, AuthError> {
	let jar = get_cookie_jar()
		.await?;

	let session_cookie = jar
		.get(crate::app::cookie::values::SESSION_TOKEN)
	;

	let Some(session_cookie) = session_cookie else {
		return Err(AuthError::Auth(LocalAuthError::NotAuthenticated));
	};

	let session_id = session_cookie.value();
	Ok(uuid::Uuid::from_str(session_id)?)
}

// notify that authentication is required; only available in requests
pub async fn require_auth() -> Result<(i32, Uuid), AuthError> {
	let token = extract_session_cookie().await?;
	let (id, _) = check_token_validity_and_refresh(token).await?;

	Ok((id, token))
}

async fn get_admin_level() -> Result<i32, AuthError> {
	let state = extract_state()?;
	let admin_level = {
		*state.user_data.admin_level.lock().await
	};
	match admin_level {
		Some(id) => Ok(id),
		None => {
			let level = sqlx::query!("
				SELECT level_id AS id, level_name
				FROM user_levels
				WHERE level_name = 'admin'
				;"
			)
				.fetch_one(&state.db_pool)
				.await?
			;

			*state.user_data.admin_level.lock().await = Some(level.id as i32);
			Ok(level.id as i32)
		},
	}
}

pub async fn require_admin() -> Result<(), AuthError> {
	let (user_level, _) = get_user_level_internal(None).await?;
	(
		user_level >= get_admin_level()
			.await?
	)
		.ok_or(AuthError::Auth(LocalAuthError::Forbidden))
}