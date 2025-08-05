use thrw_shared::user::api::{generate_keychain, get_active_keychains, kill_keychain};

use crate::{prelude::*, routes::admin::consts::KEY_LIST_ID};

#[component]
fn KeyList() -> impl IntoView {
	let key_list_ev = ReviewEvent::<{KEY_LIST_ID}>::use_provided();
	let key_list_res = Resource::new(
		key_list_ev.subscribe(),
		async |_| {
			get_active_keychains()
				.await
				.unwrap_or(vec![])
		}
	);
	view! {
		<Transition fallback=move || view! { <p>Loading...</p>}>
		{move || {
			key_list_res.get().map(|keys| {
				view! {
					<p>Keys: </p>
					<ul>
					{keys.iter().map(|key| {
						let key = key.clone();
						view! {
							<li>""{key.key_name.clone()}": entry level="{key.entry_level}"   uses="{key.uses}
								<button
									on:click= {
										move |_| {
											let key = key.clone();
											spawn_local(async move {
												let res = kill_keychain(key.key_name).await;
												if res.is_ok() {
													key_list_ev.invalidate();
												}
											});
										}
									}
								>
									X
								</button>
							</li>
						}
					}).collect_view()}
					</ul>
				}
			})
		}}
		</Transition>
	}
}

#[component]
pub fn KeyManager() -> impl IntoView {
	let key_list_ev = ReviewEvent::<{KEY_LIST_ID}>::use_provided();



	view! {
		<KeyList />

		<button
			on:click=move |_| {
				spawn_local(async move {
					let gen_res = generate_keychain(None, Some(3), None).await;
					if gen_res.is_ok() {
						key_list_ev.invalidate();
					}
				});
			}
		>
			Generate Key
		</button>
	}
}