use crate::model::{
    CourseDataResponse, ExerciseDataResponse, GameMetadata, LastSolutionResponse,
    ModuleDataResponse, NewPlayerReward, NewPlayerUnlock, NewSubmission,
};
use crate::payloads::{
    GetCourseDataParams, GetExerciseDataParams, GetLastSolutionParams, GetModuleDataParams,
    GetPlayerGamesParams, JoinGamePayload, LeaveGamePayload, LoadGamePayload, SaveGamePayload,
    SetGameLangPayload, SubmitSolutionPayload, UnlockPayload,
};
use crate::{
    errors::AppError,
    model::NewPlayerRegistration,
    response::ApiResponse,
    schema::{
        courses::dsl as courses_dsl, exercises::dsl as exercises_dsl, games::dsl as games_dsl,
        modules::dsl as modules_dsl, player_registrations::dsl as prs_dsl,
        player_unlocks::dsl as pus_dsl, players::dsl as players_dsl, rewards::dsl as rewards_dsl,
        submissions::dsl as sub_dsl,
    },
};
use anyhow::anyhow;
use axum::extract::{Path, Query};
use axum::{extract::State, response::Json};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, Duration, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::now;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use serde_json::Value as JsonValue;
use serde_json::json;
use tracing::log::warn;
use tracing::{debug, error, info, instrument};

mod helper;

/// Query all available games that are public and active.
///
/// Parameters
/// * N/A
///
/// Returns (wrapped in `ApiResponse`)
/// * game_ids as `Vec<i64>` if succeeded (200)
/// * `None` if failed (500)
#[instrument(skip(pool))]
pub async fn get_available_games(
    State(pool): State<Pool>,
) -> Result<ApiResponse<Vec<i64>>, AppError> {
    info!("Fetching available games");

    let game_ids = helper::run_query(&pool, |conn_sync| {
        games_dsl::games
            .filter(games_dsl::active.eq(true).and(games_dsl::public.eq(true)))
            .select(games_dsl::id)
            .load::<i64>(conn_sync)
    })
    .await?;

    info!("Successfully fetched {} available game IDs", game_ids.len());
    Ok(ApiResponse::ok(game_ids))
}

/// Adds a player to a game.
///
/// Parameters
/// * player_id as `i64`
/// * game_id as `i64`
/// * language as `String`
///
/// Returns (wrapped in `ApiResponse`)
/// * player_registrations_id as `i64` if succeeded (200)
/// * `None` if not found (404)
/// * `None` if failed (500)
#[instrument(skip(pool, payload))]
pub async fn join_game(
    State(pool): State<Pool>,
    Json(payload): Json<JoinGamePayload>,
) -> Result<ApiResponse<i64>, AppError> {
    info!(
        "Attempting to join game for player_id: {}",
        payload.player_id
    );
    debug!("Join game payload: {:?}", payload);

    let new_registration = NewPlayerRegistration {
        player_id: payload.player_id,
        game_id: payload.game_id,
        language: payload.language,
        progress: 0,
        game_state: json!({}),
    };

    let insert_result = helper::run_query(&pool, move |conn_sync| {
        diesel::insert_into(prs_dsl::player_registrations)
            .values(&new_registration)
            .returning(crate::schema::player_registrations::id)
            .get_result::<i64>(conn_sync)
    })
    .await;

    match insert_result {
        Ok(new_id) => {
            info!(
                "Player {} successfully joined game {}, registration_id: {}",
                payload.player_id, payload.game_id, new_id
            );
            Ok(ApiResponse::ok(new_id))
        }
        Err(AppError::DieselError(DieselError::DatabaseError(
            DatabaseErrorKind::ForeignKeyViolation,
            _,
        ))) => {
            warn!(
                "Failed to join game due to foreign key violation for player_id: {} or game_id: {}",
                payload.player_id, payload.game_id,
            );
            Err(AppError::NotFound(format!(
                "Player with ID {} or Game with ID {} not found.",
                payload.player_id, payload.game_id,
            )))
        }
        Err(e) => Err(e),
    }
}

/// Saves a game state.
///
/// Parameters
/// * player_registrations_id as `i64`
/// * game_state as `serde_json::Value`
///
/// Returns (wrapped in `ApiResponse`)
/// * success as `bool` if succeeded (200)
/// * `None` if not found (404)
/// * `None` if failed or affected more than 1 row (500)
#[instrument(skip(pool, payload))]
pub async fn save_game(
    State(pool): State<Pool>,
    Json(payload): Json<SaveGamePayload>,
) -> Result<ApiResponse<bool>, AppError> {
    info!(
        "Attempting to save game state for registration_id: {}",
        payload.player_registrations_id
    );
    debug!("Save game payload: {:?}", payload);

    let rows_affected = helper::run_query(&pool, move |conn_sync| {
        let target =
            prs_dsl::player_registrations.filter(prs_dsl::id.eq(payload.player_registrations_id));

        diesel::update(target)
            .set((
                prs_dsl::game_state.eq(payload.game_state),
                prs_dsl::saved_at.eq(now),
            ))
            .execute(conn_sync)
    })
    .await?;

    match rows_affected {
        0 => {
            error!(
                "Not found, game state not saved for registration_id: {}",
                payload.player_registrations_id
            );
            Err(AppError::NotFound(format!(
                "Player registration with ID {} not found",
                payload.player_registrations_id
            )))
        }
        1 => {
            info!(
                "Successfully saved game state for registration_id: {}",
                payload.player_registrations_id
            );
            Ok(ApiResponse::ok(true))
        }
        n => {
            error!(
                "Expected 1 row to be affected by update, but {} rows were affected for registration_id: {}",
                n, payload.player_registrations_id
            );
            Err(AppError::Internal(anyhow!(format!(
                "Update affected {} rows, expected 1",
                n
            ))))
        }
    }
}

