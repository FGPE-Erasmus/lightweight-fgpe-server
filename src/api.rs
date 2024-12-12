use crate::api::model::{GetAvailableGamesResponse, GetCourseDataPayload, GetCourseDataResponse, GetGameMetadataPayload, GetGameMetadataResponse, GetModuleDataPayload, GetModuleDataResponse, GetPlayerGamesPayload, GetPlayerGamesResponse, JoinGamePayload, JoinGameResponse, LeaveGamePayload, LeaveGameResponse, LoadGamePayload, LoadGameResponse, SaveGamePayload, SaveGameResponse, SetGameLangPayload, SetGameLangResponse};
use crate::model::{Course, Game, Module, NewPlayerRegistration, PlayerRegistration};
use crate::schema::games::{active, public};
use crate::schema::playerregistrations::{game, gamestate, id, language, leftat, player, savedat};
use crate::schema::{courses, exercises, games, modules, playerregistrations};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use diesel::{ExpressionMethods, JoinOnDsl, QueryDsl, RunQueryDsl, SelectableHelper};

mod model;

// private helper

type ApiResponse<T> = Result<Json<T>, ApiError>;
type ApiError = (StatusCode, String);

pub(super) fn internal_error<E>(err: E) -> ApiError
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

async fn run_query<T, F>(pool: &deadpool_diesel::postgres::Pool, query: F) -> Result<T, ApiError>
where
    F: FnOnce(&mut diesel::PgConnection) -> Result<T, diesel::result::Error> + Send + 'static,
    T: Send + 'static,
{
    let conn = pool.get()
        .await
        .map_err(internal_error)?;
    conn.interact(query)
        .await
        .map_err(internal_error)
        .and_then(|res| res.map_err(internal_error))
}

// public api

pub async fn get_available_games(
    State(pool): State<deadpool_diesel::postgres::Pool>,
) -> ApiResponse<GetAvailableGamesResponse> {
    let games = run_query(&pool, |conn| {
        games::table
            .filter(active.eq(true))
            .filter(public.eq(true))
            .select(Game::as_select())
            .load(conn)
    }).await?;
    Ok(Json(GetAvailableGamesResponse::new(games)))
}

pub async fn join_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<JoinGamePayload>,
) -> ApiResponse<JoinGameResponse> {
    let result = run_query(&pool, move |conn| {
        playerregistrations::table
            .filter(player.eq(payload.player_id))
            .filter(game.eq(payload.game_id))
            .select(PlayerRegistration::as_select())
            .first(conn)
    }).await;
    if result.is_ok() {
        return Ok(Json(JoinGameResponse::new(None)));
    }
    let date = Utc::now().date_naive();
    let registration = NewPlayerRegistration::new(
        payload.player_id,
        payload.game_id,
        payload.language.unwrap_or_else(|| "english".to_string()),
        0,
        String::new(),
        date,
        date,
        None,
    );
    let result = run_query(&pool, |conn| {
        diesel::insert_into(playerregistrations::table)
            .values(registration)
            .returning(PlayerRegistration::as_returning())
            .get_result(conn)
    }).await;
    match result {
        Ok(new_registration) => Ok(Json(JoinGameResponse::new(Some(new_registration.id)))),
        Err(_) => Ok(Json(JoinGameResponse::new(None)))
    }
}

pub async fn save_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<SaveGamePayload>,
) -> ApiResponse<SaveGameResponse> {
    let result = run_query(&pool, move |conn| {
        diesel::update(playerregistrations::table.find(payload.player_registration_id))
            .set((
                gamestate.eq(payload.game_state),
                savedat.eq(Utc::now().date_naive()),
            ))
            .execute(conn)
    }).await;
    match result {
        Ok(rows_updated) => Ok(Json(SaveGameResponse::new(rows_updated == 1))),
        Err(_) => Ok(Json(SaveGameResponse::new(false)))
    }
}

pub async fn load_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<LoadGamePayload>,
) -> ApiResponse<LoadGameResponse> {
    let result = run_query(&pool, move |conn| {
        playerregistrations::table
            .filter(id.eq(payload.player_registration_id))
            .select(PlayerRegistration::as_select())
            .first(conn)
    }).await;
    match result {
        Ok(pr) => Ok(Json(LoadGameResponse::new(pr.gamestate))),
        Err(_) => Ok(Json(LoadGameResponse::new(String::new())))
    }
}

