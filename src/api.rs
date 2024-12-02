use crate::api::model::{GetAvailableGamesResponse, JoinGamePayload, JoinGameResponse, LeaveGamePayload, LeaveGameResponse, LoadGamePayload, LoadGameResponse, SaveGamePayload, SaveGameResponse};
use crate::model::{Game, NewPlayerRegistration, PlayerRegistration};
use crate::schema::{games, player_registrations};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use crate::schema::player_registrations::{game, game_state, id, left_at, player, saved_at};

mod model;
mod utils;

pub async fn get_available_games(
    State(pool): State<deadpool_diesel::postgres::Pool>,
) -> Result<Json<GetAvailableGamesResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(|conn| games::table.select(Game::as_select()).load(conn))
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(GetAvailableGamesResponse::new(res)))
}

pub async fn join_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<JoinGamePayload>,
) -> Result<Json<JoinGameResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let date = Utc::now().date_naive();
    let res = conn
        .interact(move |conn| {
            diesel::insert_into(player_registrations::table)
                .values(NewPlayerRegistration::new(
                    payload.player_id,
                    payload.game_id,
                    payload.language.unwrap_or("english".to_string()),
                    0,
                    "".to_string(),
                    date,
                    date,
                    None
                ))
                .returning(PlayerRegistration::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(JoinGameResponse::new(res.id)))
}

pub async fn save_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<SaveGamePayload>,
) -> Result<Json<SaveGameResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            diesel::update(player_registrations::table.find(payload.player_registration_id))
                .set((
                    game_state.eq(payload.game_state),
                    saved_at.eq(Utc::now().date_naive())
                ))
                .execute(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(SaveGameResponse::new(res == 1)))
}

pub async fn load_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<LoadGamePayload>,
) -> Result<Json<LoadGameResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            player_registrations::table
                .filter(id.eq(payload.player_registration_id))
                .select(PlayerRegistration::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(LoadGameResponse::new(res.game_state)))
}

pub async fn leave_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<LeaveGamePayload>,
) -> Result<Json<LeaveGameResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            player_registrations::table
                .filter(player.eq(payload.player_id))
                .filter(game.eq(payload.game_id))
                .select(PlayerRegistration::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            diesel::update(player_registrations::table.find(res.id))
                .set(left_at.eq(Utc::now().date_naive()))
                .execute(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(LeaveGameResponse::new(res == 1)))
}