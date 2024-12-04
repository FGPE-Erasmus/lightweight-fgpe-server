use crate::api::model::{GetAvailableGamesResponse, GetCourseDataPayload, GetCourseDataResponse, GetExerciseDataPayload, GetExerciseDataResponse, GetGameMetadataPayload, GetGameMetadataResponse, GetLastSolutionPayload, GetLastSolutionResponse, GetModuleDataPayload, GetModuleDataResponse, GetPlayerGamesPayload, GetPlayerGamesResponse, JoinGamePayload, JoinGameResponse, LeaveGamePayload, LeaveGameResponse, LoadGamePayload, LoadGameResponse, SaveGamePayload, SaveGameResponse, SetGameLangPayload, SetGameLangResponse, SubmitSolutionPayload, SubmitSolutionResponse, UnlockPayload};
use crate::model::{Course, Exercise, Game, Module, NewPlayerRegistration, NewSubmission, PlayerRegistration, PlayerUnlock, Submission};
use crate::schema::player_registrations::{game, game_state, id, language, left_at, player, saved_at};
use crate::schema::{courses, exercises, games, modules, player_registrations, player_unlocks, submissions};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use diesel::{ExpressionMethods, JoinOnDsl, QueryDsl, RunQueryDsl, SelectableHelper};

mod model;
mod utils;

pub async fn get_available_games(
    State(pool): State<deadpool_diesel::postgres::Pool>,
) -> Result<Json<GetAvailableGamesResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(|conn| games::table.select(Game::as_select()).load(conn))
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(GetAvailableGamesResponse::new(res)))
}

pub async fn join_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<JoinGamePayload>,
) -> Result<Json<JoinGameResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let date = Utc::now().date_naive();
    let res = conn
        .interact(move |conn| {
            diesel::insert_into(player_registrations::table)
                .values(NewPlayerRegistration::new(
                    payload.player_id,
                    payload.game_id,
                    payload.language.unwrap_or("english".to_string()),
                    0,
                    "".to_string(),
                    date,
                    date,
                    None
                ))
                .returning(PlayerRegistration::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(JoinGameResponse::new(res.id)))
}

pub async fn save_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<SaveGamePayload>,
) -> Result<Json<SaveGameResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            diesel::update(player_registrations::table.find(payload.player_registration_id))
                .set((
                    game_state.eq(payload.game_state),
                    saved_at.eq(Utc::now().date_naive())
                ))
                .execute(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(SaveGameResponse::new(res == 1)))
}

pub async fn load_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<LoadGamePayload>,
) -> Result<Json<LoadGameResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            player_registrations::table
                .filter(id.eq(payload.player_registration_id))
                .select(PlayerRegistration::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(LoadGameResponse::new(res.game_state)))
}

pub async fn leave_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<LeaveGamePayload>,
) -> Result<Json<LeaveGameResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            player_registrations::table
                .filter(player.eq(payload.player_id))
                .filter(game.eq(payload.game_id))
                .select(PlayerRegistration::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            diesel::update(player_registrations::table.find(res.id))
                .set(left_at.eq(Utc::now().date_naive()))
                .execute(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(LeaveGameResponse::new(res == 1)))
}

pub async fn set_game_lang(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<SetGameLangPayload>,
) -> Result<Json<SetGameLangResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            games::table
                .filter(games::id.eq(payload.game_id))
                .select(Game::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            courses::table
                .filter(courses::id.eq(res.course))
                .select(Course::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    if res.languages.split(',').collect::<Vec<_>>().contains(&&*payload.language) {
        let res = conn
            .interact(move |conn| {
                player_registrations::table
                    .filter(player.eq(payload.player_id))
                    .filter(game.eq(payload.game_id))
                    .select(PlayerRegistration::as_select())
                    .first(conn)
            })
            .await
            .map_err(utils::internal_error)?
            .map_err(utils::internal_error)?;
        let res = conn
            .interact(move |conn| {
                diesel::update(player_registrations::table.find(res.id))
                    .set(language.eq(payload.language))
                    .execute(conn)
            })
            .await
            .map_err(utils::internal_error)?
            .map_err(utils::internal_error)?;
        Ok(Json(SetGameLangResponse::new(res == 1)))
    } else {
        Ok(Json(SetGameLangResponse::new(false)))
    }
}

pub async fn get_player_games(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetPlayerGamesPayload>,
) -> Result<Json<GetPlayerGamesResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            if payload.active {
                player_registrations::table
                    .inner_join(games::table.on(game.eq(games::id)))
                    .filter(games::active.eq(true))
                    .select(id)
                    .load(conn)
            } else {
                player_registrations::table
                    .select(id)
                    .load(conn)
            }
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(GetPlayerGamesResponse::new(res)))
}

pub async fn get_game_metadata(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetGameMetadataPayload>,
) -> Result<Json<GetGameMetadataResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            player_registrations::table
                .inner_join(games::table.on(game.eq(games::id)))
                .filter(id.eq(payload.player_registrations_id))
                .select((PlayerRegistration::as_select(), Game::as_select()))
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(GetGameMetadataResponse::new(res)))
}

