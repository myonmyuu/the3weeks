use web_sys::WebSocket;

use crate::{prelude::*, storage::logs::ClientLog};

#[component]
pub fn Home() -> impl IntoView {
	fn connect() {
		match WebSocket::new("ws://127.0.0.1:3000/ws/chat") {
			Ok(_) => log::debug!("connected?"),
			Err(err) => log::debug!("error connecting to socket: {err:?}"),
		};
	}
	view! {
		Yoooooooooo
		<button
			on:click=move |_| {
				spawn_local(async {
					let res = crate::storage::store_value(
						crate::storage::names::LOG_STORE,
						None,
						ClientLog { id: None, room: "kek".to_string(), time: js_sys::Date::now() }
					).await;
					log::debug!("test add res: {res:?}");
				});
			}
		>
			add test
		</button>
		<button
			on:click=move |_| {
				spawn_local(async {
					let res = crate::storage::get_value::<ClientLog>(
						crate::storage::names::LOG_STORE,
						crate::storage::IDBKey::Int(20)
					).await;	
					log::debug!("test get res: {res:?}");
				});
			}
		>
			get test
		</button>
		<button
			on:click=move |_| {
				connect();
			}
		>
			connectttt
		</button>
	}
}