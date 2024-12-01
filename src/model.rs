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