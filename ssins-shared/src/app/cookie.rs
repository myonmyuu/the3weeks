#[cfg(feature = "server")]
pub mod server {
	use axum_extra::extract::cookie::CookieJar;
	use leptos::prelude::ServerFnError;

	#[derive(Debug)]
	pub enum LocalCookieError {
		NoJar,
		Missing,
	}

	crate::make_error_type! {
		pub CookieError {
			Local(LocalCookieError),
		}
	}

	impl From<CookieError> for ServerFnError {
		fn from(value: CookieError) -> Self {
			Self::ServerError(format!("{:#?}", value))
		}
	}

	pub async fn get_cookie_jar() -> Result<CookieJar, CookieError> {
		match leptos_axum::extract::<CookieJar>()
			.await {
				Ok(jar) => Ok(jar),
				Err(_) => Err(CookieError::Local(LocalCookieError::NoJar)),
			}
	}
}

pub mod values {
	pub const SET_COOKIE: &str = "Set-Cookie";
	pub const SESSION_TOKEN: &str = "ssins_session";
}