/// Queries a game state.
///
/// Parameters
/// * player_registrations_id as `i64`
///
/// Returns (wrapped in `ApiResponse`)
/// * game_state as `serde_json::Value`` if succeeded (200)
/// * `None` if not found (404)
/// * `None` if failed (500)
#[instrument(skip(pool, payload))]
pub async fn load_game(
    State(pool): State<Pool>,
    Json(payload): Json<LoadGamePayload>,
) -> Result<ApiResponse<JsonValue>, AppError> {
    info!(
        "Attempting to load game state for registration_id: {}",
        payload.player_registrations_id
    );

    let loaded_game_state = helper::run_query(&pool, move |conn_sync| {
        prs_dsl::player_registrations
            .filter(prs_dsl::id.eq(payload.player_registrations_id))
            .select(prs_dsl::game_state)
            .get_result::<JsonValue>(conn_sync)
    })
    .await;

    match loaded_game_state {
        Ok(state) => {
            info!(
                "Successfully loaded game state for registration_id: {}",
                payload.player_registrations_id
            );
            Ok(ApiResponse::ok(state))
        }
        Err(AppError::DieselError(diesel::result::Error::NotFound)) => {
            error!(
                "Attempted to load game state for non-existent registration_id: {}",
                payload.player_registrations_id
            );
            Err(AppError::NotFound(format!(
                "Player registration with ID {} not found",
                payload.player_registrations_id
            )))
        }
        Err(e) => Err(e),
    }
}

/// Marks a player's registration in a game as inactive by setting the 'left_at' timestamp.
///
/// Parameters
/// * player_id as `i64`
/// * game_id as `i64`
///
/// Returns (wrapped in `ApiResponse`)
/// * `None` if succeeded (200)
/// * `None` if player registration for the given game not found (404)
/// * `None` if failed or affected more than 1 row (500)
#[instrument(skip(pool, payload))]
pub async fn leave_game(
    State(pool): State<Pool>,
    Json(payload): Json<LeaveGamePayload>,
) -> Result<ApiResponse<()>, AppError> {
    info!(
        "Attempting to mark player {} as left game {}",
        payload.player_id, payload.game_id
    );
    debug!("Leave game payload: {:?}", payload);

    let rows_affected = helper::run_query(&pool, move |conn_sync| {
        let target = prs_dsl::player_registrations.filter(
            prs_dsl::player_id
                .eq(payload.player_id)
                .and(prs_dsl::game_id.eq(payload.game_id)),
        );

        diesel::update(target)
            .set(prs_dsl::left_at.eq(now))
            .execute(conn_sync)
    })
    .await?;

    match rows_affected {
        0 => {
            error!(
                "Player registration not found for player_id: {} and game_id: {}. Cannot mark as left.",
                payload.player_id, payload.game_id
            );
            Err(AppError::NotFound(format!(
                "Player registration not found for player ID {} and game ID {}",
                payload.player_id, payload.game_id
            )))
        }
        1 => {
            info!(
                "Successfully marked player {} as left game {}",
                payload.player_id, payload.game_id
            );
            Ok(ApiResponse::ok(()))
        }
        n => {
            error!(
                "Expected 1 row to be affected by leave_game update, but {} rows were affected for player_id: {}, game_id: {}",
                n, payload.player_id, payload.game_id
            );
            Err(AppError::Internal(anyhow!(format!(
                "Update affected {} rows, expected 1 for player {} in game {}",
                n, payload.player_id, payload.game_id
            ))))
        }
    }
}

/// Sets the language for a player's registration in a game,
/// but only if the language is allowed by the game's associated course.
///
/// Parameters
/// * player_id as `i64`
/// * game_id as `i64`
/// * language as `String`
///
/// Returns (wrapped in `ApiResponse`)
/// * success as `bool` (true) if succeeded (200)
/// * `None` if player registration not found OR if the language is invalid for the course (404)
/// * `None` if failed for other reasons (500)
#[instrument(skip(pool, payload))]
pub async fn set_game_lang(
    State(pool): State<Pool>,
    Json(payload): Json<SetGameLangPayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let player_id = payload.player_id;
    let game_id = payload.game_id;
    let language = payload.language.clone();

    info!(
        "Attempting to set language to '{}' for player {} in game {}",
        language, player_id, game_id
    );
    debug!("Set game lang payload: {:?}", payload);

    let fetch_result = helper::run_query(&pool, move |conn_sync| {
        prs_dsl::player_registrations
            .filter(prs_dsl::player_id.eq(player_id))
            .filter(prs_dsl::game_id.eq(game_id))
            .inner_join(games_dsl::games.on(prs_dsl::game_id.eq(games_dsl::id)))
            .inner_join(courses_dsl::courses.on(games_dsl::course_id.eq(courses_dsl::id)))
            .select(courses_dsl::languages)
            .first::<String>(conn_sync)
    })
    .await;

    let allowed_languages_str = match fetch_result {
        Ok(langs) => langs,
        Err(AppError::DieselError(DieselError::NotFound)) => {
            error!(
                "Player registration not found for player_id: {} and game_id: {}. Cannot set language.",
                player_id, game_id
            );
            return Err(AppError::NotFound(format!(
                "Player registration not found for player ID {} and game ID {}",
                player_id, game_id
            )));
        }
        Err(e) => {
            error!(
                "Database error fetching allowed languages for player {} game {}: {:?}",
                player_id, game_id, e
            );
            return Err(e);
        }
    };

    let allowed_languages: Vec<&str> = allowed_languages_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if !allowed_languages.contains(&language.as_str()) {
        warn!(
            "Attempted to set invalid language '{}' for player {} in game {}. Allowed: {:?}. Returning 404.",
            language, player_id, game_id, allowed_languages
        );
        return Err(AppError::NotFound(format!(
            "Language '{}' is not valid or available for the registration of player ID {} in game ID {}",
            language, player_id, game_id
        )));
    }

    let update_result = helper::run_query(&pool, move |conn_sync| {
        let target = prs_dsl::player_registrations
            .filter(prs_dsl::player_id.eq(player_id))
            .filter(prs_dsl::game_id.eq(game_id));

        diesel::update(target)
            .set(prs_dsl::language.eq(language))
            .execute(conn_sync)
    })
    .await?;

    match update_result {
        1 => {
            info!(
                "Successfully set language to '{}' for player {} in game {}",
                payload.language, player_id, game_id
            );
            Ok(ApiResponse::ok(true))
        }
        0 => {
            error!(
                "Update affected 0 rows after validation for player_id: {}, game_id: {}",
                player_id, game_id
            );
            Err(AppError::Internal(anyhow!(
                "Registration found but update failed for player {} game {}",
                player_id,
                game_id
            )))
        }
        n => {
            error!(
                "Expected 1 row to be affected by set_game_lang update, but {} rows were affected for player_id: {}, game_id: {}",
                n, player_id, game_id
            );
            Err(AppError::Internal(anyhow!(format!(
                "Update affected {} rows, expected 1 for player {} in game {}",
                n, player_id, game_id
            ))))
        }
    }
}

