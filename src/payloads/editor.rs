use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::vec::Vec;

#[derive(Deserialize, Debug)]
pub struct ImportExerciseData {
    pub version: BigDecimal,
    pub order: i32,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub language: String,
    pub programming_language: String,
    #[serde(default)]
    pub init_code: String,
    #[serde(default)]
    pub pre_code: String,
    #[serde(default)]
    pub post_code: String,
    #[serde(default)]
    pub test_code: String,
    #[serde(default)]
    pub check_source: String,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub locked: bool,
    pub mode: String,
    #[serde(default = "default_json_object")]
    pub mode_parameters: JsonValue,
    pub difficulty: String,
}

#[derive(Deserialize, Debug)]
pub struct ImportModuleData {
    pub order: i32,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub language: String,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub exercises: Vec<ImportExerciseData>,
}

#[derive(Deserialize, Debug)]
pub struct ImportCourseData {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub languages: String,
    #[serde(default)]
    pub programming_languages: String,
    #[serde(default)]
    pub gamification_rule_conditions: String,
    #[serde(default)]
    pub gamification_complex_rules: String,
    #[serde(default)]
    pub gamification_rule_results: String,
    #[serde(default)]
    pub modules: Vec<ImportModuleData>,
}

#[derive(Deserialize, Debug)]
pub struct ImportCoursePayload {
    pub instructor_id: i64,
    #[serde(default)]
    pub public: bool,
    pub course_data: ImportCourseData,
}

fn default_json_object() -> JsonValue {
    serde_json::json!({})
}

#[derive(Deserialize, Debug)]
pub struct ExportCourseParams {
    pub instructor_id: i64,
    pub course_id: i64,
}
