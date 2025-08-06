#![feature(result_option_map_or_default)]
#![feature(future_join)]

use std::{default, env::{self, VarError}, net::SocketAddr, num::ParseIntError};

use axum::{response::Html, routing::get, Router, ServiceExt};
use leptos::{config::{errors::LeptosConfigError, get_configuration}, html::Var};
use leptos_axum::{file_and_error_handler, generate_route_list, LeptosRoutes};
use sqlx::{migrate::MigrateError, postgres::PgPoolOptions, Pool, Postgres};
use leptos::{*, prelude::*};
#[cfg(debug_assertions)]
use tower_http::cors::CorsLayer;

use crate::{downloader::init_downloader, state::AppState, ws::WebSocketState};

mod user;
mod ws;
mod state;
mod cookie;
mod downloader;

thrw_shared::make_error_type!(
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
	let listener_ip = env::var("THRW_IP")?;
	let listener_port = env::var("THRW_PORT")?;

	// Get Leptos configuration TODO: Currently not actually getting them, fix?
	let conf = get_configuration(None)?;
	let leptos_options = conf.leptos_options;

	let db_pool = PgPoolOptions::new()
        .max_connections(db_connect_limit)
        .connect(&db_url)
        .await?
	;

	let app_state = AppState {
		shared: thrw_shared::app::state::server::SharedAppState {
			db_pool: db_pool.clone(),
			leptos_options,
			user_data: Default::default(),
			dl_context: init_downloader(
				&db_pool
			),
		},
		socket: Default::default(),
	};

	app_state.shared.dl_context.request_channel.send(thrw_shared::app::media_request::MediaRequest::Youtube(
		thrw_shared::app::media_request::YoutubeRequest { url: "https://www.youtube.com/watch?v=wepdZFa2nUU".to_string(), audio_only: true },
		None
	));


	// Reset DB if requested
	let reset_needed = args.contains(&"--revert".to_string());
	if reset_needed {
		println!("Resetting DB...");
		sqlx::migrate!("./migrations")
			.undo(&app_state.shared.db_pool, 0)
			.await?
		;
	};

	// Migrate if necessary
	let need_migration = reset_needed || {
		#[cfg(debug_assertions)]
		{ true }
		#[cfg(not(debug_assertions))]
		args.contains(&"--migrate".to_string())
	};

	if need_migration {
		println!("Migrating DB...");
		sqlx::migrate!("./migrations")
			.run(&app_state.shared.db_pool)
			.await?
		;
	}

	tokio::spawn(async move {
		let res = thrw_shared::vfs::util::init_vfs(&db_pool).await;
		println!("vfs init res: {res:?}");
	});

	let cors = {
		#[cfg(debug_assertions)]
		{ CorsLayer::very_permissive() }
		#[cfg(not(debug_assertions))]
		CorsLayer::default()
	};

	let frontend_routes = generate_route_list(thrw_frontend::App);
	let leptos_router = Router::new()
		.leptos_routes_with_context(
			&app_state,
			frontend_routes,
			{
				let app_state = app_state.clone();
				println!("providing context...");
				move || {
					// TODO: try adding FromRef<AppState> instead 
					provide_context(app_state.shared.clone());
					provide_context(app_state.clone());
				}
			},
			{
				let leptos_options = app_state.shared.leptos_options.clone();
				move || thrw_frontend::shell(leptos_options.clone())
			}
		)
		.route("/ws/chat", get(ws::handle_ws))
		//wtf...............................
		.fallback::<
			_,
			(
				_,
				axum::http::Uri,
				axum::extract::State<AppState>,
				axum::http::Request<axum::body::Body>
			)
		>(file_and_error_handler(thrw_frontend::shell))
		.layer(cors)
		.with_state(app_state)
	;


	let listener = tokio::net::TcpListener::bind(format!("{listener_ip}:{listener_port}"))
		.await?
	;

	println!("listening on {listener_ip}:{listener_port}");
	axum::serve(
		listener,
		leptos_router
			.into_make_service_with_connect_info::<SocketAddr>()
		)
		.await?;

	Ok(())
}