pub async fn leave_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<LeaveGamePayload>,
) -> ApiResponse<LeaveGameResponse> {
    let registration = run_query(&pool, move |conn| {
        playerregistrations::table
            .filter(player.eq(payload.player_id))
            .filter(game.eq(payload.game_id))
            .select(PlayerRegistration::as_select())
            .first(conn)
    }).await?;
    let rows_updated = run_query(&pool, move |conn| {
        diesel::update(playerregistrations::table.find(registration.id))
            .set(leftat.eq(Utc::now().date_naive()))
            .execute(conn)
    }).await;
    match rows_updated {
        Ok(rows) => Ok(Json(LeaveGameResponse::new(rows == 1))),
        Err(_) => Ok(Json(LeaveGameResponse::new(false)))
    }
}

pub async fn set_game_lang(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<SetGameLangPayload>,
) -> ApiResponse<SetGameLangResponse> {
    let game_result = run_query(&pool, move |conn| {
        games::table
            .filter(games::id.eq(payload.game_id))
            .select(Game::as_select())
            .first(conn)
    }).await?;
    let course_result = run_query(&pool, move |conn| {
        courses::table
            .filter(courses::id.eq(game_result.course))
            .select(Course::as_select())
            .first(conn)
    }).await?;
    if course_result.languages
        .replace(" ", "")
        .split(',')
        .any(|lang| lang == payload.language) {
            let registration_result = run_query(&pool, move |conn| {
                playerregistrations::table
                    .filter(player.eq(payload.player_id))
                    .filter(game.eq(payload.game_id))
                    .select(PlayerRegistration::as_select())
                    .first(conn)
            }).await?;
            let rows_updated = run_query(&pool, move |conn| {
                diesel::update(playerregistrations::table.find(registration_result.id))
                    .set(language.eq(payload.language))
                    .execute(conn)
            }).await?;
            Ok(Json(SetGameLangResponse::new(rows_updated == 1)))
        } else {
            Ok(Json(SetGameLangResponse::new(false)))
    }
}

pub async fn get_player_games(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetPlayerGamesPayload>,
) -> ApiResponse<GetPlayerGamesResponse> {
    let games_result = run_query(&pool, move |conn| {
        if payload.active {
            playerregistrations::table
                .inner_join(games::table.on(game.eq(games::id)))
                .filter(player.eq(payload.player_id))
                .filter(active.eq(true))
                .filter(leftat.is_null())
                .select(id)
                .load(conn)
        } else {
            playerregistrations::table
                .filter(player.eq(payload.player_id))
                .select(id)
                .load(conn)
        }
    }).await?;
    Ok(Json(GetPlayerGamesResponse::new(games_result)))
}

pub async fn get_game_metadata(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetGameMetadataPayload>,
) -> ApiResponse<GetGameMetadataResponse> {
    let metadata_result = run_query(&pool, move |conn| {
        playerregistrations::table
            .inner_join(games::table.on(game.eq(games::id)))
            .filter(id.eq(payload.player_registrations_id))
            .select((PlayerRegistration::as_select(), Game::as_select()))
            .first(conn)
    }).await?;
    Ok(Json(GetGameMetadataResponse::new(metadata_result)))
}

pub async fn get_course_data(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetCourseDataPayload>,
) -> ApiResponse<GetCourseDataResponse> {
    let game_result = run_query(&pool, move |conn| {
        games::table
            .filter(games::id.eq(payload.game_id))
            .select(Game::as_select())
            .first(conn)
    }).await?;
    let course_result = run_query(&pool, move |conn| {
        courses::table
            .filter(courses::id.eq(game_result.course))
            .select(Course::as_select())
            .first(conn)
    }).await?;
    let modules_result = run_query(&pool, move |conn| {
        modules::table
            .filter(modules::course.eq(course_result.id))
            .filter(modules::language.eq(payload.language))
            .select(modules::id)
            .load(conn)
    }).await?;
    Ok(Json(GetCourseDataResponse::new(
        course_result.gamificationruleconditions,
        course_result.gamificationcomplexrules,
        course_result.gamificationruleresults,
        modules_result,
    )))
}

pub async fn get_module_data(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetModuleDataPayload>,
) -> ApiResponse<GetModuleDataResponse> {
    let module = run_query(&pool, move |conn| {
        modules::table.find(payload.module_id)
            .select(Module::as_select())
            .first(conn)
    }).await?;
    let exercises = run_query(&pool, move |conn| {
        exercises::table
            .filter(exercises::module.eq(payload.module_id))
            .filter(exercises::programminglanguage.eq(payload.programming_language))
            .filter(exercises::language.eq(payload.language))
            .select(exercises::id)
            .load(conn)
    }).await?;
    Ok(Json(GetModuleDataResponse::new(
        module.order, module.title, module.description, module.startdate, module.enddate, exercises
    )))
}