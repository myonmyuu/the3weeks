use crate::{components::navbar::Header, prelude::*, routes::{account::AccountRoutes, admin::AdminRoutes, chat::ChatRoutes, filesystem::FilesystemRoutes, home::Home, invalid::NotFound, login::Login, register::Register}, storage::init_storage};
use thrw_shared::{app::state::{client::LoginContext, shared::LoginState}, user::api::is_logged_in};

pub mod helpers {
	pub use super::{EmptyParent, EmptyView};
}

pub mod invalid;
pub mod home;
pub mod login;
pub mod register;
pub mod account;
pub mod admin;
pub mod chat;
pub mod filesystem;

pub fn shell(options: LeptosOptions) -> impl IntoView {
	view! {
		<!DOCTYPE html>
		<html lang="en">
			<head>
				<meta charset="utf-8"/>
				<meta name="viewport" content="width=device-width, initial-scale=1"/>
				<AutoReload options=options.clone()/>
				<HydrationScripts options/>
				<MetaTags/>
			</head>
			<body>
				<App/>
			</body>
		</html>
	}
}

#[component]
pub fn EmptyParent() -> impl IntoView {
	view! { <Outlet /> }
}

#[component]
pub fn EmptyView() -> impl IntoView {
	view! { Nothing to see here yet, slut }
}

fn add_context() {
	provide_meta_context();

	provide_context(LoginContext {
		// login state is verified on load
		login_state: RwSignal::new(LoginState::Unverified),
	});

	Effect::new(|| {
		init_storage();
	});
}

#[component]
pub fn App() -> impl IntoView {
	add_context();

	let get_login_state = Resource::new(||(), async |_| {
		let Ok(Some((id, token, level_name))) = is_logged_in().await else {
			return None;
		};
		Some((id, token, level_name))
	});

	// get the initial login state
	Effect::new(move || {
		let get_login_state = get_login_state;
		spawn_local(async move {
			let ctx = use_context::<LoginContext>().unwrap();
			let login = ctx.login_state;

			login.set(match get_login_state.await {
				Some((id, token, level_name)) => LoginState::LoggedIn(
					id,
					token,
					level_name.into()
				),
				None => LoginState::LoggedOut,
			});
		});
	});
	
	view! {
		<Stylesheet id="leptos" href="/pkg/thrw.css"/>

		<Title text="7sins"/>

		<Router>
			<Header />

			<main>
				<Routes fallback=NotFound>
					<ParentRoute path=path!("/") view=EmptyParent>
						<Route path=path!("/") view=Home/>
						<Route path=path!("/login") view=Login/>
						<Route path=path!("/register") view=Register />

						<FilesystemRoutes />

						<ChatRoutes />

						<AdminRoutes />

						<AccountRoutes />
					</ParentRoute>
				</Routes>
			</main>
		</Router>
	}
}