/// Retrieves player registration IDs for a given player.
/// Can filter for active registrations only.
///
/// Query Parameters:
/// * player_id as `i64`
/// * active as `bool` If true, only return registrations where the game is active
///           and the player has not left (left_at is NULL).
///
/// Returns (wrapped in `ApiResponse`)
/// * `Vec<i64>` of player_registrations ids if succeeded (200)
/// * `None` if the specified player_id does not exist (404).
/// * `None` if failed (500)
#[instrument(skip(pool, params))]
pub async fn get_player_games(
    State(pool): State<Pool>,
    Query(params): Query<GetPlayerGamesParams>,
) -> Result<ApiResponse<Vec<i64>>, AppError> {
    let player_id = params.player_id;
    let only_active = params.active;

    info!(
        "Fetching player registrations for player_id: {}. Active only: {}",
        player_id, only_active
    );
    debug!("Get player games params: {:?}", params);

    let player_check_result = helper::run_query(&pool, move |conn| {
        players_dsl::players
            .filter(players_dsl::id.eq(player_id))
            .select(players_dsl::id)
            .first::<i64>(conn)
    })
    .await;

    match player_check_result {
        Ok(_) => info!("Player {} found. Fetching registrations...", player_id),
        Err(AppError::DieselError(DieselError::NotFound)) => {
            error!("Player with ID {} not found.", player_id);
            return Err(AppError::NotFound(format!(
                "Player with ID {} not found.",
                player_id
            )));
        }
        Err(e) => {
            error!(
                "Database error checking player existence for ID {}: {:?}",
                player_id, e
            );
            return Err(e);
        }
    }

    let registration_ids = helper::run_query(&pool, move |conn_sync| {
        if only_active {
            prs_dsl::player_registrations
                .filter(prs_dsl::player_id.eq(player_id))
                .filter(prs_dsl::left_at.is_null())
                .inner_join(games_dsl::games.on(prs_dsl::game_id.eq(games_dsl::id)))
                .filter(games_dsl::active.eq(true))
                .select(prs_dsl::id)
                .load::<i64>(conn_sync)
        } else {
            prs_dsl::player_registrations
                .filter(prs_dsl::player_id.eq(player_id))
                .select(prs_dsl::id)
                .load::<i64>(conn_sync)
        }
    })
    .await?;

    info!(
        "Successfully fetched {} registrations for player_id: {}",
        registration_ids.len(),
        player_id
    );
    Ok(ApiResponse::ok(registration_ids))
}

/// Retrieves detailed metadata for a specific player registration and its associated game.
///
/// Path Parameters:
/// * registration_id as `i64`: The ID of the player_registration record.
///
/// Returns (wrapped in `ApiResponse`)
/// * `GameMetadata` struct if succeeded (200).
/// * `None` if the specified player_registrations_id does not exist (404).
/// * `None` if failed for other reasons (500).
#[instrument(skip(pool))]
pub async fn get_game_metadata(
    State(pool): State<Pool>,
    Path(registration_id): Path<i64>,
) -> Result<ApiResponse<GameMetadata>, AppError> {
    info!(
        "Fetching game metadata for registration_id: {}",
        registration_id
    );

    type QueryResultTuple = (
        i64,                   // id
        i32,                   // progress
        DateTime<Utc>,         // joined_at
        Option<DateTime<Utc>>, // left_at
        String,                // language
        i64,                   // id
        String,                // title
        bool,                  // active
        String,                // description
        String,                // programming_language
        i32,                   // total_exercises
        DateTime<Utc>,         // start_date
        DateTime<Utc>,         // end_date
    );

    let query_result = helper::run_query(&pool, move |conn_sync| {
        prs_dsl::player_registrations
            .filter(prs_dsl::id.eq(registration_id))
            .inner_join(games_dsl::games.on(prs_dsl::game_id.eq(games_dsl::id)))
            .select((
                prs_dsl::id,
                prs_dsl::progress,
                prs_dsl::joined_at,
                prs_dsl::left_at,
                prs_dsl::language,
                games_dsl::id,
                games_dsl::title,
                games_dsl::active,
                games_dsl::description,
                games_dsl::programming_language,
                games_dsl::total_exercises,
                games_dsl::start_date,
                games_dsl::end_date,
            ))
            .first::<QueryResultTuple>(conn_sync)
    })
    .await;

    match query_result {
        Ok(data) => {
            let metadata = GameMetadata {
                registration_id: data.0,
                progress: data.1,
                joined_at: data.2,
                left_at: data.3,
                language: data.4,
                game_id: data.5,
                game_title: data.6,
                game_active: data.7,
                game_description: data.8,
                game_programming_language: data.9,
                game_total_exercises: data.10,
                game_start_date: data.11,
                game_end_date: data.12,
            };
            info!(
                "Successfully fetched game metadata for registration_id: {}",
                registration_id
            );
            Ok(ApiResponse::ok(metadata))
        }
        Err(AppError::DieselError(DieselError::NotFound)) => {
            error!("Player registration with ID {} not found.", registration_id);
            Err(AppError::NotFound(format!(
                "Player registration with ID {} not found.",
                registration_id
            )))
        }
        Err(e) => Err(e),
    }
}

