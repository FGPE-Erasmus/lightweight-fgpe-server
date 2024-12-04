use chrono::NaiveDate;
use diesel::prelude::*;
use crate::schema::*;

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Course {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub languages: String,
    pub programming_languages: String,
    pub gamification_rule_conditions: String,
    pub gamification_complex_rules: String,
    pub gamification_rule_results: String,
    pub created_at: NaiveDate,
    pub updated_at: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Game {
    pub id: i32,
    pub title: String,
    pub public: bool,
    pub active: bool,
    pub description: String,
    pub course: i32,
    pub programming_language: String,
    pub module_lock: f32,
    pub exercise_lock: bool,
    pub total_exercises: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub created_at: NaiveDate,
    pub updated_at: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Module {
    pub id: i32,
    pub course: i32,
    pub order: i32,
    pub title: String,
    pub description: String,
    pub language: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Exercise {
    pub id: i32,
    pub version: i32,
    pub module: i32,
    pub order: i32,
    pub title: String,
    pub description: String,
    pub language: String,
    pub programming_language: String,
    pub init_code: String,
    pub pre_code: String,
    pub post_code: String,
    pub test_code: String,
    pub check_source: String,
    pub hidden: bool,
    pub locked: bool,
    pub mode: String,
    pub mode_parameters: String,
    pub difficulty: String,
    pub created_at: NaiveDate,
    pub updated_at: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Insertable)]
#[diesel(table_name = submissions)]
pub struct NewSubmission {
    pub exercise: i32,
    pub player: i32,
    pub client: String,
    pub submitted_code: String,
    pub metrics: String,
    pub result: f64,
    pub result_description: String,
    pub feedback: String,
    pub earned_rewards: String,
    pub entered_at: NaiveDate,
    pub submitted_at: NaiveDate,
}

impl NewSubmission {
    pub fn new(exercise: i32, player: i32, client: String, submitted_code: String, metrics: String,
               result: f64, result_description: String, feedback: String, earned_rewards: String,
               entered_at: NaiveDate, submitted_at: NaiveDate) -> Self {
        Self { exercise, player, client, submitted_code, metrics, result, result_description,
            feedback, earned_rewards, entered_at, submitted_at }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Submission {
    pub id: i32,
    pub exercise: i32,
    pub player: i32,
    pub client: String,
    pub submitted_code: String,
    pub metrics: String,
    pub result: f64,
    pub result_description: String,
    pub feedback: String,
    pub earned_rewards: String,
    pub entered_at: NaiveDate,
    pub submitted_at: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Player {
    pub id: i32,
    pub email: String,
    pub display_name: String,
    pub display_avatar: String,
    pub points: i32,
    pub created_at: NaiveDate,
    pub last_active: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Group {
    pub id: i32,
    pub display_name: String,
    pub display_avatar: String,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct PlayerGroup {
    pub player: i32,
    pub group: i32,
    pub joined_at: NaiveDate,
    pub left_at: Option<NaiveDate>,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable, Insertable)]
pub struct PlayerUnlock {
    pub player: i32,
    pub exercise: i32,
    pub unlocked_at: NaiveDate,
}

impl PlayerUnlock {
    pub fn new(player: i32, exercise: i32, unlocked_at: NaiveDate) -> Self {
        Self { player, exercise, unlocked_at }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Reward {
    pub id: i32,
    pub course: i32,
    pub name: String,
    pub description: String,
    pub message_when_won: String,
    pub image_url: String,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct PlayerReward {
    pub player: i32,
    pub reward: i32,
    pub game: Option<i32>,
    pub count: i32,
    pub used_count: i32,
    pub obtained_at: NaiveDate,
    pub expires_at: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct PlayerRegistration {
    pub id: i32,
    pub player: i32,
    pub game: i32,
    pub language: String,
    pub progress: i32,
    pub game_state: String,
    pub saved_at: NaiveDate,
    pub joined_at: NaiveDate,
    pub left_at: Option<NaiveDate>,
}

#[derive(serde::Serialize, serde::Deserialize, Insertable)]
#[diesel(table_name = player_registrations)]
pub struct NewPlayerRegistration {
    pub player: i32,
    pub game: i32,
    pub language: String,
    pub progress: i32,
    pub game_state: String,
    pub saved_at: NaiveDate,
    pub joined_at: NaiveDate,
    pub left_at: Option<NaiveDate>,
}

impl NewPlayerRegistration {
    pub fn new(player: i32, game: i32, language: String, progress: i32, game_state: String,
               saved_at: NaiveDate, joined_at: NaiveDate, left_at: Option<NaiveDate>) -> Self {
        Self { player, game, language, progress, game_state, saved_at, joined_at, left_at }
    }
}

allow_tables_to_appear_in_same_query!(player_registrations, games);
allow_tables_to_appear_in_same_query!(modules, exercises);
