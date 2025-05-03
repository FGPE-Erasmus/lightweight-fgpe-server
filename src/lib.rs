use crate::cli::Args;
use anyhow::Context;
use axum::Router;
use axum::routing::{get, post};
use axum_keycloak_auth::PassthroughMode;
use axum_keycloak_auth::instance::{KeycloakAuthInstance, KeycloakConfig};
use axum_keycloak_auth::layer::KeycloakAuthLayer;
use deadpool_diesel::Runtime;
use deadpool_diesel::postgres::{Manager, Pool};
use tracing::log::info;

pub mod cli;
pub mod model;
pub mod payloads;
pub mod response;
pub mod schema;

mod api;
mod errors;

pub fn init_router(args: &Args) -> anyhow::Result<Router> {
    info!("Initializing database pool...");
    let pool = init_pool(&args.connection_str, args.db_pool_max_size)
        .context("Failed to initialize database pool")?;

    info!("Initializing Keycloak authentication layer...");
    let keycloak_layer =
        init_protection_layer(args).context("Failed to initialize Keycloak layer")?;

    info!("Initializing router...");
    Ok(init_router_internal(pool, keycloak_layer))
}

pub fn init_test_router(pool: Pool) -> Router {
    let student_api = student_routes();
    let teacher_api = teacher_routes();
    let editor_api = editor_routes();

    Router::new()
        .nest("/student", student_api)
        .nest("/teacher", teacher_api)
        .nest("/editor", editor_api)
        .with_state(pool)
}

fn init_router_internal(pool: Pool, keycloak_layer: KeycloakAuthLayer<String>) -> Router {
    let student_api = student_routes().layer(keycloak_layer.clone());
    let teacher_api = teacher_routes().layer(keycloak_layer.clone());
    let editor_api = editor_routes().layer(keycloak_layer.clone());

    Router::new()
        .nest("/student", student_api)
        .nest("/teacher", teacher_api)
        .nest("/editor", editor_api)
        .with_state(pool)
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

fn student_routes() -> Router<Pool> {
    Router::new()
        // protected routes go here
        .route(
            "/get_available_games",
            get(api::student::get_available_games),
        )
        .route("/join_game", post(api::student::join_game))
        .route("/save_game", post(api::student::save_game))
        .route("/load_game", post(api::student::load_game))
        .route("/leave_game", post(api::student::leave_game))
        .route("/set_game_lang", post(api::student::set_game_lang))
        .route("/get_player_games", get(api::student::get_player_games))
        .route(
            "/get_game_metadata/{registration_id}",
            get(api::student::get_game_metadata),
        )
        .route("/get_course_data", get(api::student::get_course_data))
        .route("/get_module_data", get(api::student::get_module_data))
        .route("/get_exercise_data", get(api::student::get_exercise_data))
        .route("/submit_solution", post(api::student::submit_solution))
        .route("/unlock", post(api::student::unlock))
        .route("/get_last_solution", get(api::student::get_last_solution))
    // public routes go here
}

fn teacher_routes() -> Router<Pool> {
    Router::new()
        // protected routes go here
        .route(
            "/get_instructor_games",
            get(api::teacher::get_instructor_games),
        )
        .route(
            "/get_instructor_game_metadata",
            get(api::teacher::get_instructor_game_metadata),
        )
        .route("/list_students", get(api::teacher::list_students))
        .route(
            "/get_student_progress",
            get(api::teacher::get_student_progress),
        )
        .route(
            "/get_student_exercises",
            get(api::teacher::get_student_exercises),
        )
        .route(
            "/get_student_submissions",
            get(api::teacher::get_student_submissions),
        )
        .route(
            "/get_submission_data",
            get(api::teacher::get_submission_data),
        )
        .route("/get_exercise_stats", get(api::teacher::get_exercise_stats))
        .route(
            "/get_exercise_submissions",
            get(api::teacher::get_exercise_submissions),
        )
        .route("/create_game", post(api::teacher::create_game))
        .route("/modify_game", post(api::teacher::modify_game))
        .route(
            "/add_game_instructor",
            post(api::teacher::add_game_instructor),
        )
        .route(
            "/remove_game_instructor",
            post(api::teacher::remove_game_instructor),
        )
        .route("/activate_game", post(api::teacher::activate_game))
        .route("/stop_game", post(api::teacher::stop_game))
        .route(
            "/remove_game_student",
            post(api::teacher::remove_game_student),
        )
        .route(
            "/translate_email_to_player_id",
            get(api::teacher::translate_email_to_player_id),
        )
        .route("/create_group", post(api::teacher::create_group))
        .route("/dissolve_group", post(api::teacher::dissolve_group))
        .route("/add_group_member", post(api::teacher::add_group_member))
        .route(
            "/remove_group_member",
            post(api::teacher::remove_group_member),
        )
        .route("/create_player", post(api::teacher::create_player))
        .route("/disable_player", post(api::teacher::disable_player))
        .route("/delete_player", post(api::teacher::delete_player))
        .route(
            "/generate_invite_link",
            post(api::teacher::generate_invite_link),
        )
        .route(
            "/process_invite_link",
            post(api::teacher::process_invite_link),
        )
    // public routes go here
}

fn editor_routes() -> Router<Pool> {
    Router::new()
        // protected routes go here
        .route("/import_course", post(api::editor::import_course))
        .route("/export_course", get(api::editor::export_course))
    // public routes go here
}
