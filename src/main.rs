use crate::cli::Args;
use anyhow::Context;
use axum::Router;
use axum::routing::{get, post};
use axum_keycloak_auth::PassthroughMode;
use axum_keycloak_auth::instance::{KeycloakAuthInstance, KeycloakConfig};
use axum_keycloak_auth::layer::KeycloakAuthLayer;
use clap::Parser;
use deadpool_diesel::Runtime;
use deadpool_diesel::postgres::{Manager, Pool};
use std::net::SocketAddr;
use tracing::log::info;
use tracing_subscriber::{EnvFilter, fmt};

mod api;
mod cli;
mod errors;
mod model;
mod payloads;
mod response;
mod schema;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    init_tracing(&args.log_level)?;

    info!("Initializing database pool...");
    let pool = init_pool(&args.connection_str, args.db_pool_max_size)
        .context("Failed to initialize database pool")?;

    info!("Initializing Keycloak authentication layer...");
    let keycloak_layer =
        init_protection_layer(&args).context("Failed to initialize Keycloak layer")?;

    info!("Initializing router...");
    let router = init_router(pool, keycloak_layer);

    info!("Starting server...");
    run(router, args.server_address)
        .await
        .context("Server failed to run")?;

    Ok(())
}

fn init_tracing(log_level: &str) -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::try_new(log_level)?).init();
    Ok(())
}

fn init_pool(conn_str: &str, max_size: u32) -> anyhow::Result<Pool> {
    let manager = Manager::new(conn_str, Runtime::Tokio1);
    let pool = Pool::builder(manager).max_size(max_size as usize).build()?;
    Ok(pool)
}

fn init_protection_layer(args: &Args) -> anyhow::Result<KeycloakAuthLayer<String>> {
    let config = KeycloakConfig::builder()
        .server(args.keycloak_server_url.clone())
        .realm(args.keycloak_realm.clone())
        .build();

    let instance = KeycloakAuthInstance::new(config);

    let layer = KeycloakAuthLayer::builder()
        .instance(instance)
        .passthrough_mode(PassthroughMode::Block)
        .persist_raw_claims(false)
        .expected_audiences(vec![args.keycloak_audiences.clone()])
        .build();

    Ok(layer)
}

fn student_routes(keycloak_layer: KeycloakAuthLayer<String>) -> Router<Pool> {
    Router::new()
        // protected routes go here
        .route("/get_available_games", get(api::get_available_games))
        .route("/join_game", post(api::join_game))
        .route("/save_game", post(api::save_game))
        .route("/load_game", post(api::load_game))
        .route("/leave_game", post(api::leave_game))
        .route("/set_game_lang", post(api::set_game_lang))
        .route("/get_player_games", get(api::get_player_games))
        .route(
            "/get_game_metadata/:registration_id",
            get(api::get_game_metadata),
        )
        .route("/get_course_data", get(api::get_course_data))
        .route("/get_module_data", get(api::get_module_data))
        .route("/get_exercise_data", get(api::get_exercise_data))
        .route("/submit_solution", post(api::submit_solution))
        .route("/unlock", post(api::unlock))
        .route("/get_last_solution", get(api::get_last_solution))
        .layer(keycloak_layer)
        // public routes go here
}

fn teacher_routes(keycloak_layer: KeycloakAuthLayer<String>) -> Router<Pool> {
    Router::new()
        // protected routes go here
        .layer(keycloak_layer)
        // public routes go here
}

fn init_router(pool: Pool, keycloak_layer: KeycloakAuthLayer<String>) -> Router {
    let student_api = student_routes(keycloak_layer.clone());
    let teacher_api = teacher_routes(keycloak_layer);

    Router::new()
        .nest("/student", student_api)
        .nest("/teacher", teacher_api)
        .with_state(pool)
}

async fn run(router: Router, addr: SocketAddr) -> anyhow::Result<()> {
    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to address {}", addr))?;
    axum::serve(listener, router.into_make_service())
        .await
        .context("Axum server error")?;
    Ok(())
}
