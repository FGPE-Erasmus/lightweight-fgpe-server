use crate::schema::game_ownership;
use crate::schema::games;
use crate::schema::group_ownership;
use crate::schema::groups;
use crate::schema::instructors;
use crate::schema::invites;
use crate::schema::player_groups;
use crate::schema::players;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = games)]
pub struct NewGame {
    pub title: String,
    pub public: bool,
    pub active: bool,
    pub description: String,
    pub course_id: i64,
    pub programming_language: String,
    pub module_lock: f64,
    pub exercise_lock: bool,
    pub total_exercises: i32,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    // created_at, updated_at have DB defaults
}

#[derive(Insertable, Debug)]
#[diesel(table_name = game_ownership)]
pub struct NewGameOwnership {
    pub game_id: i64,
    pub instructor_id: i64,
    pub owner: bool,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = groups)]
pub struct NewGroup {
    pub display_name: String,
    pub display_avatar: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = group_ownership)]
pub struct NewGroupOwnership {
    pub group_id: i64,
    pub instructor_id: i64,
    pub owner: bool,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = player_groups)]
pub struct NewPlayerGroup {
    pub player_id: i64,
    pub group_id: i64,
    // joined_at has DB default
    // left_at is nullable
}

#[derive(Insertable, Debug)]
#[diesel(table_name = players)]
pub struct NewPlayer {
    pub email: String,
    pub display_name: String,
    pub display_avatar: Option<String>,
    // points defaults to 0 in DB
    // created_at, last_active have DB defaults
    // disabled defaults to false in DB
}

#[derive(Insertable)]
#[diesel(table_name = instructors)]
pub struct NewInstructor {
    pub id: i64,
    pub email: String,
    pub display_name: String,
}

#[derive(AsChangeset, Debug, Default)]
#[diesel(table_name = games)]
pub struct GameChangeset {
    pub title: Option<String>,
    pub public: Option<bool>,
    pub active: Option<bool>,
    pub description: Option<String>,
    pub module_lock: Option<f64>,
    pub exercise_lock: Option<bool>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InstructorGameMetadataResponse {
    pub title: String,
    pub description: String,
    pub active: bool,
    pub public: bool,
    pub total_exercises: i32,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub is_owner: bool,
    pub player_count: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct StudentProgressResponse {
    pub attempts: i64,
    pub solved_exercises: i64,
    pub progress: f64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct StudentExercisesResponse {
    pub attempted_exercises: Vec<i64>,
    pub solved_exercises: Vec<i64>,
}

#[derive(Deserialize, Serialize, Debug, Queryable)]
pub struct SubmissionDataResponse {
    pub id: i64,
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
    pub submitted_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ExerciseStatsResponse {
    pub attempts: i64,
    pub successful_attempts: i64,
    pub difficulty: f64,
    pub solved_percentage: f64,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = invites)]
pub struct NewInvite {
    pub uuid: Uuid,
    pub instructor_id: i64,
    pub game_id: Option<i64>,
    pub group_id: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InviteLinkResponse {
    pub invite_uuid: Uuid,
}

#[derive(Queryable, Debug)]
#[diesel(table_name = invites_dsl::invites)]
pub struct Invite {
    pub id: i64,
    pub uuid: Uuid,
    pub instructor_id: i64,
    pub game_id: Option<i64>,
    pub group_id: Option<i64>,
}
