use thrw_shared::{app::state::{client::LoginContext, shared::LoginState}, user::api::log_out};

use crate::prelude::*;

#[component]
pub fn Header() -> impl IntoView {
	let login_ctx = use_context::<LoginContext>().expect("login context missing");
	let login = login_ctx.login_state;
	
	view! {
		<header>
			<div class="links_l">
				<a href="/" class="logo" >The 3 Weeks</a>
			</div>
			<div class="links_r">
				<Show when=move || !matches!(check_login(None), Some(true)) fallback=move||view! {
					<a href="/admin">Admin</a>
					<a href="/account">Account</a>
					<button
						on:click= move |_| {
							let login = login;
							spawn_local(async move {
								if log_out().await.is_ok() {
									login.set(LoginState::LoggedOut)
								}
							});
						}
					>
						Logout
					</button>
				}>
					<a href="/register">Register</a>
					<a href="/login">Login</a>
				</Show>
			</div>
		</header>
	}
}

#[component]
pub fn Footer() -> impl IntoView {
	view! {
		<footer>
		</footer>
	}
}