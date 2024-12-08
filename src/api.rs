use crate::api::model::{GetAvailableGamesResponse, GetCourseDataPayload, GetCourseDataResponse, GetExerciseDataPayload, GetExerciseDataResponse, GetGameMetadataPayload, GetGameMetadataResponse, GetLastSolutionPayload, GetLastSolutionResponse, GetModuleDataPayload, GetModuleDataResponse, GetPlayerGamesPayload, GetPlayerGamesResponse, JoinGamePayload, JoinGameResponse, LeaveGamePayload, LeaveGameResponse, LoadGamePayload, LoadGameResponse, SaveGamePayload, SaveGameResponse, SetGameLangPayload, SetGameLangResponse, SubmitSolutionPayload, SubmitSolutionResponse, UnlockPayload};
use crate::model::{Course, Exercise, Game, Module, NewPlayerRegistration, NewSubmission, PlayerRegistration, PlayerUnlock, Submission};
use crate::schema::playerregistrations::{game, gamestate, id, language, leftat, player, savedat};
use crate::schema::{courses, exercises, games, modules, playerregistrations, playerunlocks, submissions};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use diesel::{ExpressionMethods, JoinOnDsl, QueryDsl, RunQueryDsl, SelectableHelper};
use crate::schema::games::{active, public};

mod model;

// private helper

type ApiResponse<T> = Result<Json<T>, ApiError>;
type ApiError = (StatusCode, String);

pub(super) fn internal_error<E>(err: E) -> ApiError
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

async fn run_query<T, F>(pool: &deadpool_diesel::postgres::Pool, query: F) -> Result<T, ApiError>
where
    F: FnOnce(&mut diesel::PgConnection) -> Result<T, diesel::result::Error> + Send + 'static,
    T: Send + 'static,
{
    let conn = pool.get()
        .await
        .map_err(internal_error)?;
    conn.interact(query)
        .await
        .map_err(internal_error)
        .and_then(|res| res.map_err(internal_error))
}

// public api

pub async fn get_available_games(
    State(pool): State<deadpool_diesel::postgres::Pool>,
) -> ApiResponse<GetAvailableGamesResponse> {
    let games = run_query(&pool, |conn| {
        games::table
            .filter(active.eq(true))
            .filter(public.eq(true))
            .select(Game::as_select())
            .load(conn)
    }).await?;
    Ok(Json(GetAvailableGamesResponse::new(games)))
}

pub async fn join_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<JoinGamePayload>,
) -> ApiResponse<JoinGameResponse> {
    let result = run_query(&pool, move |conn| {
        playerregistrations::table
            .filter(player.eq(payload.player_id))
            .filter(game.eq(payload.game_id))
            .select(PlayerRegistration::as_select())
            .first(conn)
    }).await;
    if let Ok(_) = result {
        return Ok(Json(JoinGameResponse::new(None)));
    }
    let date = Utc::now().date_naive();
    let registration = NewPlayerRegistration::new(
        payload.player_id,
        payload.game_id,
        payload.language.unwrap_or_else(|| "english".to_string()),
        0,
        String::new(),
        date,
        date,
        None,
    );
    let result = run_query(&pool, |conn| {
        diesel::insert_into(playerregistrations::table)
            .values(registration)
            .returning(PlayerRegistration::as_returning())
            .get_result(conn)
    }).await;
    match result {
        Ok(new_registration) => Ok(Json(JoinGameResponse::new(Some(new_registration.id)))),
        Err(_) => Ok(Json(JoinGameResponse::new(None)))
    }
}

pub async fn save_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<SaveGamePayload>,
) -> ApiResponse<SaveGameResponse> {
    let result = run_query(&pool, move |conn| {
        diesel::update(playerregistrations::table.find(payload.player_registration_id))
            .set((
                gamestate.eq(payload.game_state),
                savedat.eq(Utc::now().date_naive()),
            ))
            .execute(conn)
    }).await;
    match result {
        Ok(rows_updated) => Ok(Json(SaveGameResponse::new(rows_updated == 1))),
        Err(_) => Ok(Json(SaveGameResponse::new(false)))
    }
}

pub async fn load_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<LoadGamePayload>,
) -> ApiResponse<LoadGameResponse> {
    let result = run_query(&pool, move |conn| {
        playerregistrations::table
            .filter(id.eq(payload.player_registration_id))
            .select(PlayerRegistration::as_select())
            .first(conn)
    }).await;
    match result {
        Ok(pr) => Ok(Json(LoadGameResponse::new(pr.gamestate))),
        Err(_) => Ok(Json(LoadGameResponse::new(String::new())))
    }
}

pub async fn leave_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<LeaveGamePayload>,
) -> ApiResponse<LeaveGameResponse> {
    let registration = run_query(&pool, move |conn| {
        playerregistrations::table
            .filter(player.eq(payload.player_id))
            .filter(game.eq(payload.game_id))
            .select(PlayerRegistration::as_select())
            .first(conn)
    }).await?;
    let rows_updated = run_query(&pool, move |conn| {
        diesel::update(playerregistrations::table.find(registration.id))
            .set(leftat.eq(Utc::now().date_naive()))
            .execute(conn)
    }).await;
    match rows_updated {
        Ok(rows) => Ok(Json(LeaveGameResponse::new(rows == 1))),
        Err(_) => Ok(Json(LeaveGameResponse::new(false)))
    }
}

