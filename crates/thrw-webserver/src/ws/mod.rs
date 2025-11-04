use std::{net::SocketAddr, str::FromStr};

use axum::{extract::{ws::{Message, Utf8Bytes, WebSocket}, ConnectInfo, State, WebSocketUpgrade}, http::{HeaderMap, StatusCode}, response::IntoResponse};
use axum_extra::extract::CookieJar;
use leptos::prelude::use_context;
use thrw_shared::{app::{cookie::values::SESSION_TOKEN, state::server::SharedAppState}, user::auth::{check_token_validity_and_refresh, check_token_validity_and_refresh_with_state}, ws::ThrwSocketMessage};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct WebSocketState {
	broadcaster: tokio::sync::broadcast::Sender<(ThrwSocketMessage, i32, SocketAddr)>,
}
impl Default for WebSocketState {
	fn default() -> Self {
		Self { broadcaster: tokio::sync::broadcast::channel(512).0 }
	}
}

#[derive(Debug, Clone)]
enum ClientWebsocketMessageError {
	NoMessage,
	ResultError,
}

#[derive(Debug, Clone)]
enum ClientWebsocketMessage {
	Ssins(ThrwSocketMessage),
	Other(Message),
	InvalidSsins,
	Err(ClientWebsocketMessageError),
}
impl<E> From<Option<Result<Message, E>>> for ClientWebsocketMessage {
	fn from(value: Option<Result<Message, E>>) -> Self {
		match value {
			Some(res) => res.into(),
			None => Self::Err(ClientWebsocketMessageError::NoMessage),
		}
	}
}
impl<E> From<Result<Message, E>> for ClientWebsocketMessage {
	fn from(value: Result<Message, E>) -> Self {
		match value {
			Ok(msg) => msg.into(),
			Err(_) => Self::Err(ClientWebsocketMessageError::ResultError),
		}
	}
}
impl From<Message> for ClientWebsocketMessage {
	fn from(value: Message) -> Self {
		match value {
			Message::Text(uft_bytes) => {
				let Ok(ws_msg) = serde_json::de::from_str::<ThrwSocketMessage>(&uft_bytes) else {
					return Self::Other(Message::Text(uft_bytes));
				};
				Self::Ssins(ws_msg)
			}
			other => Self::Other(other)
		}
	}
}


pub async fn handle_ws(
	ws: WebSocketUpgrade,
	headers: HeaderMap,
	ConnectInfo(addr): ConnectInfo<SocketAddr>,
	State(state): State<AppState>,
) -> impl IntoResponse {
	leptos::logging::log!("websocket connection requested for '{addr}'...");
	let token = match crate::cookie::try_extract_cookie(&headers, SESSION_TOKEN) {
		Ok(token) => Uuid::from_str(&token),
		Err(err) => {
			leptos::logging::log!("user authentication error: '{err:#?}'");
			return StatusCode::UNAUTHORIZED.into_response();
		},
	};

	let token = match token {
		Ok(id) => id,
		Err(err) => {
			leptos::logging::log!("user authentication error (invalid uuid): '{err:#?}'");
			return StatusCode::UNAUTHORIZED.into_response();
		},
	};

	if let Err(err) = check_token_validity_and_refresh_with_state(token, state.clone().shared).await {
		leptos::logging::log!("user authentication error (invalid session): '{err:#?}'");
		return StatusCode::UNAUTHORIZED.into_response();
	}

	ws
		.on_upgrade(move |socket| handle_socket(socket, addr, token, state))
		.into_response()
}

async fn handle_socket(mut socket: WebSocket, addr: SocketAddr, token: Uuid, state: AppState) {
	leptos::logging::log!("setting up socket for '{addr}'");
	
	let ws_state = state.socket;
	let mut receive = ws_state.broadcaster.subscribe();
	let sender = ws_state.broadcaster.clone();
	let char_id = {
		let ClientWebsocketMessage::Ssins(ThrwSocketMessage::Intoduce(char_id)) = ClientWebsocketMessage::from(socket.recv().await) else {
			leptos::logging::log!("socket '{addr}' did not introduce, closing");
			return;
		};

		// match character_belons_to_user(&state.shared.db_pool, token, char_id).await {
		// 	Ok(valid) => {
		// 		if !valid {
		// 			leptos::logging::log!("socket '{addr}' tried to log into non-owned character, closing connection");
		// 			return;
		// 		}

		// 		char_id
		// 	},
		// 	Err(err) => {
		// 		leptos::logging::log!("error validating character ownership, closing socket '{addr}': '{err}'");
		// 		return;
		// 	},
		// }
		1
	};

	let mut recv_task = tokio::spawn( async move {
		loop {
			match ClientWebsocketMessage::from(socket.recv().await) {
				ClientWebsocketMessage::Ssins(thrw_socket_message) => {
					if let Err(err) = sender.send((thrw_socket_message, char_id, addr)) {
						leptos::logging::log!("error transmitting client message: '{err}'");
					}
				},
				ClientWebsocketMessage::Other(message) => todo!(),
				ClientWebsocketMessage::InvalidSsins => todo!(),
				ClientWebsocketMessage::Err(err) => {
					leptos::logging::log!("error receiving client message: '{err:?}'");
					break;
				},
			}
		}
	});

	let mut send_task = tokio::spawn(async move {
		while let Ok(mes) = receive.recv().await {
			
		}
	});

	tokio::select! {
		_ = &mut recv_task => send_task.abort(),
		_ = &mut send_task => recv_task.abort(),
	}

	leptos::logging::log!("socket with address '{addr}' closing");
}