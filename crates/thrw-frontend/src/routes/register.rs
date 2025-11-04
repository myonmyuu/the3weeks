use thrw_shared::user::api::register;

use crate::prelude::*;

#[component]
pub fn Register() -> impl IntoView {
	let (mail, set_mail) = signal("".to_string());
	let keychain = RwSignal::new("".to_string());
	let (pass, set_pass) = signal("".to_string());
	let (pass_c, set_pass_c) = signal("".to_string());

	view! {
		Register
		<input type="text"
			bind:value=keychain
		/>
		<input type="text"
			bind:value=(mail, set_mail)
		/>
		<input type="password"
			bind:value=(pass, set_pass)
		/>
		<input type="password"
			bind:value=(pass_c, set_pass_c)
		/>
		<button
			on:click=move |_| {
				let (key, mail, pass, pass_c) = (
					keychain(),
					mail(),
					pass(),
					pass_c()
				);
				if pass != pass_c {
					//TODO: show an error
					return;
				}
				spawn_local(async {
					let res = register(mail, pass, key).await;
					if res.is_err() {
						log::debug!("{}", res.unwrap_err());
					}
				});
			}
		>
			Register
		</button>
	}
}