pub async fn set_game_lang(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<SetGameLangPayload>,
) -> ApiResponse<SetGameLangResponse> {
    let game_result = run_query(&pool, move |conn| {
        games::table
            .filter(games::id.eq(payload.game_id))
            .select(Game::as_select())
            .first(conn)
    }).await?;
    let course_result = run_query(&pool, move |conn| {
        courses::table
            .filter(courses::id.eq(game_result.course))
            .select(Course::as_select())
            .first(conn)
    }).await?;
    if course_result.languages
        .replace(" ", "")
        .split(',')
        .any(|lang| lang == payload.language) {
            let registration_result = run_query(&pool, move |conn| {
                playerregistrations::table
                    .filter(player.eq(payload.player_id))
                    .filter(game.eq(payload.game_id))
                    .select(PlayerRegistration::as_select())
                    .first(conn)
            }).await?;
            let rows_updated = run_query(&pool, move |conn| {
                diesel::update(playerregistrations::table.find(registration_result.id))
                    .set(language.eq(payload.language))
                    .execute(conn)
            }).await?;
            Ok(Json(SetGameLangResponse::new(rows_updated == 1)))
        } else {
            Ok(Json(SetGameLangResponse::new(false)))
    }
}

pub async fn get_player_games(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetPlayerGamesPayload>,
) -> ApiResponse<GetPlayerGamesResponse> {
    let games_result = run_query(&pool, move |conn| {
        if payload.active {
            playerregistrations::table
                .inner_join(games::table.on(game.eq(games::id)))
                .filter(player.eq(payload.player_id))
                .filter(active.eq(true))
                .filter(leftat.is_null())
                .select(id)
                .load(conn)
        } else {
            playerregistrations::table
                .filter(player.eq(payload.player_id))
                .select(id)
                .load(conn)
        }
    }).await?;
    Ok(Json(GetPlayerGamesResponse::new(games_result)))
}

