use axum::http::HeaderMap;
use cookie::Cookie;

pub mod names {
	pub const COOKIE: &str = "cookie";
}

#[derive(Debug, Clone)]
pub enum CookieError {
	NotFound,
	NoCookies,
	Stringify,
}

pub fn try_extract_cookie(headers: &HeaderMap, key: &str) -> Result<String, CookieError> {
	// println!("{headers:?}");
	let Some(cookie_header) = headers.get(names::COOKIE) else {
		return Err(CookieError::NoCookies);
	};

	let Ok(cookie_str) = cookie_header.to_str() else {
		return Err(CookieError::Stringify);
	};

	for cookie in cookie_str.split(";") {
		let Ok(cookie) = Cookie::parse(cookie) else {
			continue;
		};

		if cookie.name() == key {
			return Ok(cookie.value().to_string());
		}
	}

	Err(CookieError::NotFound)
}