pub async fn get_course_data(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetCourseDataPayload>,
) -> Result<Json<GetCourseDataResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            games::table
                .filter(games::id.eq(payload.game_id))
                .select(Game::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    let course = conn
        .interact(move |conn| {
            courses::table
                .filter(courses::id.eq(res.course))
                .select(Course::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            modules::table
                .filter(modules::course.eq(course.id))
                .filter(modules::language.eq(payload.language))
                .select(modules::id)
                .load(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(GetCourseDataResponse::new(
        course.gamification_rule_conditions,
        course.gamification_complex_rules,
        course.gamification_rule_results,
        res
    )))
}

pub async fn get_module_data(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetModuleDataPayload>,
) -> Result<Json<GetModuleDataResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let module = conn
        .interact(move |conn| {
            modules::table
                .filter(modules::id.eq(payload.module_id))
                .select(Module::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            exercises::table
                .filter(exercises::module.eq(payload.module_id))
                .filter(exercises::programming_language.eq(payload.programming_language))
                .filter(exercises::language.eq(payload.language))
                .select(exercises::id)
                .load(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(GetModuleDataResponse::new(
        module.order, module.title, module.description, module.start_date, module.end_date, res
    )))
}

pub async fn get_exercise_data(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetExerciseDataPayload>,
) -> Result<Json<GetExerciseDataResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let mut exercise = conn
        .interact(move |conn| {
            exercises::table
                .filter(exercises::id.eq(payload.exercise_id))
                .select(Exercise::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    let player_unlock = conn
        .interact(move |conn| {
            player_unlocks::table
                .filter(player_unlocks::player.eq(payload.player_id))
                .filter(player_unlocks::exercise.eq(payload.exercise_id))
                .select(PlayerUnlock::as_select())
                .load(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    let res_game = conn
        .interact(move |conn| {
            games::table
                .filter(games::id.eq(payload.game_id))
                .select(Game::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    exercise.hidden = exercise.hidden && player_unlock.is_empty();
    exercise.locked = (exercise.locked || res_game.exercise_lock) && player_unlock.is_empty(); //todo helper functions from mail
    Ok(Json(GetExerciseDataResponse::new(exercise)))
}

pub async fn submit_solution(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<SubmitSolutionPayload>,
) -> Result<Json<SubmitSolutionResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            submissions::table
                .filter(submissions::player.eq(payload.player_id))
                .filter(submissions::exercise.eq(payload.exercise_id))
                .select(Submission::as_select())
                .load(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    let _ = conn
        .interact(move |conn| {
            diesel::insert_into(submissions::table)
                .values(NewSubmission::new(
                    payload.exercise_id,
                    payload.player_id,
                    payload.submission_client,
                    payload.submission_submitted_code,
                    payload.submission_metrics,
                    payload.submission_result,
                    payload.submission_result_description,
                    payload.submission_feedback,
                    payload.submission_earned_rewards,
                    payload.submission_entered_at,
                    Utc::now().date_naive()
                ))
                .returning(Submission::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    if res.is_empty() {
        let module = conn
            .interact(move |conn| {
                exercises::table
                    .inner_join(modules::table.on(exercises::module.eq(modules::id)))
                    .filter(exercises::id.eq(payload.exercise_id))
                    .select(Module::as_select())
                    .first(conn)
            })
            .await
            .map_err(utils::internal_error)?
            .map_err(utils::internal_error)?;

        let game_res = conn
            .interact(move |conn| {
                games::table
                    .filter(games::course.eq(module.course))
                    .select(Game::as_select())
                    .first(conn)
            })
            .await
            .map_err(utils::internal_error)?
            .map_err(utils::internal_error)?;

        let _ = conn
            .interact(move |conn| {
                diesel::update(player_registrations::table)
                    .filter(player_registrations::player.eq(payload.player_id))
                    .filter(player_registrations::game.eq(game_res.id))
                    .set((
                        player_registrations::progress.eq(player_registrations::progress + 1),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(utils::internal_error)?
            .map_err(utils::internal_error)?;

        //todo update playerrewards
    }
    Ok(Json(SubmitSolutionResponse::new(res.is_empty())))
}

pub async fn unlock(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<UnlockPayload>,
) -> Result<Json<()>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let _ = conn
        .interact(move |conn| {
            diesel::insert_into(player_unlocks::table)
                .values(PlayerUnlock::new(payload.player_id, payload.exercise_id, Utc::now().date_naive()))
                .returning(PlayerUnlock::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(())) //todo revise
}

pub async fn get_last_solution(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(_): Json<GetLastSolutionPayload>,
) -> Result<Json<GetLastSolutionResponse>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(utils::internal_error)?;
    let res = conn
        .interact(move |conn| {
            submissions::table
                .order(submissions::id.desc())
                .select(Submission::as_select())
                .first(conn)
        })
        .await
        .map_err(utils::internal_error)?
        .map_err(utils::internal_error)?;
    Ok(Json(GetLastSolutionResponse::new(
        res.submitted_code,
        res.metrics,
        res.result,
        res.result_description,
        res.feedback
    )))//todo revise
}