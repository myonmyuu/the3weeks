use std::{fmt::Display, str::FromStr};

use axum::http::{header::{InvalidHeaderName, InvalidHeaderValue}, HeaderName, HeaderValue};
use chrono::{DateTime, Utc};
use cookie::{time::OffsetDateTime, Cookie};
use leptos::{prelude::{use_context, ServerFnError}, server};
use leptos_axum::ResponseOptions;
use sqlx::Row;
use uuid::Uuid;

use crate::app::{cookie::server::{get_cookie_jar, CookieError}, state::{extract_db, extract_state, ExtractError}};

pub mod values {
	pub const SESSION_LENGTH: i64 = 30;
}

#[derive(Debug)]
pub enum LocalAuthError {
	NotAuthenticated,
	Invalid,
	Expired,
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
		HeaderValue(InvalidHeaderValue)
	}
}

impl From<AuthError> for ServerFnError {
	fn from(value: AuthError) -> Self {
		Self::ServerError(format!("{:?}", value))
	}
}

pub async fn remove_expired_sessions() -> Result<(), AuthError> {
	let db_pool = extract_db()?;

	sqlx::query(
		"DELETE FROM sessions WHERE expires_at < $1;"
	)
		.bind(chrono::offset::Utc::now())
		.execute(&db_pool)
		.await?
	;

	Ok(())
}

fn get_next_expiry_time() -> DateTime<Utc> {
	chrono::offset::Utc::now() + chrono::TimeDelta::minutes(values::SESSION_LENGTH)
}

pub async fn end_session(token: Uuid) -> Result<(), AuthError> {
	let db_pool = extract_db()?;

	// execution does not return on error, we want to write the cookies either way
	let _del_query_res = sqlx::query(
		"DELETE FROM sessions WHERE session_id = $1;"
	)
		.bind(token)
		.execute(&db_pool)
		.await
	;
	let response_options = use_context::<ResponseOptions>()
		.ok_or::<ServerFnError>(ServerFnError::ServerError("No response options".into()))?;
	
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
	Ok(())

}

pub async fn check_token_validity_and_refresh(token: Uuid) -> Result<(), AuthError> {
	let db_pool = extract_db()?;

	let token_row = sqlx::query(
		"SELECT expires_at FROM sessions WHERE session_id = $1;"
	)
		.bind(token)
		.fetch_optional(&db_pool)
		.await?
	;

	let Some(token_row) = token_row else {
		return Err(AuthError::Auth(LocalAuthError::NotAuthenticated));
	};

	let expires_at: DateTime<Utc> = token_row.try_get("expires_at")?;
	if chrono::offset::Utc::now() > expires_at {
		end_session(token).await?;
		return Err(AuthError::Auth(LocalAuthError::Expired));
	}

	refresh_token(token).await?;
	refresh_token_cookie(token)?;

	Ok(())
}

pub fn refresh_token_cookie(token: Uuid) -> Result<(), AuthError> {
	let response_options = use_context::<ResponseOptions>()
		.ok_or::<ServerFnError>(ServerFnError::ServerError("No response options".into()))?;
	
	let cookie = Cookie::build(
		(crate::app::cookie::values::SESSION_TOKEN, token.to_string())
	)
		.path("/")
		.http_only(true)
		.same_site(cookie::SameSite::Lax)
		.expires(OffsetDateTime::now_utc() + cookie::time::Duration::minutes(crate::user::auth::values::SESSION_LENGTH))
		.build()
	;

	response_options.append_header(
		HeaderName::from_str(crate::app::cookie::values::SET_COOKIE)?,
		HeaderValue::from_str(&cookie.to_string())?
	);

	Ok(())
}

pub async fn refresh_token(token: Uuid) -> Result<(), AuthError> {
	let db_pool = extract_db()?;

	sqlx::query(
		"UPDATE sessions SET expires_at = $1 WHERE session_id = $2;"
	)
		.bind(get_next_expiry_time())
		.bind(token)
		.execute(&db_pool)
		.await?
	;

	Ok(())
}

pub async fn generate_auth_token(user_id: i32) -> Result<String, AuthError> {
	let db_pool = extract_db()?;

	let uuid = uuid::Uuid::new_v4();
	sqlx::query(
		"INSERT INTO sessions
		(session_id, user_id, expires_at)
		VALUES
		($1, $2, $3);"
	)
		.bind(uuid)
		.bind(user_id)
		.bind(get_next_expiry_time())
		.execute(&db_pool)
		.await?
	;

	Ok(uuid.to_string())
}

pub async fn require_auth() -> Result<Uuid, AuthError> {
	let jar = get_cookie_jar()
		.await?;

	let session_cookie = jar
		.get(crate::app::cookie::values::SESSION_TOKEN)
	;

	let Some(session_cookie) = session_cookie else {
		return Err(AuthError::Auth(LocalAuthError::NotAuthenticated));
	};

	let session_id = session_cookie.value();
	let token = uuid::Uuid::from_str(&session_id)?;
	check_token_validity_and_refresh(token).await?;

	Ok(token)
}