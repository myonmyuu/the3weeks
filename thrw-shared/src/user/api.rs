use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::prelude::client::*;
use crate::user::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserKeyChain {
	pub key_name:		String,
	pub uses:			i16,
	pub entry_level:	i16,
}

#[server]
pub async fn generate_keychain(
	name: Option<String>,
	uses: Option<i16>,
	entry_level: Option<i16>,
) -> Result<UserKeyChain, ServerFnError> {
	use crate::app::state::prelude::extract_db;
	require_admin().await?;

	let db_pool = extract_db()?;

	let key_name = name.unwrap_or(uuid::Uuid::new_v4().to_string());
	let uses = uses.unwrap_or(1);
	let entry_level = match entry_level {
		Some(lvl) => lvl,
		None => {
			let res = sqlx::query!("
				SELECT level_id
				FROM user_levels
				WHERE level_name = 'user'
				;"
			)
				.fetch_one(&db_pool)
				.await?
			;
			res.level_id
		}
	};

	let res = sqlx::query!("
		INSERT INTO key_chains
			(key_name, uses, entry_level)
		VALUES
			($1, $2, $3)
		;",
		key_name,
		uses,
		entry_level
	)
		.execute(&db_pool)
		.await?
	;

	if res.rows_affected() != 1 {
		return Err(ServerFnError::ServerError("error creating keychain".to_string()));
	}

	Ok(UserKeyChain { key_name, uses, entry_level })
}

#[server]
pub async fn kill_keychain(keychain: String) -> Result<(), ServerFnError> {
	use crate::app::state::prelude::extract_db;
	require_admin().await?;

	let db_pool = extract_db()?;

	let res = sqlx::query!("
		DELETE FROM key_chains
		WHERE key_name = $1
		;",
		keychain
	)
		.execute(&db_pool)
		.await?
	;

	(res.rows_affected() > 0)
		.ok_or(ServerFnError::ServerError("keychain not found".to_string()))
}

#[server]
pub async fn get_active_keychains() -> Result<Vec<UserKeyChain>, ServerFnError> {
	use crate::app::state::prelude::extract_db;
	require_admin().await?;

	let db_pool = extract_db()?;
	sqlx::query_as!(
		UserKeyChain,
		"SELECT *
		FROM key_chains
		;"
	)
		.fetch_all(&db_pool)
		.await
		.map_err(Into::into)
}

#[server]
pub async fn get_user_level() -> Result<(i32, String), ServerFnError> {
	get_user_level_internal(None)
		.await
		.map_err(Into::into)
}

#[server]
pub async fn register(email: String, pw: String, keychain: String) -> Result<(), ServerFnError> {
	use crate::app::state::server::extract_db;
	let entry_level = use_keychain(keychain).await?;
	println!("user registering: {email}");
	let db_pool = extract_db()?;

	let exists = sqlx::query!("
		SELECT id
		FROM users
		where email = $1
		;",
		email.clone()
	)
		.fetch_optional(&db_pool)
		.await?
	;

	if exists.is_some() {
		return Err(ServerFnError::ServerError("Email is already registered".to_string()));
	}

	let pw_hash = hash(pw);
	let affected = sqlx::query!("
		INSERT INTO users
			(email, pwhash, user_level)
		VALUES
			($1, $2, $3)
		;",
		email,
		pw_hash,
		entry_level
	)
		.execute(&db_pool)
		.await?
	;

	if affected.rows_affected() == 0 {
		return Err(ServerFnError::ServerError("Error creating account".to_string()));
	}

	Ok(())
}

#[server]
pub async fn log_in(email: String, pw: String) -> Result<(i32, String, String), ServerFnError> {
	use crate::app::state::server::extract_db;
	use axum::extract::ConnectInfo;

	let ip: ConnectInfo<SocketAddr> = leptos_axum::extract().await?;

	if let Ok(token) = super::auth::extract_session_cookie().await
		&& check_token_validity_and_refresh(token).await.is_ok() {
			return Err(AuthError::Auth(LocalAuthError::AlreadyLoggedIn).into());
		}

	println!("user {email} ({}) logging in...", ip.0);
	// init db and get row
	let db_pool = extract_db()?;
	let acc_row = sqlx::query!("
		SELECT id, email, pwhash
		FROM users
		WHERE email = $1
		;",
		email.clone()
	)
		.fetch_optional(&db_pool)
		.await?
	;

	let Some(acc_row) = acc_row else {
		return Err(ServerFnError::ServerError("User does not exist".to_string()));
	};

	let pw_hash = acc_row.pwhash;
	let user_id = acc_row.id;
	
	match verify_hash(&pw, &pw_hash) {
		Ok(false) => return Err(ServerFnError::ServerError("Wrong password".to_string())),
		Err(e) => return Err(ServerFnError::ServerError(e)),

		Ok(true) => {},
	};

	let token = generate_auth_token(user_id, ip.0).await;
	let Ok(token) = token else {
		return Err(ServerFnError::ServerError(format!("Unable to generate auth token: {:?}", token.unwrap_err())));
	};

	refresh_token_cookie(token)?;

	println!("user {email} logged in!");

	let (_, level_name) = get_user_level_internal(Some(user_id)).await?;
	Ok((user_id, token.to_string(), level_name))
}

#[server]
pub async fn log_out() -> Result<(), ServerFnError> {
	let Ok((_, token)) = require_auth().await else {
		return Err(ServerFnError::ServerError("not logged in".to_string()));
	};
	end_session(token).await?;
	Ok(())
}

#[server]
pub async fn is_logged_in() -> Result<Option<(i32, String, String)>, ServerFnError> {
	match require_auth().await {
		// logged in
		Ok((id, token)) => {
			let (_, level_name) = get_user_level().await?;
			Ok(Some((id, token.to_string(), level_name)))
		},
		// no error, but logged out
		Err(AuthError::Auth(LocalAuthError::NotAuthenticated)) => Ok(None),
		// something went wrong
		Err(err) => Err(err.into())
	}
}