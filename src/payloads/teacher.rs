use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct GetInstructorGamesParams {
    pub instructor_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct GetInstructorGameMetadataParams {
    pub instructor_id: i64,
    pub game_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct ListStudentsParams {
    pub instructor_id: i64,
    pub game_id: i64,
    pub group_id: Option<i64>,
    #[serde(default)]
    pub only_active: bool,
}

#[derive(Deserialize, Debug)]
pub struct GetStudentProgressParams {
    pub instructor_id: i64,
    pub game_id: i64,
    pub player_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct GetStudentExercisesParams {
    pub instructor_id: i64,
    pub game_id: i64,
    pub player_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct GetStudentSubmissionsParams {
    pub instructor_id: i64,
    pub game_id: i64,
    pub player_id: i64,
    #[serde(default)]
    pub success_only: bool,
}

#[derive(Deserialize, Debug)]
pub struct GetSubmissionDataParams {
    pub instructor_id: i64,
    pub submission_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct GetExerciseStatsParams {
    pub instructor_id: i64,
    pub game_id: i64,
    pub exercise_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct GetExerciseSubmissionsParams {
    pub instructor_id: i64,
    pub game_id: i64,
    pub exercise_id: i64,
    #[serde(default)]
    pub success_only: bool,
}

#[derive(Deserialize, Debug)]
pub struct CreateGamePayload {
    pub instructor_id: i64,
    pub title: String,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub description: String,
    pub course_id: i64,
    pub programming_language: String,
    #[serde(default)]
    pub module_lock: f64,
    #[serde(default)]
    pub exercise_lock: bool,
    // start_date and end_date are not in payload, will be defaulted
}

#[derive(Deserialize, Debug)]
pub struct ModifyGamePayload {
    pub instructor_id: i64,
    pub game_id: i64,

    pub title: Option<String>,
    pub public: Option<bool>,
    pub active: Option<bool>,
    pub description: Option<String>,
    pub module_lock: Option<f64>,
    pub exercise_lock: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct AddGameInstructorPayload {
    pub requesting_instructor_id: i64,
    pub game_id: i64,
    pub instructor_to_add_id: i64,
    #[serde(default)]
    pub is_owner: bool,
}

#[derive(Deserialize, Debug)]
pub struct RemoveGameInstructorPayload {
    pub requesting_instructor_id: i64,
    pub game_id: i64,
    pub instructor_to_remove_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct ActivateGamePayload {
    pub instructor_id: i64,
    pub game_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct StopGamePayload {
    pub instructor_id: i64,
    pub game_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct RemoveGameStudentPayload {
    pub instructor_id: i64,
    pub game_id: i64,
    pub student_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct TranslateEmailParams {
    pub email: String,
}

#[derive(Deserialize, Debug)]
pub struct CreateGroupPayload {
    pub instructor_id: i64,
    pub display_name: String,
    pub display_avatar: Option<String>,
    #[serde(default)]
    pub member_list: Vec<i64>,
}

#[derive(Deserialize, Debug)]
pub struct DissolveGroupPayload {
    pub instructor_id: i64,
    pub group_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct AddGroupMemberPayload {
    pub instructor_id: i64,
    pub group_id: i64,
    pub player_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct RemoveGroupMemberPayload {
    pub instructor_id: i64,
    pub group_id: i64,
    pub player_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct CreatePlayerPayload {
    pub instructor_id: i64,
    pub email: String,
    pub display_name: String,
    pub display_avatar: Option<String>,
    pub game_id: Option<i64>,
    pub group_id: Option<i64>,
    // Optional language if adding to game (needed for NewPlayerRegistration)
    // Defaulting to "en" if game_id is Some and language is None
    pub language: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct DisablePlayerPayload {
    pub instructor_id: i64,
    pub player_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct DeletePlayerPayload {
    pub instructor_id: i64,
    pub player_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct GenerateInviteLinkPayload {
    pub instructor_id: i64,
    pub game_id: Option<i64>,
    pub group_id: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct ProcessInviteLinkPayload {
    pub player_id: i64,
    pub uuid: Uuid,
}