// pub async fn get_game_metadata(
//     State(pool): State<deadpool_diesel::postgres::Pool>,
//     Json(payload): Json<GetGameMetadataPayload>,
// ) -> Result<Json<GetGameMetadataResponse>, (StatusCode, String)> {
//     let conn = pool.get().await.map_err(utils::internal_error)?;
//     let res = conn
//         .interact(move |conn| {
//             playerregistrations::table
//                 .inner_join(games::table.on(game.eq(games::id)))
//                 .filter(id.eq(payload.player_registrations_id))
//                 .select((PlayerRegistration::as_select(), Game::as_select()))
//                 .first(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     Ok(Json(GetGameMetadataResponse::new(res)))
// }
//
// pub async fn get_course_data(
//     State(pool): State<deadpool_diesel::postgres::Pool>,
//     Json(payload): Json<GetCourseDataPayload>,
// ) -> Result<Json<GetCourseDataResponse>, (StatusCode, String)> {
//     let conn = pool.get().await.map_err(utils::internal_error)?;
//     let res = conn
//         .interact(move |conn| {
//             games::table
//                 .filter(games::id.eq(payload.game_id))
//                 .select(Game::as_select())
//                 .first(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     let course = conn
//         .interact(move |conn| {
//             courses::table
//                 .filter(courses::id.eq(res.course))
//                 .select(Course::as_select())
//                 .first(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     let res = conn
//         .interact(move |conn| {
//             modules::table
//                 .filter(modules::course.eq(course.id))
//                 .filter(modules::language.eq(payload.language))
//                 .select(modules::id)
//                 .load(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     Ok(Json(GetCourseDataResponse::new(
//         course.gamificationruleconditions,
//         course.gamificationcomplexrules,
//         course.gamificationruleresults,
//         res
//     )))
// }
//
// pub async fn get_module_data(
//     State(pool): State<deadpool_diesel::postgres::Pool>,
//     Json(payload): Json<GetModuleDataPayload>,
// ) -> Result<Json<GetModuleDataResponse>, (StatusCode, String)> {
//     let conn = pool.get().await.map_err(utils::internal_error)?;
//     let module = conn
//         .interact(move |conn| {
//             modules::table
//                 .filter(modules::id.eq(payload.module_id))
//                 .select(Module::as_select())
//                 .first(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     let res = conn
//         .interact(move |conn| {
//             exercises::table
//                 .filter(exercises::module.eq(payload.module_id))
//                 .filter(exercises::programminglanguage.eq(payload.programming_language))
//                 .filter(exercises::language.eq(payload.language))
//                 .select(exercises::id)
//                 .load(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     Ok(Json(GetModuleDataResponse::new(
//         module.order, module.title, module.description, module.startdate, module.enddate, res
//     )))
// }
//
// pub async fn get_exercise_data(
//     State(pool): State<deadpool_diesel::postgres::Pool>,
//     Json(payload): Json<GetExerciseDataPayload>,
// ) -> Result<Json<GetExerciseDataResponse>, (StatusCode, String)> {
//     let conn = pool.get().await.map_err(utils::internal_error)?;
//     let mut exercise = conn
//         .interact(move |conn| {
//             exercises::table
//                 .filter(exercises::id.eq(payload.exercise_id))
//                 .select(Exercise::as_select())
//                 .first(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     let player_unlock = conn
//         .interact(move |conn| {
//             playerunlocks::table
//                 .filter(playerunlocks::player.eq(payload.player_id))
//                 .filter(playerunlocks::exercise.eq(payload.exercise_id))
//                 .select(PlayerUnlock::as_select())
//                 .load(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     let res_game = conn
//         .interact(move |conn| {
//             games::table
//                 .filter(games::id.eq(payload.game_id))
//                 .select(Game::as_select())
//                 .first(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     exercise.hidden = exercise.hidden && player_unlock.is_empty();
//     exercise.locked = (exercise.locked || res_game.exerciselock) && player_unlock.is_empty(); //todo helper functions from mail
//     Ok(Json(GetExerciseDataResponse::new(exercise)))
// }
//
// pub async fn submit_solution(
//     State(pool): State<deadpool_diesel::postgres::Pool>,
//     Json(payload): Json<SubmitSolutionPayload>,
// ) -> Result<Json<SubmitSolutionResponse>, (StatusCode, String)> {
//     let conn = pool.get().await.map_err(utils::internal_error)?;
//     let res = conn
//         .interact(move |conn| {
//             submissions::table
//                 .filter(submissions::player.eq(payload.player_id))
//                 .filter(submissions::exercise.eq(payload.exercise_id))
//                 .select(Submission::as_select())
//                 .load(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     let _ = conn
//         .interact(move |conn| {
//             diesel::insert_into(submissions::table)
//                 .values(NewSubmission::new(
//                     payload.exercise_id,
//                     payload.player_id,
//                     payload.submission_client,
//                     payload.submission_submitted_code,
//                     payload.submission_metrics,
//                     payload.submission_result,
//                     payload.submission_result_description,
//                     payload.submission_feedback,
//                     payload.submission_earned_rewards,
//                     payload.submission_entered_at,
//                     Utc::now().date_naive()
//                 ))
//                 .returning(Submission::as_returning())
//                 .get_result(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     if res.is_empty() {
//         let module = conn
//             .interact(move |conn| {
//                 exercises::table
//                     .inner_join(modules::table.on(exercises::module.eq(modules::id)))
//                     .filter(exercises::id.eq(payload.exercise_id))
//                     .select(Module::as_select())
//                     .first(conn)
//             })
//             .await
//             .map_err(utils::internal_error)?
//             .map_err(utils::internal_error)?;
//
//         let game_res = conn
//             .interact(move |conn| {
//                 games::table
//                     .filter(games::course.eq(module.course))
//                     .select(Game::as_select())
//                     .first(conn)
//             })
//             .await
//             .map_err(utils::internal_error)?
//             .map_err(utils::internal_error)?;
//
//         let _ = conn
//             .interact(move |conn| {
//                 diesel::update(playerregistrations::table)
//                     .filter(playerregistrations::player.eq(payload.player_id))
//                     .filter(playerregistrations::game.eq(game_res.id))
//                     .set((
//                         playerregistrations::progress.eq(playerregistrations::progress + 1),
//                     ))
//                     .execute(conn)
//             })
//             .await
//             .map_err(utils::internal_error)?
//             .map_err(utils::internal_error)?;
//
//         //todo update playerrewards
//     }
//     Ok(Json(SubmitSolutionResponse::new(res.is_empty())))
// }
//
// pub async fn unlock(
//     State(pool): State<deadpool_diesel::postgres::Pool>,
//     Json(payload): Json<UnlockPayload>,
// ) -> Result<Json<()>, (StatusCode, String)> {
//     let conn = pool.get().await.map_err(utils::internal_error)?;
//     let _ = conn
//         .interact(move |conn| {
//             diesel::insert_into(playerunlocks::table)
//                 .values(PlayerUnlock::new(payload.player_id, payload.exercise_id, Utc::now().date_naive()))
//                 .returning(PlayerUnlock::as_returning())
//                 .get_result(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     Ok(Json(())) //todo revise
// }
//
// pub async fn get_last_solution(
//     State(pool): State<deadpool_diesel::postgres::Pool>,
//     Json(_): Json<GetLastSolutionPayload>,
// ) -> Result<Json<GetLastSolutionResponse>, (StatusCode, String)> {
//     let conn = pool.get().await.map_err(utils::internal_error)?;
//     let res = conn
//         .interact(move |conn| {
//             submissions::table
//                 .order(submissions::id.desc())
//                 .select(Submission::as_select())
//                 .first(conn)
//         })
//         .await
//         .map_err(utils::internal_error)?
//         .map_err(utils::internal_error)?;
//     Ok(Json(GetLastSolutionResponse::new(
//         res.submittedcode,
//         res.metrics,
//         res.result,
//         res.resultdescription,
//         res.feedback
//     )))//todo revise
// }