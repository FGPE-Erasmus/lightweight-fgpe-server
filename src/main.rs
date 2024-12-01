use std::net::SocketAddr;
use std::str::FromStr;
use axum::Router;
use axum::routing::{get, post};
use deadpool_diesel::postgres::Pool;

mod api;
mod model;
mod schema;

const DB_URL: &str = "postgresql://postgres:admin@localhost:8899/gam";
//const DB_URL: &str = "postgresql://wiktor:DB%2Bpass%402024%21@localhost:5432/fgpepp";
//const DB_URL: &str = "postgresql://localhost:5432/fgpepp";
const SERVER_URL: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() {
    init_tracing();

    let pool = init_pool();
    let router = init_router(pool);

    run(router).await
}

fn init_tracing() {
    tracing_subscriber::fmt::init();
}

fn init_pool() -> Pool {
    let manager = deadpool_diesel::postgres::Manager::new(DB_URL, deadpool_diesel::Runtime::Tokio1);
    Pool::builder(manager)
        .build()
        .expect("should be able to build db conn pool in normal circumstances")
}

fn init_router(pool: Pool) -> Router {
    Router::new()
        .route("/get_available_games", get(api::get_available_games))
        .route("/join_game", post(api::join_game))
        .route("/save_game", post(api::save_game))
        .with_state(pool)
}

async fn run(router: Router) {
    let addr = SocketAddr::from_str(SERVER_URL).unwrap();
    tracing::debug!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}