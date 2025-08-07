use std::{fmt::Debug, time::Duration};

#[cfg(feature = "server")]
use std::time::Instant;
#[cfg(feature = "server")]
use argon2::{password_hash::{PasswordHasher, PasswordVerifier, rand_core::OsRng, SaltString}, Argon2};

use leptos::prelude::ServerFnError;
#[cfg(not(feature = "server"))]
use web_sys::window;

pub struct InstantWrapper {
	#[cfg(feature = "server")]
	time: Instant,
	#[cfg(not(feature = "server"))]
	start: f64
}
impl InstantWrapper {
	pub fn now() -> Self {
		#[cfg(feature = "server")]
		{
			Self { time: Instant::now() }
		}
		#[cfg(not(feature = "server"))]
		{
			let perf = window()
				.expect("no global `window` exists")
				.performance()
				.expect("performance should be available");
			Self { start: perf.now() }
		}
	}

	pub fn elapsed(&self) -> f64 {
		#[cfg(feature = "server")]
		{
			self.time.elapsed().as_secs_f64()
		}
		#[cfg(not(feature = "server"))]
        {
			(Self::now().start - self.start) / 1000.0
		}
    }
}

/// Hash with a random salt string
pub fn hash(v: String) -> String {
	#[cfg(feature = "server")]
	{
		let arg = Argon2::default();
		let salt = SaltString::generate(&mut OsRng);
		arg.hash_password(v.as_bytes(), &salt).unwrap().to_string()
	}
	#[cfg(not(feature = "server"))]
	{
		v
	}
}

pub fn verify_hash(pw: &String, pw_hash: &str) -> Result<bool, String> {
	#[cfg(feature = "server")]
	{
		let parsed_hash = argon2::PasswordHash::new(pw_hash)
			.map_err(|e| e.to_string())?;
		if Argon2::default().verify_password(pw.as_bytes(), &parsed_hash).is_err() {
			return Ok(false);
		}
		Ok(true)
	}
	#[cfg(not(feature = "server"))]
	{
		Err("Can't verify hashes on client".to_string())
	}
}

pub async fn wait_until<F>(mut check: F, timeout_secs: f64) -> Result<(), ()>
where
	F: FnMut() -> bool,
{
	let start = InstantWrapper::now();
	while start.elapsed() < timeout_secs {
		if check() {
			return Ok(())
		}

		#[cfg(feature = "server")]
		tokio::time::sleep(Duration::from_millis(50)).await;
		#[cfg(not(feature = "server"))]
		gloo_timers::future::sleep(Duration::from_millis(50)).await;
	}
	Err(())
}

pub fn make_server_err<T: Debug>(err: T) -> ServerFnError {
	ServerFnError::ServerError(format!("{err:?}"))
}