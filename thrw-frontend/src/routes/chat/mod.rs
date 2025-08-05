use std::{cell::RefCell, rc::Rc, sync::Arc};

use codee::string::JsonSerdeCodec;
use leptos_use::{core::ConnectionReadyState, use_websocket, use_websocket_with_options, UseWebSocketOptions, UseWebSocketReturn};
use thrw_shared::{util::wait_until, ws::ThrwSocketMessage};
use tokio::sync::oneshot::{self, error::RecvError};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{ErrorEvent, Event, WebSocket};

use crate::prelude::*;

pub(self) mod consts {
	const CHAT_IDS: i32 = 10000;
	pub const ID_: i32 = CHAT_IDS + 1;
}

#[derive(Debug, Clone)]
pub(self) enum ChatConnectState {
	None,
	Connecting,
	Socket,
	Character(i32)
}

pub enum ClientChatError {
	SocketSetup(JsValue),
	SocketConnect,
	SocketRecv(RecvError)
}

#[derive(Clone)]
pub(self) struct LoggedInCharacter {
	pub id: i32,
}

#[derive(Clone)]
pub(self) struct WebsocketContext {
	pub state: Signal<ConnectionReadyState>,
	pub message: Signal<Option<ThrwSocketMessage>>,
    send: Arc<dyn Fn(&ThrwSocketMessage) + Send + Sync>,
	close: Arc<dyn Fn() + Send + Sync + 'static,>
}
impl WebsocketContext {
	pub fn new(
		message: Signal<Option<ThrwSocketMessage>>,
		state: Signal<ConnectionReadyState>,
		send: Arc<dyn Fn(&ThrwSocketMessage) + Send + Sync>,
		close: Arc<dyn Fn() + Send + Sync + 'static,> 
	) -> Self {
        Self {
            message,
			state,
            send,
			close
        }
    }

    // create a method to avoid having to use parantheses around the field
    #[inline(always)]
    pub fn send(&self, message: &ThrwSocketMessage) {
        (self.send)(message)
    }

	#[inline(always)]
	pub fn close(&self) {
		(self.close)()
	}
}

async fn setup_ws_connection() -> Result<(), ClientChatError> {
	let UseWebSocketReturn {
		message,
		send,
		ready_state,
		close,
		..
	} = use_websocket_with_options::<ThrwSocketMessage, ThrwSocketMessage, JsonSerdeCodec, _, _>(
		"/ws/chat",
		UseWebSocketOptions::default()
			.on_error(|err| {
				log::debug!("socket err: {err}");
			})
			.reconnect_limit(leptos_use::ReconnectLimit::Limited(0))
	);

	let con_res = wait_until(
		|| matches!(ready_state.get_untracked(), leptos_use::core::ConnectionReadyState::Open),
		4.0
	).await;

	match con_res {
		Ok(_) => {
			let ctx = WebsocketContext::new(
				message,
				ready_state,
				Arc::new(send.clone()),
				Arc::new(close.clone())
			);
			provide_context(ctx);

			Ok(())
		},
		Err(_) => Err(ClientChatError::SocketConnect),
	}
}

pub(self) fn is_websocket_connected() -> impl Fn() -> bool {
	move || {
		let Some(ctx) = use_context::<WebsocketContext>() else {
			return false;
		};

		matches!(ctx.state.get(), ConnectionReadyState::Open)
	}
}

#[component]
fn ChatConnect() -> impl IntoView {
	let connect_state_signal = use_context::<RwSignal<ChatConnectState>>().unwrap();
	view! {
		<button
			disabled=move||!matches!(connect_state_signal(), ChatConnectState::None)
			on:click=move |_| {
				connect_state_signal(ChatConnectState::Connecting);
				spawn_local(async move {
					match setup_ws_connection().await {
						Ok(_) => connect_state_signal(ChatConnectState::Socket),
						Err(_) => connect_state_signal(ChatConnectState::None),
					};
				});
			}
		>
			connect
		</button>
	}
}

#[component]
fn ChatRoot() -> impl IntoView {
	let chat_state_signal = RwSignal::new(ChatConnectState::None);
	let _ = Effect::new(move |_| {
		if matches!(chat_state_signal(), ChatConnectState::None)
		&& let Some(ctx) = use_context::<WebsocketContext>() {
			ctx.close();
		}
	});
	provide_context(chat_state_signal);
	view! {
		<Show when=move||matches!(chat_state_signal(), ChatConnectState::None)>
			<ChatConnect />
		</Show>
	}
}

#[component(transparent)]
pub fn ChatRoutes() -> impl MatchNestedRoutes + Clone {
	view! {
		<ProtectedParentRoute
			path=path!("/chat")
			view=EmptyParent
			condition=check_login_raw
			redirect_path=||"/login"
		>
			<Route path=path!("/") view=ChatRoot />
		</ProtectedParentRoute>
	}
	.into_inner()
}