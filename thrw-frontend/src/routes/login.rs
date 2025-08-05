use thrw_shared::{app::state::{client::LoginContext, shared::LoginState}, user::api::log_in};

use crate::prelude::*;

#[component]
pub fn Login() -> impl IntoView {
	let login_ctx = use_context::<LoginContext>().expect("login context missing");
	let login = login_ctx.login_state;
	let (mail, set_mail) = signal("test".to_string());
	let (pass, set_pass) = signal("".to_string());
	// *login_ctx.login_state.write() = LoginState::LoggedOut;
	view! {
		<input type="text"
			bind:value=(mail, set_mail)
		/>
		<input type="password"
			bind:value=(pass, set_pass)
		/>
		<p>"Test: " {mail}</p>
		<button
			on:click=move |_| {
				let (mail, pass) = (
					mail(),
					pass()
				);
				let login = login;
				spawn_local(async move {
					let res = log_in(mail, pass).await;
					match res {
						Ok((id, uuid, level_name)) => {
							login.set(LoginState::LoggedIn(id, uuid, level_name.into()));
						},
						Err(err) => {
							log::debug!("login error: {err}");
						},
					};
				});
			}
		>
			Log in
		</button>
	}
}