/// Retrieves course gamification data and relevant module IDs for a specific game and language.
///
/// Parameters:
/// * game_id as `i64`: The ID of the game.
/// * language: The language to filter modules by.
///
/// Returns (wrapped in `ApiResponse`)
/// * `CourseDataResponse` struct if succeeded (200).
/// * `None` if the specified game_id or its associated course does not exist (404).
/// * `None` if failed for other reasons (500).
#[instrument(skip(pool, params))]
pub async fn get_course_data(
    State(pool): State<Pool>,
    Query(params): Query<GetCourseDataParams>,
) -> Result<ApiResponse<CourseDataResponse>, AppError> {
    let language = params.language;
    let game_id = params.game_id;

    info!(
        "Fetching course data for game_id: {} and language: {}",
        game_id, language
    );
    debug!(
        "Get course data params: game_id={}, language={}",
        game_id, language
    );

    type CourseInfoTuple = (i64, String, String, String); // course_id, conditions, complex, results

    let course_info_result = helper::run_query(&pool, move |conn_sync| {
        games_dsl::games
            .filter(games_dsl::id.eq(game_id))
            .inner_join(courses_dsl::courses.on(games_dsl::course_id.eq(courses_dsl::id)))
            .select((
                courses_dsl::id,
                courses_dsl::gamification_rule_conditions,
                courses_dsl::gamification_complex_rules,
                courses_dsl::gamification_rule_results,
            ))
            .first::<CourseInfoTuple>(conn_sync)
    })
    .await;

    let (course_id, conditions, complex_rules, results) = match course_info_result {
        Ok(data) => data,
        Err(AppError::DieselError(DieselError::NotFound)) => {
            error!(
                "Game with ID {} or its associated course not found.",
                game_id
            );
            return Err(AppError::NotFound(format!(
                "Game with ID {} or its associated course not found.",
                game_id
            )));
        }
        Err(e) => return Err(e),
    };

    let lang_for_modules = language.clone();
    let module_ids_result = helper::run_query(&pool, move |conn_sync| {
        modules_dsl::modules
            .filter(modules_dsl::course_id.eq(course_id))
            .filter(modules_dsl::language.eq(lang_for_modules))
            .select(modules_dsl::id)
            .load::<i64>(conn_sync)
    })
    .await?;

    let response_data = CourseDataResponse {
        gamification_rule_conditions: conditions,
        gamification_complex_rules: complex_rules,
        gamification_rule_results: results,
        module_ids: module_ids_result,
    };

    info!(
        "Successfully fetched course data and {} module IDs for game_id: {} and language: {}",
        response_data.module_ids.len(),
        game_id,
        language
    );
    Ok(ApiResponse::ok(response_data))
}

/// Retrieves module details and filtered exercise IDs.
///
/// Parameters:
/// * module_id as `i64`: The ID of the module.
/// * language as `String`: The language to filter exercises by.
/// * programming_language as `String`: The programming language to filter exercises by.
///
/// Returns (wrapped in `ApiResponse`)
/// * `ModuleDataResponse` struct if succeeded (200).
/// * `None` if the specified module_id does not exist (404).
/// * `None` if failed for other reasons (500).
#[instrument(skip(pool, params))]
pub async fn get_module_data(
    State(pool): State<Pool>,
    Query(params): Query<GetModuleDataParams>,
) -> Result<ApiResponse<ModuleDataResponse>, AppError> {
    let language = params.language.clone();
    let programming_language = params.programming_language.clone();
    let module_id = params.module_id;

    info!(
        "Fetching data for module_id: {}, language: {}, programming_language: {}",
        module_id, language, programming_language
    );
    debug!(
        "Get module data params: module_id={}, language={}, programming_language={}",
        module_id, language, programming_language
    );

    type ModuleInfoTuple = (i32, String, String, DateTime<Utc>, DateTime<Utc>); // order, title, desc, start, end

    let module_info_result = helper::run_query(&pool, move |conn_sync| {
        modules_dsl::modules
            .filter(modules_dsl::id.eq(module_id))
            .select((
                modules_dsl::order,
                modules_dsl::title,
                modules_dsl::description,
                modules_dsl::start_date,
                modules_dsl::end_date,
            ))
            .first::<ModuleInfoTuple>(conn_sync)
    })
    .await;

    let (order, title, description, start_date, end_date) = match module_info_result {
        Ok(data) => data,
        Err(AppError::DieselError(DieselError::NotFound)) => {
            error!("Module with ID {} not found.", module_id);
            return Err(AppError::NotFound(format!(
                "Module with ID {} not found.",
                module_id
            )));
        }
        Err(e) => return Err(e),
    };

    let module_id_for_exercises = module_id;
    let exercise_ids_result = helper::run_query(&pool, move |conn_sync| {
        exercises_dsl::exercises
            .filter(exercises_dsl::module_id.eq(module_id_for_exercises))
            .filter(exercises_dsl::language.eq(language))
            .filter(exercises_dsl::programming_language.eq(programming_language))
            .select(exercises_dsl::id)
            .load::<i64>(conn_sync)
    })
    .await?;

    let response_data = ModuleDataResponse {
        order,
        title,
        description,
        start_date,
        end_date,
        exercise_ids: exercise_ids_result,
    };

    info!(
        "Successfully fetched data and {} exercise IDs for module_id: {}",
        response_data.exercise_ids.len(),
        module_id
    );
    Ok(ApiResponse::ok(response_data))
}

