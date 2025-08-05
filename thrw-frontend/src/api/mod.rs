use crate::prelude::*;

#[server]
pub async fn server_print(str: String) -> Result<(), ServerFnError> {
	use thrw_shared::app::cookie::server::get_cookie_jar;
	
	// let response_ctx = use_context::<ResponseOptions>()
	// 	.ok_or::<ServerFnError>(ServerFnError::ServerError("No response options".into()))?;
	
	// let db = use_context::<thrw_shared::app::state::AppState>()
	// 	.ok_or::<ServerFnError>(ServerFnError::ServerError("No access to app state".into()))?;
	// // let cookies = use_context::<Cook()

	let db = thrw_shared::app::state::server::extract_db()?;
	println!("db connections: {}", db.size());

	let jar = get_cookie_jar().await?;
	println!("cookie jar: {jar:?}");

	println!("{str}");
	Ok(())
}
