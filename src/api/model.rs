use crate::model::{Game, PlayerRegistration};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetAvailableGamesResponse {
    games: Vec<Game>
}

impl GetAvailableGamesResponse {
    pub fn new(games: Vec<Game>) -> Self {
        Self { games }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct JoinGamePayload {
    pub(super) player_id: i32,
    pub(super) game_id: i32,
    pub(super) language: Option<String>
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct JoinGameResponse {
    player_registration_id: i32
}

impl JoinGameResponse {
    pub fn new(player_registration_id: i32) -> Self {
        Self { player_registration_id }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SaveGamePayload {
    pub(super) player_registration_id: i32,
    pub(super) game_state: String
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SaveGameResponse {
    saved: bool
}

impl SaveGameResponse {
    pub fn new(saved: bool) -> Self {
        Self { saved }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LoadGamePayload {
    pub(super) player_registration_id: i32
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LoadGameResponse {
    game_state: String
}

impl LoadGameResponse {
    pub fn new(game_state: String) -> Self {
        Self { game_state }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LeaveGamePayload {
    pub(super) player_id: i32,
    pub(super) game_id: i32
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LeaveGameResponse {
    left: bool
}

impl LeaveGameResponse {
    pub fn new(left: bool) -> Self {
        Self { left }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SetGameLangPayload {
    pub(super) player_id: i32,
    pub(super) game_id: i32,
    pub(super) language: String
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SetGameLangResponse {
    set: bool
}

impl SetGameLangResponse {
    pub fn new(set: bool) -> Self {
        Self { set }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetPlayerGamesPayload {
    pub(super) player_id: i32,
    pub(super) active: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetPlayerGamesResponse {
    games: Vec<i32>
}

impl GetPlayerGamesResponse {
    pub fn new(games: Vec<i32>) -> Self {
        Self { games }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetGameMetadataPayload {
    pub(super) player_registrations_id: i32
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetGameMetadataResponse {
    data: (PlayerRegistration, Game)
}

impl GetGameMetadataResponse {
    pub fn new(data: (PlayerRegistration, Game)) -> Self {
        Self { data }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetCourseDataPayload {
    pub(super) game_id: i32,
    pub(super) language: String
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetCourseDataResponse {
    course_gamification_rule_conditions: String,
    gamification_complex_rules: String,
    gamification_rule_results: String,
    modules: Vec<i32>
}

impl GetCourseDataResponse {
    pub fn new(course_gamification_rule_conditions: String, gamification_complex_rules: String,
               gamification_rule_results: String, modules: Vec<i32>) -> Self {
        Self { course_gamification_rule_conditions, gamification_complex_rules, gamification_rule_results, modules }
    }
}