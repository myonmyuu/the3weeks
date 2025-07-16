use std::{default, env::{self, VarError}, num::ParseIntError};

use axum::{response::Html, Router};
use leptos::{config::{errors::LeptosConfigError, get_configuration}, html::Var};
use leptos_axum::{file_and_error_handler, generate_route_list, LeptosRoutes};
use sqlx::{migrate::MigrateError, postgres::PgPoolOptions, Pool, Postgres};
use ssins_frontend::{App, Login};
use leptos::{*, prelude::*};
use ssins_shared::app::state::UserData;

mod user;

ssins_shared::make_error_type!(
	StartupError {
		Sqlx(sqlx::Error),
		ParseInt(ParseIntError),
		Var(VarError),
		Migrate(MigrateError),
		LeptosConfig(LeptosConfigError),
		IO(std::io::Error),
	}
);

#[tokio::main]
async fn main() -> Result<(), StartupError> {
	println!("Starting 7sins...");

	// Ensure default environment, collect arguments
	dotenvy::dotenv().ok();
	let args: Vec<String> = std::env::args().collect();
	println!("args: {}", args.join(", "));

	// Initialize environement values
	let db_url = env::var("DATABASE_URL")?;
	let db_connect_limit = env::var("DATABASE_MAX_CONNECTIONS")?.parse::<u32>()?;
	let listener_ip = env::var("SSIN_IP")?;
	let listener_port = env::var("SSIN_PORT")?;

	// Get Leptos configuration TODO: Currently not actually getting them, fix?
	let conf = get_configuration(None)?;
	let leptos_options = conf.leptos_options;

	let db_pool = PgPoolOptions::new()
        .max_connections(db_connect_limit)
        .connect(&db_url)
        .await?
	;

	let app_state = ssins_shared::app::state::AppState {
		db_pool,
		leptos_options,
		user_data: Default::default(),
	};

	// Migrate if necessary
	let need_migration = {
		#[cfg(debug_assertions)]
		{
			true
		}
		#[cfg(not(debug_assertions))]
		{
			args.contains(&"--migrate".to_string())
		}
	};
	if need_migration {
		println!("Migrating DB...");
		sqlx::migrate!("./migrations")
			.run(&app_state.db_pool)
			.await?;
	}

	let app_routes = generate_route_list(App);

	let app = Router::new()
		.leptos_routes_with_context(
			&app_state,
			app_routes, 
			{
				let app_state = app_state.clone();
				println!("providing context...");
				move || provide_context(app_state.clone())
			},
			{
				let leptos_options = app_state.leptos_options.clone();
				move || ssins_frontend::shell(leptos_options.clone())
			}
		)
		//wtf...............................
		.fallback::<_, (_, axum::http::Uri, axum::extract::State<ssins_shared::app::state::AppState>, axum::http::Request<axum::body::Body>)>(file_and_error_handler(ssins_frontend::shell))
		.with_state(app_state)
	;


	let listener = tokio::net::TcpListener::bind(format!("{listener_ip}:{listener_port}"))
		.await?
	;

	println!("listening on {listener_ip}:{listener_port}");
	axum::serve(listener, app.into_make_service())
		.await?;

	Ok(())
}

async fn handler_root() -> Html<&'static str> {
	Html("<h1>Hello, World!</h1>")
}