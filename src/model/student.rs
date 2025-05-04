use crate::schema::player_registrations;
use crate::schema::player_rewards;
use crate::schema::player_unlocks;
use crate::schema::submissions;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Insertable, Debug)]
#[diesel(table_name = player_registrations)]
pub struct NewPlayerRegistration {
    pub player_id: i64,
    pub game_id: i64,
    pub language: String,
    pub progress: i32,
    pub game_state: JsonValue,
    // joined_at and saved_at have DB defaults (CURRENT_TIMESTAMP)
    // left_at is nullable (defaults to NULL)
}

#[derive(Insertable, Debug)]
#[diesel(table_name = submissions)]
pub struct NewSubmission {
    pub exercise_id: i64,
    pub game_id: i64,
    pub player_id: i64,
    pub client: String,
    pub submitted_code: String,
    pub metrics: JsonValue,
    pub result: BigDecimal,
    pub result_description: JsonValue,
    pub first_solution: bool,
    pub feedback: String,
    pub earned_rewards: JsonValue,
    pub entered_at: DateTime<Utc>,
    // submitted_at has a DB default (CURRENT_TIMESTAMP)
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = player_rewards)]
pub struct NewPlayerReward {
    pub player_id: i64,
    pub reward_id: i64,
    pub game_id: Option<i64>,
    pub count: i32,
    pub used_count: i32,
    pub obtained_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = player_unlocks)]
pub struct NewPlayerUnlock {
    pub player_id: i64,
    pub exercise_id: i64,
    // unlocked_at has a DB default (CURRENT_TIMESTAMP)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GameMetadata {
    pub registration_id: i64,
    pub progress: i32,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub language: String,

    pub game_id: i64,
    pub game_title: String,
    pub game_active: bool,
    pub game_description: String,
    pub game_programming_language: String,
    pub game_total_exercises: i32,
    pub game_start_date: DateTime<Utc>,
    pub game_end_date: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CourseDataResponse {
    pub gamification_rule_conditions: String,
    pub gamification_complex_rules: String,
    pub gamification_rule_results: String,
    pub module_ids: Vec<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ModuleDataResponse {
    pub order: i32,
    pub title: String,
    pub description: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,

    pub exercise_ids: Vec<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ExerciseDataResponse {
    // exercises fields
    pub order: i32,
    pub title: String,
    pub description: String,
    pub init_code: String,
    pub pre_code: String,
    pub post_code: String,
    pub test_code: String,
    pub check_source: String,
    pub mode: String,
    pub mode_parameters: JsonValue,
    pub difficulty: String,
    // calculated fields
    pub hidden: bool,
    pub locked: bool,
}

#[derive(Deserialize, Serialize, Debug, Queryable)]
pub struct LastSolutionResponse {
    pub submitted_code: String,
    pub metrics: JsonValue,
    pub result: BigDecimal,
    pub result_description: JsonValue,
    pub feedback: String,
    pub submitted_at: DateTime<Utc>,
}
