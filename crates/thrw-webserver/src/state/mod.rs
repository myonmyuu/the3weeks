use axum::extract::FromRef;
use leptos::config::LeptosOptions;
use thrw_shared::app::state::server::SharedAppState;

use crate::ws::WebSocketState;

#[derive(axum::extract::FromRef, Debug, Clone)]
pub struct AppState {
	pub shared: SharedAppState,
	pub socket: WebSocketState,
}
impl FromRef<AppState> for LeptosOptions {
	fn from_ref(input: &AppState) -> Self {
		input.shared.leptos_options.clone()
	}
}