/// Retrieves detailed exercise data, calculating context-dependent hidden/locked status.
///
/// Parameters:
/// * exercise_id as `i64`: The ID of the exercise.
/// * game_id as `i64`: The ID of the current game context.
/// * player_id as `i64`: The ID of the current player context.
///
/// Returns (wrapped in `ApiResponse`)
/// * `ExerciseDataResponse` struct if succeeded (200).
/// * `None` if the specified exercise_id or game_id does not exist (404).
/// * `None` if failed for other reasons (500).
#[instrument(skip(pool, params))]
pub async fn get_exercise_data(
    State(pool): State<Pool>,
    Query(params): Query<GetExerciseDataParams>,
) -> Result<ApiResponse<ExerciseDataResponse>, AppError> {
    let exercise_id = params.exercise_id;
    let game_id = params.game_id;
    let player_id = params.player_id;

    info!(
        "Fetching data for exercise_id: {}, game_id: {}, player_id: {}",
        exercise_id, game_id, player_id
    );
    debug!(
        "Get exercise data params: exercise_id={}, game_id={}, player_id={}",
        exercise_id, game_id, player_id
    );

    type ExerciseInfoTuple = (
        i64,       // module_id
        i32,       // order
        String,    // title
        String,    // description
        String,    // init_code
        String,    // pre_code
        String,    // post_code
        String,    // test_code
        String,    // check_source
        String,    // mode
        JsonValue, // mode_parameters
        String,    // difficulty
        bool,      // raw hidden
        bool,      // raw locked
    );

    let exercise_info_result = helper::run_query(&pool, move |conn| {
        exercises_dsl::exercises
            .find(exercise_id)
            .select((
                exercises_dsl::module_id,
                exercises_dsl::order,
                exercises_dsl::title,
                exercises_dsl::description,
                exercises_dsl::init_code,
                exercises_dsl::pre_code,
                exercises_dsl::post_code,
                exercises_dsl::test_code,
                exercises_dsl::check_source,
                exercises_dsl::mode,
                exercises_dsl::mode_parameters,
                exercises_dsl::difficulty,
                exercises_dsl::hidden,
                exercises_dsl::locked,
            ))
            .first::<ExerciseInfoTuple>(conn)
    })
    .await;

    let (
        module_id,
        order,
        title,
        description,
        init_code,
        pre_code,
        post_code,
        test_code,
        check_source,
        mode,
        mode_parameters,
        difficulty,
        exercise_raw_hidden,
        exercise_raw_locked,
    ) = match exercise_info_result {
        Ok(data) => data,
        Err(AppError::DieselError(DieselError::NotFound)) => {
            error!("Exercise with ID {} not found.", exercise_id);
            return Err(AppError::NotFound(format!(
                "Exercise with ID {} not found.",
                exercise_id
            )));
        }
        Err(e) => return Err(e),
    };

    type GameInfoTuple = (f64, bool); // module_lock, exercise_lock
    let game_info_result = helper::run_query(&pool, move |conn| {
        games_dsl::games
            .find(game_id)
            .select((games_dsl::module_lock, games_dsl::exercise_lock))
            .first::<GameInfoTuple>(conn)
    })
    .await;

    let (game_module_lock, game_exercise_lock) = match game_info_result {
        Ok(data) => data,
        Err(AppError::DieselError(DieselError::NotFound)) => {
            error!("Game with ID {} not found.", game_id);
            return Err(AppError::NotFound(format!(
                "Game with ID {} not found.",
                game_id
            )));
        }
        Err(e) => return Err(e),
    };

    let has_unlock = helper::run_query(&pool, move |conn| {
        diesel::dsl::select(diesel::dsl::exists(
            pus_dsl::player_unlocks
                .filter(pus_dsl::player_id.eq(player_id))
                .filter(pus_dsl::exercise_id.eq(exercise_id)),
        ))
        .get_result::<bool>(conn)
    })
    .await?;

    let hidden_flag = exercise_raw_hidden && !has_unlock;

    let mut is_locked_by_condition = exercise_raw_locked;

    if !is_locked_by_condition && game_module_lock > 0.0 {
        let total_module_exercises_res = helper::run_query(&pool, move |conn| {
            exercises_dsl::exercises
                .filter(exercises_dsl::module_id.eq(module_id))
                .count()
                .get_result::<i64>(conn)
        })
        .await;

        let total_module_exercises = match total_module_exercises_res {
            Ok(count) => count,
            Err(_) => {
                return Err(AppError::Internal(anyhow!(
                    "Failed to count exercises in module"
                )));
            }
        };

        if total_module_exercises > 0 {
            let solved_in_module_res = helper::run_query(&pool, move |conn| {
                sub_dsl::submissions
                    .filter(sub_dsl::player_id.eq(player_id))
                    .filter(sub_dsl::game_id.eq(game_id))
                    .filter(
                        sub_dsl::result
                            .gt(BigDecimal::from_f64(0.0).expect("0.0 is valid BigDecimal")),
                    )
                    .inner_join(
                        exercises_dsl::exercises.on(sub_dsl::exercise_id.eq(exercises_dsl::id)),
                    )
                    .filter(exercises_dsl::module_id.eq(module_id))
                    .select(diesel::dsl::count_distinct(sub_dsl::exercise_id))
                    .get_result::<i64>(conn)
            })
            .await;

            let solved_in_module = match solved_in_module_res {
                Ok(count) => count,
                Err(_) => {
                    return Err(AppError::Internal(anyhow!(
                        "Failed to count solved exercises"
                    )));
                }
            };

            let solved_ratio = solved_in_module as f64 / total_module_exercises as f64;
            if solved_ratio >= game_module_lock {
                is_locked_by_condition = true;
            }
        }
    }

    if !is_locked_by_condition && game_exercise_lock && order > 1 {
        let prev_exercise_id_res = helper::run_query(&pool, move |conn| {
            exercises_dsl::exercises
                .filter(exercises_dsl::module_id.eq(module_id))
                .filter(exercises_dsl::order.eq(order - 1))
                .select(exercises_dsl::id)
                .first::<i64>(conn)
                .optional()
        })
        .await?;

        if let Some(prev_exercise_id) = prev_exercise_id_res {
            let prev_solved = helper::run_query(&pool, move |conn| {
                diesel::dsl::select(diesel::dsl::exists(
                    sub_dsl::submissions
                        .filter(sub_dsl::player_id.eq(player_id))
                        .filter(sub_dsl::game_id.eq(game_id))
                        .filter(sub_dsl::exercise_id.eq(prev_exercise_id))
                        .filter(
                            sub_dsl::result
                                .gt(BigDecimal::from_f64(0.0).expect("0.0 is valid BigDecimal")),
                        ),
                ))
                .get_result::<bool>(conn)
            })
            .await?;

            if !prev_solved {
                is_locked_by_condition = true;
            }
        }
    }

    let locked_flag = is_locked_by_condition && !has_unlock;

    let response_data = ExerciseDataResponse {
        order,
        title,
        description,
        init_code,
        pre_code,
        post_code,
        test_code,
        check_source,
        mode,
        mode_parameters,
        difficulty,
        hidden: hidden_flag,
        locked: locked_flag,
    };

    info!(
        "Successfully fetched data for exercise_id: {} (Hidden: {}, Locked: {})",
        exercise_id, hidden_flag, locked_flag
    );
    Ok(ApiResponse::ok(response_data))
}

