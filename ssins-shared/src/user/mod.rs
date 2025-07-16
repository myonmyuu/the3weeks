use std::str::FromStr;
use leptos::prelude::*;

#[cfg(feature = "server")]
use argon2::{password_hash::{PasswordHasher, PasswordVerifier, rand_core::OsRng, SaltString}, Argon2};

#[cfg(feature = "server")]
pub mod auth;

#[derive(Clone)]
pub enum UserId {
	SSins(u32),
}

#[cfg(feature = "server")]
fn hash(v: String) -> String {
	let arg = Argon2::default();
	let salt = SaltString::generate(&mut OsRng);
	arg.hash_password(v.as_bytes(), &salt).unwrap().to_string()
}

#[server]
pub async fn register(email: String, pw: String) -> Result<(), ServerFnError> {
	use crate::app::state::extract_db;
	println!("user registering: {}", email);
	let db_pool = extract_db()?;

	let exists = sqlx::query(
		"SELECT id FROM users where email = $1;"
	)
		.bind(email.clone())
		.fetch_optional(&db_pool)
		.await?
	;

	if exists.is_some() {
		return Err(ServerFnError::ServerError("Email is already registered".to_string()));
	}

	let pw_hash = hash(pw);
	let affected = sqlx::query(
		"INSERT INTO users
		(email, pwhash)
		VALUES
		($1, $2);"
	)
		.bind(email)
		.bind(pw_hash)
		.execute(&db_pool)
		.await?
	;

	if affected.rows_affected() == 0 {
		return Err(ServerFnError::ServerError("Error creating account".to_string()));
	}

	Ok(())
}

#[server]
pub async fn log_in(email: String, pw: String) -> Result<(), ServerFnError> {
	use sqlx::Row;
	use leptos_axum::ResponseOptions;
	use axum::http::{HeaderName, HeaderValue};
	use cookie::{time::{Duration, OffsetDateTime}, Cookie};
	use crate::app::state::extract_db;

	println!("user logging in: {}", email);
	// init db and get row
	let db_pool = extract_db()?;
	let acc_row = sqlx::query(
	"SELECT id, email, pwhash FROM users
		WHERE
		email = $1;"
	)
		.bind(email)
		.fetch_optional(&db_pool)
		.await?
	;

	let Some(acc_row) = acc_row else {
		return Err(ServerFnError::ServerError("User does not exist".to_string()));
	};

	let row_text = acc_row.columns()
		.iter()
		.map(|c| format!("{:?}", c))
		.collect::<Vec<_>>()
		.join(", ");
	println!("{row_text}");
	let pw_hash: String = acc_row.try_get("pwhash")?;
	let user_id: i32 = acc_row.try_get("id")?;
	let parsed_hash = argon2::PasswordHash::new(&pw_hash)
		.map_err(|e| ServerFnError::ServerError::<server_fn::error::NoCustomError>(e.to_string()))?;
	
	if let Err(_) = Argon2::default().verify_password(pw.as_bytes(), &parsed_hash) {
		return Err(ServerFnError::ServerError("Wrong password".to_string()));
	}

	let token = auth::generate_auth_token(user_id).await;
	let Ok(token) = token else {
		return Err(ServerFnError::ServerError(format!("Unable to generate auth token: {:?}", token.unwrap_err())));
	};

	Ok(())
}

#[server]
pub async fn log_out() -> Result<(), ServerFnError> {
	let Ok(token) = auth::require_auth().await else {
		return Err(ServerFnError::ServerError("not logged in".to_string()));
	};
	auth::end_session(token).await?;
	Ok(())
}

#[server]
pub async fn is_logged_in() -> Result<(), ServerFnError> {
	if auth::require_auth().await.is_err() {
		return Err(ServerFnError::ServerError("not logged in".to_string()));
	}
	Ok(())
}