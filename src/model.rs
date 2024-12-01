use chrono::NaiveDate;
use diesel::prelude::*;
use crate::schema::*;

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