/// Submits a solution attempt for an exercise.
///
/// Parameters:
/// * player_id as `i64`
/// * exercise_id as `i64`
/// * game_id as `i64`
/// * client as `String`
/// * submitted_code as `String`
/// * metrics as `serde_json::Value`
/// * result as `BigDecimal`
/// * result_description as `serde_json::Value`
/// * first_solution as `bool`
/// * feedback as `String`
/// * earned_rewards as `serde_json::Value`
/// * entered_at as `DateTime<Utc>`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if this was the first *correct* submission for the exercise/player/game, false otherwise (200).
/// * `None` if player, game, exercise, reward or registration not found (404).
/// * `None` if failed for other reasons (500).
#[instrument(skip(pool, payload))]
pub async fn submit_solution(
    State(pool): State<Pool>,
    Json(payload): Json<SubmitSolutionPayload>,
) -> Result<ApiResponse<bool>, AppError> {
    info!(
        "Attempting submission for exercise_id: {}, player_id: {}, game_id: {}",
        payload.exercise_id, payload.player_id, payload.game_id
    );
    debug!("Submit solution payload: {:?}", payload);

    let conn = pool.get().await.map_err(|pool_err| {
        error!(
            "Failed to get DB connection from pool for transaction: {:?}",
            pool_err
        );
        AppError::PoolError(pool_err)
    })?;
    debug!("DB connection obtained from pool for transaction");

    let transaction_result = conn.interact(move |conn_sync| {
        conn_sync.transaction(|transaction_conn| {
            let player_id = payload.player_id;
            let exercise_id = payload.exercise_id;
            let game_id = payload.game_id;
            let current_result_is_correct = payload.result > BigDecimal::from(0);

            let map_err = |e: DieselError| -> AppError {
                AppError::DieselError(e)
            };

            let registration_exists = diesel::dsl::select(diesel::dsl::exists(
                prs_dsl::player_registrations
                    .filter(prs_dsl::player_id.eq(player_id))
                    .filter(prs_dsl::game_id.eq(game_id))
            )).get_result::<bool>(transaction_conn)
                .map_err(map_err)?;

            if !registration_exists {
                warn!("Player registration not found for player {} game {}. Cannot submit.", player_id, game_id);
                return Err(AppError::NotFound(format!(
                    "Player registration not found for player ID {} in game ID {}.",
                    player_id, game_id
                )));
            }

            let was_previously_solved = diesel::dsl::select(diesel::dsl::exists(
                sub_dsl::submissions
                    .filter(sub_dsl::player_id.eq(player_id))
                    .filter(sub_dsl::exercise_id.eq(exercise_id))
                    .filter(sub_dsl::game_id.eq(game_id))
                    .filter(sub_dsl::result.gt(BigDecimal::from_f64(0.0).
                        expect("0.0 is valid BigDecimal")))
            )).get_result::<bool>(transaction_conn)
                .map_err(map_err)?;

            let is_first_correct = current_result_is_correct && !was_previously_solved;

            let new_submission = NewSubmission {
                exercise_id,
                game_id,
                player_id,
                client: payload.client.clone(),
                submitted_code: payload.submitted_code.clone(),
                metrics: payload.metrics.clone(),
                result: payload.result.clone(),
                result_description: payload.result_description.clone(),
                first_solution: is_first_correct,
                feedback: payload.feedback.clone(),
                earned_rewards: payload.earned_rewards.clone(),
                entered_at: payload.entered_at,
            };

            diesel::insert_into(sub_dsl::submissions)
                .values(&new_submission)
                .execute(transaction_conn)
                .map_err(|e| {
                    if let DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) = e {
                        error!("Foreign key violation during submission insert: {:?}", e);
                        AppError::NotFound("Referenced player, game, or exercise not found.".to_string())
                    } else {
                        map_err(e)
                    }
                })?;

            if is_first_correct {
                info!("First correct submission for exercise {}, player {}, game {}. Updating progress.",
                      exercise_id, player_id, game_id);

                let rows_affected = diesel::update(
                    prs_dsl::player_registrations
                        .filter(prs_dsl::player_id.eq(player_id))
                        .filter(prs_dsl::game_id.eq(game_id))
                )
                    .set(prs_dsl::progress.eq(prs_dsl::progress + 1))
                    .execute(transaction_conn)
                    .map_err(map_err)?;

                if rows_affected != 1 {
                    error!("Failed to update progress for player {} game {}: Expected 1 row affected, got {}",
                           player_id, game_id, rows_affected);
                    return Err(AppError::Internal(anyhow!(
                        "Failed to update progress, inconsistent state."
                    )));
                }

                if let Some(rewards_array) = payload.earned_rewards.as_array() {
                    let now_ts = Utc::now();

                    for reward_val in rewards_array {
                        if let Some(reward_id_num) = reward_val.as_i64() {
                            let reward_id = reward_id_num;

                            let valid_period_opt = rewards_dsl::rewards
                                .find(reward_id)
                                .select(rewards_dsl::valid_period)
                                .first::<Option<Duration>>(transaction_conn)
                                .map_err(|e| match e {
                                    DieselError::NotFound => {
                                        error!("Reward ID {} specified in earned_rewards not found.", reward_id);
                                        AppError::Internal(anyhow!("Reward ID {} not found", reward_id))
                                    },
                                    _ => map_err(e),
                                })?;

                            let expires_at_ts = match valid_period_opt {
                                Some(interval) => {
                                    now_ts + interval
                                },
                                None => {
                                    error!("Reward ID {}, specified period was invalid", reward_id);
                                    return Err(AppError::Internal(anyhow!("Reward ID {}, specified period was invalid", reward_id)));
                                }
                            };

                            let new_player_reward = NewPlayerReward {
                                player_id,
                                reward_id,
                                game_id: Some(game_id),
                                count: 1,
                                used_count: 0,
                                obtained_at: now_ts,
                                expires_at: expires_at_ts,
                            };

                            diesel::insert_into(crate::schema::player_rewards::table)
                                .values(&new_player_reward)
                                .on_conflict((
                                    crate::schema::player_rewards::player_id,
                                    crate::schema::player_rewards::reward_id,
                                    crate::schema::player_rewards::game_id,
                                ))
                                .do_update()
                                .set(crate::schema::player_rewards::count.eq(crate::schema::player_rewards::count + 1))
                                .execute(transaction_conn)
                                .map_err(map_err)?;

                        } else {
                            warn!("Invalid non-integer reward ID found in earned_rewards: {:?}", reward_val);
                        }
                    }
                } else if !payload.earned_rewards.is_null() {
                    warn!("earned_rewards field was not a valid JSON array: {:?}", payload.earned_rewards);
                }

                let (game_module_lock, game_exercise_lock) = games_dsl::games
                    .find(game_id)
                    .select((games_dsl::module_lock, games_dsl::exercise_lock))
                    .first::<(f64, bool)>(transaction_conn)
                    .map_err(|e| match e {
                        DieselError::NotFound => {
                            error!("Game with ID {} not found during unlock check.", game_id);
                            AppError::NotFound(format!("Game with ID {} not found.", game_id))
                        },
                        _ => map_err(e),
                    })?;

                if game_module_lock > 0.0 || game_exercise_lock {
                    info!("Game lock conditions met, attempting unlock for exercise {} player {}", exercise_id, player_id);
                    internal_unlock_exercise(transaction_conn, player_id, exercise_id)?;
                }

            }

            Ok(is_first_correct)

        })
    }).await;

    let was_first_correct_submission = match transaction_result {
        Ok(Ok(result)) => result,
        Ok(Err(app_err)) => {
            if let AppError::Internal(_) = app_err {
                error!("Transaction failed with internal error: {:?}", app_err);
            }
            return Err(app_err);
        }
        Err(interact_err) => {
            error!(
                "Deadpool interact error during transaction: {:?}",
                interact_err
            );
            return Err(AppError::InteractError(interact_err));
        }
    };

    info!(
        "Submission processed for exercise {}, player {}, game {}. Was first correct: {}",
        payload.exercise_id, payload.player_id, payload.game_id, was_first_correct_submission
    );
    Ok(ApiResponse::ok(was_first_correct_submission))
}

