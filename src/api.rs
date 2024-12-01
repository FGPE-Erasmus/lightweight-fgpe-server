use crate::model::Game;
use crate::schema::games;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use diesel::{QueryDsl, RunQueryDsl, SelectableHelper};

mod utils;

pub async fn get_available_games(
    State(pool): State<deadpool_diesel::postgres::Pool>,
) -> Result<Json<Vec<Game>>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(|conn| games::table.select(Game::as_select()).load(conn))
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(res))
}