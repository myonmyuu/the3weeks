use argon2::Argon2;
use leptos::{prelude::*, reactive::spawn_local};

use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes}, path, StaticSegment
};
use ssins_shared::user::{log_in, register};

#[server]
pub async fn server_print(str: String) -> Result<(), ServerFnError> {
	use leptos_axum::ResponseOptions;
	use sqlx::{Pool, Postgres};
	use ssins_shared::app::cookie::server::get_cookie_jar;
	
	// let response_ctx = use_context::<ResponseOptions>()
	// 	.ok_or::<ServerFnError>(ServerFnError::ServerError("No response options".into()))?;
	
	// let db = use_context::<ssins_shared::app::state::AppState>()
	// 	.ok_or::<ServerFnError>(ServerFnError::ServerError("No access to app state".into()))?;
	// // let cookies = use_context::<Cook()

	let db = ssins_shared::app::state::extract_db()?;
	println!("db connections: {}", db.size());

	let jar = get_cookie_jar().await?;
	println!("cookie jar: {:?}", jar);

	println!("{}", str);
	Ok(())
}

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
pub fn App() -> impl IntoView {
	provide_meta_context();
	
	view! {
        <Stylesheet id="leptos" href="/pkg/ssins.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=NotFound>
					<Route path=path!("/") view=Home/>
                    <Route path=path!("/login") view=Login/>
					<Route path=path!("/register") view=Register />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn NotFound() -> impl IntoView {
	view! {
		Nothing to see here, slut
	}
}

#[component]
pub fn Home() -> impl IntoView {
	view! {
		Yoooooooooo
	}
}

#[component]
pub fn Login() -> impl IntoView {
	let (mail, set_mail) = signal("test".to_string());
	let (pass, set_pass) = signal("".to_string());

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
				spawn_local(async {
					let res = log_in(mail, pass).await;
					if res.is_err() {
						log::debug!("{}", res.unwrap_err());
					}
				});
			}
		>
			Log in
		</button>
	}
}

#[component]
pub fn Register() -> impl IntoView {
	let (mail, set_mail) = signal("".to_string());
	let (pass, set_pass) = signal("".to_string());
	let (pass_c, set_pass_c) = signal("".to_string());

	view! {
		Register
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
				let (mail, pass, pass_c) = (
					mail(),
					pass(),
					pass_c()
				);
				if pass != pass_c {
					//TODO: show an error
					return;
				}
				spawn_local(async {
					let res = register(mail, pass).await;
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