/// Attempts to insert a player unlock record. Ignores conflicts.
/// Returns NotFound if player or exercise doesn't exist (FK violation).
fn internal_unlock_exercise(
    conn: &mut PgConnection,
    player_id: i64,
    exercise_id: i64,
) -> Result<(), AppError> {
    let new_unlock = NewPlayerUnlock {
        player_id,
        exercise_id,
    };

    let result = diesel::insert_into(pus_dsl::player_unlocks)
        .values(&new_unlock)
        .on_conflict((pus_dsl::player_id, pus_dsl::exercise_id))
        .do_nothing()
        .execute(conn);

    match result {
        Ok(_) => Ok(()),
        Err(DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _)) => {
            error!(
                "Foreign key violation during unlock insert for player {}, exercise {}.",
                player_id, exercise_id
            );
            Err(AppError::NotFound(format!(
                "Player with ID {} or Exercise with ID {} not found.",
                player_id, exercise_id
            )))
        }
        Err(e) => {
            error!(
                "Database error during unlock insert for player {}, exercise {}: {:?}",
                player_id, exercise_id, e
            );
            Err(AppError::DieselError(e))
        }
    }
}

/// Explicitly unlocks (and unhides) an exercise for a player.
/// This operation does nothing if already unlocked.
///
/// Parameters:
/// * player_id as `i64`: The ID of the player.
/// * exercise_id as `i64`: The ID of the exercise to unlock.
///
/// Returns (wrapped in `ApiResponse`)
/// * `None` on success (200).
/// * `None` if the player or exercise does not exist (404).
/// * `None` if failed for other reasons (500).
#[instrument(skip(pool, payload))]
pub async fn unlock(
    State(pool): State<Pool>,
    Json(payload): Json<UnlockPayload>,
) -> Result<ApiResponse<()>, AppError> {
    let player_id = payload.player_id;
    let exercise_id = payload.exercise_id;

    info!(
        "Attempting to unlock exercise {} for player {}",
        exercise_id, player_id
    );

    let conn = pool.get().await.map_err(AppError::PoolError)?;
    let unlock_result = conn
        .interact(move |conn_sync| internal_unlock_exercise(conn_sync, player_id, exercise_id))
        .await;

    match unlock_result {
        Ok(Ok(())) => {
            info!(
                "Successfully processed unlock request for exercise {} player {}",
                exercise_id, player_id
            );
            Ok(ApiResponse::ok(()))
        }
        Ok(Err(app_err)) => Err(app_err),
        Err(interact_err) => Err(AppError::InteractError(interact_err)),
    }
}

