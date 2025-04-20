use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Deserialize, Debug)]
pub struct JoinGamePayload {
    pub player_id: i64,
    pub game_id: i64,
    pub language: String,
}

#[derive(Deserialize, Debug)]
pub struct SaveGamePayload {
    pub player_registrations_id: i64,
    pub game_state: JsonValue,
}

#[derive(Deserialize, Debug)]
pub struct LoadGamePayload {
    pub player_registrations_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct LeaveGamePayload {
    pub player_id: i64,
    pub game_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct SetGameLangPayload {
    pub player_id: i64,
    pub game_id: i64,
    pub language: String,
}

#[derive(Deserialize, Debug)]
pub struct GetPlayerGamesParams {
    pub player_id: i64,
    pub active: bool,
}

#[derive(Deserialize, Debug)]
pub struct GetCourseDataParams {
    pub game_id: i64,
    pub language: String,
}

#[derive(Deserialize, Debug)]
pub struct GetModuleDataParams {
    pub module_id: i64,
    pub language: String,
    pub programming_language: String,
}

#[derive(Deserialize, Debug)]
pub struct GetExerciseDataParams {
    pub exercise_id: i64,
    pub game_id: i64,
    pub player_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct SubmitSolutionPayload {
    pub player_id: i64,
    pub exercise_id: i64,
    pub game_id: i64,
    pub client: String,
    pub submitted_code: String,
    pub metrics: JsonValue,
    pub result: BigDecimal,
    pub result_description: JsonValue,
    pub feedback: String,
    pub entered_at: DateTime<Utc>,
    pub earned_rewards: JsonValue,
}

#[derive(Deserialize, Debug)]
pub struct UnlockPayload {
    pub player_id: i64,
    pub exercise_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct GetLastSolutionParams {
    pub player_id: i64,
    pub exercise_id: i64,
}
