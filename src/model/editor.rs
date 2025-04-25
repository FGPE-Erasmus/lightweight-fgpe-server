use crate::schema::{courses, course_ownership, modules, exercises};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;
use serde_json::Value as JsonValue;

#[derive(Insertable, Debug)]
#[diesel(table_name = courses)]
pub struct NewCourse {
    pub title: String,
    pub description: String,
    pub languages: String,
    pub programming_languages: String,
    pub gamification_rule_conditions: String,
    pub gamification_complex_rules: String,
    pub gamification_rule_results: String,
    pub public: bool,
    // created_at, updated_at have DB defaults
}

#[derive(Insertable, Debug)]
#[diesel(table_name = course_ownership)]
pub struct NewCourseOwnership {
    pub course_id: i64,
    pub instructor_id: i64,
    pub owner: bool,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = modules)]
pub struct NewModule {
    pub course_id: i64,
    pub order: i32,
    pub title: String,
    pub description: String,
    pub language: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = exercises)]
pub struct NewExercise {
    pub version: BigDecimal,
    pub module_id: i64,
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
    pub mode_parameters: JsonValue,
    pub difficulty: String,
    // created_at, updated_at have DB defaults
}

#[derive(Serialize, Debug, Clone)]
pub struct ExportExerciseResponse {
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
    pub mode_parameters: JsonValue,
    pub difficulty: String,
    // Add fields needed for internal processing if required, like id/module_id,
    // but potentially skip serializing them if not part of the final export format.
    // #[serde(skip)] pub id: i64,
    // #[serde(skip)] pub module_id: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct ExportModuleResponse {
    pub order: i32,
    pub title: String,
    pub description: String,
    pub language: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    // #[serde(skip)] pub id: i64, // Keep internal ID if needed

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exercises: Vec<ExportExerciseResponse>,
}

#[derive(Serialize, Debug)]
pub struct ExportCourseResponse {
    pub title: String,
    pub description: String,
    pub languages: String,
    pub programming_languages: String,
    pub gamification_rule_conditions: String,
    pub gamification_complex_rules: String,
    pub gamification_rule_results: String,
    // pub public: bool, // Include if needed in export

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<ExportModuleResponse>,
}

#[derive(Queryable, Debug)]
pub struct CourseQueryResult {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub languages: String,
    pub programming_languages: String,
    pub gamification_rule_conditions: String,
    pub gamification_complex_rules: String,
    pub gamification_rule_results: String,
    // pub public: bool,
}

#[derive(Queryable, Debug, Clone)]
pub struct ModuleQueryResult {
    pub id: i64,
    pub course_id: i64,
    pub order: i32,
    pub title: String,
    pub description: String,
    pub language: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

#[derive(Queryable, Debug, Clone)]
pub struct ExerciseQueryResult {
    pub id: i64,
    pub module_id: i64,
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
    pub mode_parameters: JsonValue,
    pub difficulty: String,
}