/// Retrieves the last relevant submission for a player and exercise.
/// Prioritizes the last correct submission, falls back to the last submission overall.
///
/// Parameters:
/// * player_id as `i64`: The ID of the player.
/// * exercise_id as `i64`: The ID of the exercise.
///
/// Returns (wrapped in `ApiResponse`)
/// * `LastSolutionResponse`: Contains submission data if found, otherwise `None` (200).
/// * `None` if player/exercise not found (404)
/// * `None` if a database error occurs (500).
#[instrument(skip(pool, params))]
pub async fn get_last_solution(
    State(pool): State<Pool>,
    Query(params): Query<GetLastSolutionParams>,
) -> Result<ApiResponse<Option<LastSolutionResponse>>, AppError> {
    let player_id = params.player_id;
    let exercise_id = params.exercise_id;

    info!(
        "Fetching last solution for player_id: {}, exercise_id: {}",
        player_id, exercise_id
    );
    debug!("Get last solution params: {:?}", params);

    let player_exists = helper::run_query(&pool, move |conn| {
        diesel::dsl::select(diesel::dsl::exists(players_dsl::players.find(player_id)))
            .get_result::<bool>(conn)
    })
    .await?;

    if !player_exists {
        error!("Player with ID {} not found.", player_id);
        return Err(AppError::NotFound(format!(
            "Player with ID {} not found.",
            player_id
        )));
    }

    let exercise_exists = helper::run_query(&pool, move |conn| {
        diesel::dsl::select(diesel::dsl::exists(
            exercises_dsl::exercises.find(exercise_id),
        ))
        .get_result::<bool>(conn)
    })
    .await?;

    if !exercise_exists {
        error!("Exercise with ID {} not found.", exercise_id);
        return Err(AppError::NotFound(format!(
            "Exercise with ID {} not found.",
            exercise_id
        )));
    }

    let selection = (
        sub_dsl::submitted_code,
        sub_dsl::metrics,
        sub_dsl::result,
        sub_dsl::result_description,
        sub_dsl::feedback,
        sub_dsl::submitted_at,
    );

    let last_correct_result = helper::run_query(&pool, move |conn| {
        sub_dsl::submissions
            .filter(sub_dsl::player_id.eq(player_id))
            .filter(sub_dsl::exercise_id.eq(exercise_id))
            .filter(sub_dsl::result.gt(BigDecimal::from_f64(0.0).expect("0.0 is valid BigDecimal")))
            .order(sub_dsl::submitted_at.desc())
            .select(selection)
            .first::<LastSolutionResponse>(conn)
    })
    .await;

    match last_correct_result {
        Ok(solution) => {
            info!(
                "Found last correct solution for player {}, exercise {}",
                player_id, exercise_id
            );
            return Ok(ApiResponse::ok(Some(solution)));
        }
        Err(AppError::DieselError(DieselError::NotFound)) => {
            info!(
                "No correct solution found for player {}, exercise {}. Checking for any submission.",
                player_id, exercise_id
            );
        }
        Err(e) => return Err(e),
    }

    let last_any_result = helper::run_query(&pool, move |conn| {
        sub_dsl::submissions
            .filter(sub_dsl::player_id.eq(player_id))
            .filter(sub_dsl::exercise_id.eq(exercise_id))
            .order(sub_dsl::submitted_at.desc())
            .select(selection)
            .first::<LastSolutionResponse>(conn)
    })
    .await;

    match last_any_result {
        Ok(solution) => {
            info!(
                "Found last overall submission for player {}, exercise {}",
                player_id, exercise_id
            );
            Ok(ApiResponse::ok(Some(solution)))
        }
        Err(AppError::DieselError(DieselError::NotFound)) => {
            info!(
                "No submissions found for player {}, exercise {}",
                player_id, exercise_id
            );
            Ok(ApiResponse::ok(None))
        }
        Err(e) => Err(e),
    }
}
