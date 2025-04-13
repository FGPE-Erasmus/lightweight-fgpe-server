use crate::cli::Args;
use axum::routing::{get, post};
use axum::Router;
use clap::Parser;
use deadpool_diesel::postgres::Pool;
use std::net::SocketAddr;
use std::str::FromStr;
use axum::extract::State;
use axum::http::{header, Method};
use axum::response::Redirect;
use axum_keycloak_auth::instance::{KeycloakAuthInstance, KeycloakConfig};
use axum_keycloak_auth::layer::KeycloakAuthLayer;
use axum_keycloak_auth::{PassthroughMode, Url};
use tower_http::cors::CorsLayer;

mod api;
mod cli;
mod model;
mod schema;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    init_tracing();

    let pool = init_pool(&args.connection_str);
    let router = init_router(pool);

    run(router, &args.server_url).await
}

fn init_tracing() {
    tracing_subscriber::fmt::init();
}

fn init_pool(conn_str: &str) -> Pool {
    let manager =
        deadpool_diesel::postgres::Manager::new(conn_str, deadpool_diesel::Runtime::Tokio1);
    Pool::builder(manager)
        .build()
        .expect("should be able to build db conn pool in normal circumstances")
}

fn init_router(pool: Pool) -> Router {
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3006".parse::<header::HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .route("/get_available_games", get(api::get_available_games))
//        .layer(init_protection_layer())
        .route("/join_game", post(api::join_game))
        .route("/save_game", post(api::save_game))
        .route("/load_game", post(api::load_game))
        .route("/leave_game", post(api::leave_game))
        .route("/set_game_lang", post(api::set_game_lang))
        .route("/get_player_games", post(api::get_player_games))
        .route("/get_game_metadata", post(api::get_game_metadata))
        .route("/get_course_data", post(api::get_course_data))
        .route("/get_module_data", post(api::get_module_data))
        .route("/get_exercise_data", post(api::get_exercise_data))
        .route("/submit_solution", post(api::submit_solution))
        .route("/unlock", post(api::unlock))
        .route("/get_last_solution", post(api::get_last_solution))
        .with_state(pool)
        .layer(cors)
}

fn init_protection_layer() -> KeycloakAuthLayer<String> {
    KeycloakAuthLayer::<String>::builder()
        .instance(KeycloakAuthInstance::new(
            KeycloakConfig::builder()
                .server(Url::parse("http://localhost:8080/").unwrap())
                .realm(String::from("rustapp"))
                .build(),
        ))
        .passthrough_mode(PassthroughMode::Block)
        .persist_raw_claims(false)
        .expected_audiences(vec![String::from("account")])
        .build()
}

async fn run(router: Router, server_url: &str) {
    let addr = SocketAddr::from_str(server_url).unwrap();
    tracing::debug!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
