use anyhow::anyhow;
use super::helper;

use crate::{
    errors::AppError,
    payloads::teacher::GetInstructorGamesParams,
    response::ApiResponse,
    schema::{
        game_ownership::dsl as go_dsl,
        instructors::dsl as instructors_dsl,
        games::dsl as games_dsl,
        player_registrations::dsl as pr_dsl,
        players::dsl as players_dsl,
        player_groups::dsl as pg_dsl,
        submissions::dsl as sub_dsl,
        exercises::dsl as exercises_dsl,
        courses::dsl as courses_dsl,
        modules::dsl as modules_dsl,
        groups::dsl as groups_dsl,
        group_ownership::dsl as gro_dsl,
        player_rewards::dsl as prw_dsl,
        player_unlocks::dsl as pu_dsl
    },
};
use axum::{
    extract::{Query, State},
    Json,
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::{count, exists};
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use serde_json::json;
use tracing::{debug, error, info, instrument};
use tracing::log::warn;
use crate::model::student::NewPlayerRegistration;
use crate::model::teacher::{ExerciseStatsResponse, GameChangeset, InstructorGameMetadataResponse, NewGame, NewGameOwnership, NewGroup, NewGroupOwnership, NewPlayer, NewPlayerGroup, StudentExercisesResponse, StudentProgressResponse, SubmissionDataResponse};
use crate::payloads::teacher::{ActivateGamePayload, AddGameInstructorPayload, AddGroupMemberPayload, CreateGamePayload, CreateGroupPayload, CreatePlayerPayload, DeletePlayerPayload, DisablePlayerPayload, DissolveGroupPayload, GetExerciseStatsParams, GetExerciseSubmissionsParams, GetInstructorGameMetadataParams, GetStudentExercisesParams, GetStudentProgressParams, GetStudentSubmissionsParams, GetSubmissionDataParams, ListStudentsParams, ModifyGamePayload, RemoveGameInstructorPayload, RemoveGameStudentPayload, RemoveGroupMemberPayload, StopGamePayload, TranslateEmailParams};

/// Retrieves all game IDs associated with a specific instructor.
///
/// Parameters:
/// * instructor_id as `i64`: The ID of the instructor whose games are to be retrieved.
///
/// Returns (wrapped in `ApiResponse`)
/// * `Vec<i64>`: A list of game IDs if succeeded (200). The list may be empty if the instructor has no associated games.
/// * `None`: If the specified `instructor_id` does not exist (404).
/// * `None`: If a database error occurs during the query (500).
#[instrument(skip(pool, params))]
pub async fn get_instructor_games(
    State(pool): State<Pool>,
    Query(params): Query<GetInstructorGamesParams>,
) -> Result<ApiResponse<Vec<i64>>, AppError> {
    let instructor_id = params.instructor_id;
    info!(
        "Fetching games associated with instructor_id: {}",
        instructor_id
    );
    debug!("Get instructor games params: {:?}", params);

    let instructor_check_result = helper::run_query(&pool, move |conn| {
        instructors_dsl::instructors
            .filter(instructors_dsl::id.eq(instructor_id))
            .select(instructors_dsl::id)
            .first::<i64>(conn)
    }).await;

    match instructor_check_result {
        Ok(_) => info!(
            "Instructor {} found. Fetching associated games...",
            instructor_id
        ),
        Err(AppError::DieselError(DieselError::NotFound)) => {
            error!("Instructor with ID {} not found.", instructor_id);
            return Err(AppError::NotFound(format!(
                "Instructor with ID {} not found.",
                instructor_id
            )));
        }
        Err(e) => return Err(e)
    }

    let game_ids = helper::run_query(&pool, move |conn_sync| {
        go_dsl::game_ownership
            .filter(go_dsl::instructor_id.eq(instructor_id))
            .select(go_dsl::game_id)
            .load::<i64>(conn_sync)
    }).await?;

    info!(
        "Successfully fetched {} game IDs for instructor_id: {}",
        game_ids.len(),
        instructor_id
    );
    Ok(ApiResponse::ok(game_ids))
}

/// Retrieves detailed metadata for a specific game if the instructor has access.
///
/// Access is granted if the instructor is listed in the `game_ownership` table
/// for the given game (regardless of owner status) OR if the instructor's ID is 0 (admin).
/// The endpoint returns game details, ownership status for the requesting instructor,
/// and the total number of players registered for the game.
///
/// Parameters:
/// * instructor_id as `i64`: The ID of the instructor requesting the data.
/// * game_id as `i64`: The ID of the game whose metadata is requested.
///
/// Returns (wrapped in `ApiResponse`)
/// * `InstructorGameMetadataResponse`: Contains game details, ownership, and player count if successful (200).
/// * `None`: If the instructor or game doesn't exist, or if the instructor lacks permission (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, params))]
pub async fn get_instructor_game_metadata(
    State(pool): State<Pool>,
    Query(params): Query<GetInstructorGameMetadataParams>,
) -> Result<ApiResponse<InstructorGameMetadataResponse>, AppError> {
    let instructor_id = params.instructor_id;
    let game_id = params.game_id;

    info!(
        "Fetching metadata for game_id: {} requested by instructor_id: {}",
        game_id, instructor_id
    );
    debug!("Get instructor game metadata params: {:?}", params);

    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        instructor_id, game_id
    );

    type GameDetailsTuple = (
        String,        // title
        String,        // description
        bool,          // active
        bool,          // public
        i32,           // total_exercises
        DateTime<Utc>, // start_date
        DateTime<Utc>, // end_date
    );

    // No need to handle DieselError::NotFound specifically for the game here,
    // as the permission check guarantees its existence if Ok(()) was returned.
    // Still need to handle other potential DieselErrors.
    let (title, description, active, public, total_exercises, start_date, end_date) =
        helper::run_query(&pool, move |conn| {
            games_dsl::games
                .find(game_id)
                .select((
                    games_dsl::title,
                    games_dsl::description,
                    games_dsl::active,
                    games_dsl::public,
                    games_dsl::total_exercises,
                    games_dsl::start_date,
                    games_dsl::end_date,
                )).first::<GameDetailsTuple>(conn)
        }).await?;

    let mut is_owner = false;
    if instructor_id != 0 {
        let owner_status_result = helper::run_query(&pool, move |conn| {
            go_dsl::game_ownership
                .filter(go_dsl::instructor_id.eq(instructor_id))
                .filter(go_dsl::game_id.eq(game_id))
                .select(go_dsl::owner)
                .first::<bool>(conn)
        }).await?;

        is_owner = owner_status_result;
    }

    let player_count = helper::run_query(&pool, move |conn| {
        pr_dsl::player_registrations
            .filter(pr_dsl::game_id.eq(game_id))
            .select(count(pr_dsl::id))
            .first::<i64>(conn)
    }).await?;

    let response_data = InstructorGameMetadataResponse {
        title,
        description,
        active,
        public,
        total_exercises,
        start_date,
        end_date,
        is_owner,
        player_count,
    };

    info!(
        "Successfully fetched metadata for game_id: {} for instructor_id: {}",
        game_id, instructor_id
    );
    Ok(ApiResponse::ok(response_data))
}

/// Lists student IDs participating in a specific game, with optional filters.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
/// Filters students based on their registration in the specified game.
/// Optionally filters further to include only students belonging to a specific group
/// and/or only students who are not marked as disabled.
///
/// Parameters:
/// * instructor_id as `i64`: The ID of the instructor requesting the list.
/// * game_id as `i64`: The ID of the game whose students are to be listed.
/// * group_id as `Option<i64>`: If provided, only students belonging to this group ID are included.
/// * only_active as `bool`: If true, only students with `players.disabled = false` are included. Defaults to false.
///
/// Returns (wrapped in `ApiResponse`)
/// * `Vec<i64>`: A list of `Players.id` matching the criteria (200). Can be empty.
/// * `None`: If the instructor lacks permission for the game, or if the game/group doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, params))]
pub async fn list_students(
    State(pool): State<Pool>,
    Query(params): Query<ListStudentsParams>,
) -> Result<ApiResponse<Vec<i64>>, AppError> {
    let instructor_id = params.instructor_id;
    let game_id = params.game_id;
    let group_id_filter = params.group_id;
    let only_active_filter = params.only_active;

    info!(
        "Listing students for game_id: {} requested by instructor_id: {}. Filters: group_id={:?}, only_active={}",
        game_id, instructor_id, group_id_filter, only_active_filter
    );
    debug!("List students params: {:?}", params);

    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        instructor_id, game_id
    );

    let student_ids = helper::run_query(&pool, move |conn_sync| {
        let mut query = pr_dsl::player_registrations
            .filter(pr_dsl::game_id.eq(game_id))
            .inner_join(players_dsl::players.on(pr_dsl::player_id.eq(players_dsl::id)))
            .inner_join(pg_dsl::player_groups.on(pg_dsl::player_id.eq(players_dsl::id)))
            .into_boxed();

        if only_active_filter {
            info!("Applying filter: only_active = true (players.disabled = false)");
            query = query.filter(players_dsl::disabled.eq(false));
        }

        if let Some(gid) = group_id_filter {
            info!("Applying filter: group_id = {}", gid);
            query = query.filter(pg_dsl::group_id.eq(gid));
        }

        query
            .select(players_dsl::id)
            .load::<i64>(conn_sync)
    }).await?;

    info!(
        "Successfully fetched {} student IDs for game_id: {} with applied filters.",
        student_ids.len(),
        game_id
    );
    Ok(ApiResponse::ok(student_ids))
}

/// Retrieves progress metrics for a specific student within a specific game.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
/// Calculates the total number of submissions (attempts), the number of exercises solved
/// for the first time, and the progress percentage based on the game's total exercises.
///
/// Parameters:
/// * instructor_id as `i64`: The ID of the instructor requesting the data.
/// * game_id as `i64`: The ID of the game context.
/// * player_id as `i64`: The ID of the student whose progress is requested.
///
/// Returns (wrapped in `ApiResponse`)
/// * `StudentProgressResponse`: Contains attempts, solved count, and progress percentage (200).
/// * `None`: If instructor lacks permission, or if game/player/registration doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, params))]
pub async fn get_student_progress(
    State(pool): State<Pool>,
    Query(params): Query<GetStudentProgressParams>,
) -> Result<ApiResponse<StudentProgressResponse>, AppError> {
    let instructor_id = params.instructor_id;
    let game_id = params.game_id;
    let player_id = params.player_id;

    info!(
        "Fetching progress for player_id: {} in game_id: {} requested by instructor_id: {}",
        player_id, game_id, instructor_id
    );
    debug!("Get student progress params: {:?}", params);

    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        instructor_id, game_id
    );

    let is_registered = helper::run_query(&pool, move |conn| {
        diesel::select(exists(
            pr_dsl::player_registrations
                .filter(pr_dsl::player_id.eq(player_id))
                .filter(pr_dsl::game_id.eq(game_id)),
        )).get_result::<bool>(conn)
    }).await?;

    if !is_registered {
        warn!(
            "Player {} is not registered in game {}. Cannot fetch progress.",
            player_id, game_id
        );
        return Err(AppError::NotFound(format!(
            "Player with ID {} is not registered in game with ID {}.",
            player_id, game_id
        )));
    }
    info!("Player {} confirmed registered in game {}.", player_id, game_id);

    let game_total_exercises = helper::run_query(&pool, move |conn| {
        games_dsl::games
            .find(game_id)
            .select(games_dsl::total_exercises)
            .first::<i32>(conn)
    }).await?;

    let total_attempts = helper::run_query(&pool, move |conn| {
        sub_dsl::submissions
            .filter(sub_dsl::player_id.eq(player_id))
            .filter(sub_dsl::game_id.eq(game_id))
            .count()
            .get_result::<i64>(conn)
    }).await?;

    let solved_exercises_count = helper::run_query(&pool, move |conn| {
        sub_dsl::submissions
            .filter(sub_dsl::player_id.eq(player_id))
            .filter(sub_dsl::game_id.eq(game_id))
            .filter(sub_dsl::first_solution.eq(true))
            .count()
            .get_result::<i64>(conn)
    }).await?;

    let progress_percentage = if game_total_exercises > 0 {
        (solved_exercises_count as f64 / game_total_exercises as f64) * 100.0
    } else {
        warn!(
            "Game {} has total_exercises <= 0. Setting progress to 0.0.",
            game_id
        );
        0.0
    };

    let response_data = StudentProgressResponse {
        attempts: total_attempts,
        solved_exercises: solved_exercises_count,
        progress: progress_percentage,
    };

    info!(
        "Successfully fetched progress for player_id: {} in game_id: {}. Attempts: {}, Solved: {}, Progress: {:.2}%",
        player_id, game_id, total_attempts, solved_exercises_count, progress_percentage
    );
    Ok(ApiResponse::ok(response_data))
}

/// Retrieves lists of attempted and solved exercise IDs for a specific student within a game.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
/// Finds all unique exercises the student has submitted to (attempted) and all unique exercises
/// where the student's submission was marked as the first correct solution (solved).
///
/// Parameters:
/// * instructor_id as `i64`: The ID of the instructor requesting the data.
/// * game_id as `i64`: The ID of the game context.
/// * player_id as `i64`: The ID of the student whose exercise lists are requested.
///
/// Returns (wrapped in `ApiResponse`)
/// * `StudentExercisesResponse`: Contains lists of attempted and solved exercise IDs (200). Lists can be empty.
/// * `None`: If instructor lacks permission, or if game/player/registration doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, params))]
pub async fn get_student_exercises(
    State(pool): State<Pool>,
    Query(params): Query<GetStudentExercisesParams>,
) -> Result<ApiResponse<StudentExercisesResponse>, AppError> {
    let instructor_id = params.instructor_id;
    let game_id = params.game_id;
    let player_id = params.player_id;

    info!(
        "Fetching exercise lists for player_id: {} in game_id: {} requested by instructor_id: {}",
        player_id, game_id, instructor_id
    );
    debug!("Get student exercises params: {:?}", params);

    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        instructor_id, game_id
    );

    let is_registered = helper::run_query(&pool, move |conn| {
        diesel::select(exists(
            pr_dsl::player_registrations
                .filter(pr_dsl::player_id.eq(player_id))
                .filter(pr_dsl::game_id.eq(game_id)),
        ))
            .get_result::<bool>(conn)
    }).await?;

    if !is_registered {
        warn!(
            "Player {} is not registered in game {}. Cannot fetch exercise lists.",
            player_id, game_id
        );
        return Err(AppError::NotFound(format!(
            "Player with ID {} is not registered in game with ID {}.",
            player_id, game_id
        )));
    }
    info!("Player {} confirmed registered in game {}.", player_id, game_id);

    let attempted_exercises_list = helper::run_query(&pool, move |conn| {
        sub_dsl::submissions
            .filter(sub_dsl::player_id.eq(player_id))
            .filter(sub_dsl::game_id.eq(game_id))
            .select(sub_dsl::exercise_id)
            .distinct()
            .load::<i64>(conn)
    }).await?;

    let solved_exercises_list = helper::run_query(&pool, move |conn| {
        sub_dsl::submissions
            .filter(sub_dsl::player_id.eq(player_id))
            .filter(sub_dsl::game_id.eq(game_id))
            .filter(sub_dsl::first_solution.eq(true))
            .select(sub_dsl::exercise_id)
            .distinct()
            .load::<i64>(conn)
    }).await?;

    let response_data = StudentExercisesResponse {
        attempted_exercises: attempted_exercises_list,
        solved_exercises: solved_exercises_list,
    };

    info!(
        "Successfully fetched exercise lists for player_id: {} in game_id: {}. Attempted: {}, Solved: {}",
        player_id, game_id, response_data.attempted_exercises.len(), response_data.solved_exercises.len()
    );
    Ok(ApiResponse::ok(response_data))
}

/// Retrieves a list of submission IDs for a specific student within a game, with optional success filter.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
/// Fetches submission IDs for the specified player and game. Optionally filters to include
/// only submissions considered successful (result >= 50).
///
/// Parameters:
/// * instructor_id as `i64`: The ID of the instructor requesting the data.
/// * game_id as `i64`: The ID of the game context.
/// * player_id as `i64`: The ID of the student whose submissions are requested.
/// * success_only as `bool`: If true, only submissions with `result >= 50` are included. Defaults to false.
///
/// Returns (wrapped in `ApiResponse`)
/// * `Vec<i64>`: A list of `Submissions.id` matching the criteria (200). Can be empty.
/// * `None`: If instructor lacks permission, or if game/player/registration doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, params))]
pub async fn get_student_submissions(
    State(pool): State<Pool>,
    Query(params): Query<GetStudentSubmissionsParams>,
) -> Result<ApiResponse<Vec<i64>>, AppError> {
    let instructor_id = params.instructor_id;
    let game_id = params.game_id;
    let player_id = params.player_id;
    let success_only_filter = params.success_only;

    info!(
        "Fetching submissions for player_id: {} in game_id: {} requested by instructor_id: {}. Filter: success_only={}",
        player_id, game_id, instructor_id, success_only_filter
    );
    debug!("Get student submissions params: {:?}", params);

    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        instructor_id, game_id
    );

    let is_registered = helper::run_query(&pool, move |conn| {
        diesel::select(exists(
            pr_dsl::player_registrations
                .filter(pr_dsl::player_id.eq(player_id))
                .filter(pr_dsl::game_id.eq(game_id)),
        ))
            .get_result::<bool>(conn)
    })
        .await?;

    if !is_registered {
        warn!(
            "Player {} is not registered in game {}. Cannot fetch submissions.",
            player_id, game_id
        );
        return Err(AppError::NotFound(format!(
            "Player with ID {} is not registered in game with ID {}.",
            player_id, game_id
        )));
    }
    info!("Player {} confirmed registered in game {}.", player_id, game_id);

    let submission_ids = helper::run_query(&pool, move |conn_sync| {
        let mut query = sub_dsl::submissions
            .filter(sub_dsl::player_id.eq(player_id))
            .filter(sub_dsl::game_id.eq(game_id))
            .into_boxed();

        if success_only_filter {
            info!("Applying filter: success_only = true (result >= 50)");
            let success_threshold = BigDecimal::from(50);
            query = query.filter(sub_dsl::result.ge(success_threshold));
        }

        query
            .select(sub_dsl::id)
            .order(sub_dsl::submitted_at.desc()) // Optional: Order by submission time (newest first)
            .load::<i64>(conn_sync)
    }).await?;

    info!(
        "Successfully fetched {} submission IDs for player_id: {} in game_id: {} with applied filters.",
        submission_ids.len(),
        player_id,
        game_id
    );
    Ok(ApiResponse::ok(submission_ids))
}

/// Retrieves the full data for a specific submission.
///
/// Requires the requesting instructor to have permission for the game associated
/// with the submission (admin or listed in game_ownership).
///
/// Parameters:
/// * instructor_id as `i64`: The ID of the instructor requesting the data.
/// * submission_id as `i64`: The ID of the submission to retrieve.
///
/// Returns (wrapped in `ApiResponse`)
/// * `SubmissionDataResponse`: Contains all fields of the submission record (200).
/// * `None`: If the submission is not found, or the instructor lacks permission for the associated game (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, params))]
pub async fn get_submission_data(
    State(pool): State<Pool>,
    Query(params): Query<GetSubmissionDataParams>,
) -> Result<ApiResponse<SubmissionDataResponse>, AppError> {
    let instructor_id = params.instructor_id;
    let submission_id = params.submission_id;

    info!(
        "Fetching data for submission_id: {} requested by instructor_id: {}",
        submission_id, instructor_id
    );
    debug!("Get submission data params: {:?}", params);

    let submission_data_result = helper::run_query(&pool, move |conn| {
        sub_dsl::submissions
            .find(submission_id)
            .first::<SubmissionDataResponse>(conn)
    }).await;

    let submission_data = match submission_data_result {
        Ok(data) => data,
        Err(AppError::DieselError(DieselError::NotFound)) => {
            error!("Submission with ID {} not found.", submission_id);
            return Err(AppError::NotFound(format!(
                "Submission with ID {} not found.",
                submission_id
            )));
        }
        Err(e) => return Err(e),
    };

    let game_id = submission_data.game_id;
    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {} (associated with submission {})",
        instructor_id, game_id, submission_id
    );

    info!(
        "Successfully fetched data for submission_id: {}",
        submission_id
    );
    Ok(ApiResponse::ok(submission_data))
}

/// Retrieves statistics for a specific exercise within a game.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
/// Calculates total attempts, successful attempts, difficulty (based on success rate),
/// and the percentage of registered players in the game who have solved the exercise.
///
/// Parameters:
/// * instructor_id as `i64`: The ID of the instructor requesting the data.
/// * game_id as `i64`: The ID of the game context.
/// * exercise_id as `i64`: The ID of the exercise to analyze.
///
/// Returns (wrapped in `ApiResponse`)
/// * `ExerciseStatsResponse`: Contains calculated statistics for the exercise (200).
/// * `None`: If instructor lacks permission, or if game/exercise doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, params))]
pub async fn get_exercise_stats(
    State(pool): State<Pool>,
    Query(params): Query<GetExerciseStatsParams>,
) -> Result<ApiResponse<ExerciseStatsResponse>, AppError> {
    let instructor_id = params.instructor_id;
    let game_id = params.game_id;
    let exercise_id = params.exercise_id;

    info!(
        "Fetching stats for exercise_id: {} in game_id: {} requested by instructor_id: {}",
        exercise_id, game_id, instructor_id
    );
    debug!("Get exercise stats params: {:?}", params);

    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        instructor_id, game_id
    );

    let exercise_exists = helper::run_query(&pool, move |conn| {
        diesel::select(exists(exercises_dsl::exercises.find(exercise_id)))
            .get_result::<bool>(conn)
    })
        .await?;

    if !exercise_exists {
        error!(
            "Cannot get stats: Exercise with ID {} not found.",
            exercise_id
        );
        return Err(AppError::NotFound(format!(
            "Exercise with ID {} not found.",
            exercise_id
        )));
    }
    info!("Exercise {} confirmed to exist.", exercise_id);

    let success_threshold = BigDecimal::from(50);

    let total_attempts = helper::run_query(&pool, {
        let game_id = game_id;
        let exercise_id = exercise_id;
        move |conn| {
            sub_dsl::submissions
                .filter(sub_dsl::game_id.eq(game_id))
                .filter(sub_dsl::exercise_id.eq(exercise_id))
                .count()
                .get_result::<i64>(conn)
        }
    }).await?;

    let successful_attempts = helper::run_query(&pool, {
        let game_id = game_id;
        let exercise_id = exercise_id;
        let success_threshold = success_threshold.clone();
        move |conn| {
            sub_dsl::submissions
                .filter(sub_dsl::game_id.eq(game_id))
                .filter(sub_dsl::exercise_id.eq(exercise_id))
                .filter(sub_dsl::result.ge(success_threshold))
                .count()
                .get_result::<i64>(conn)
        }
    }).await?;

    let first_solutions_count = helper::run_query(&pool, {
        let game_id = game_id;
        let exercise_id = exercise_id;
        move |conn| {
            sub_dsl::submissions
                .filter(sub_dsl::game_id.eq(game_id))
                .filter(sub_dsl::exercise_id.eq(exercise_id))
                .filter(sub_dsl::first_solution.eq(true))
                .count()
                .get_result::<i64>(conn)
        }
    }).await?;

    let total_players_in_game = helper::run_query(&pool, {
        let game_id = game_id;
        move |conn| {
            pr_dsl::player_registrations
                .filter(pr_dsl::game_id.eq(game_id))
                .count()
                .get_result::<i64>(conn)
        }
    }).await?;

    let difficulty = if total_attempts > 0 {
        100.0 - (successful_attempts as f64 / total_attempts as f64 * 100.0)
    } else {
        warn!(
            "No attempts found for exercise {} in game {}. Setting difficulty to 0.0.",
            exercise_id, game_id
        );
        0.0
    };

    let solved_percentage = if total_players_in_game > 0 {
        first_solutions_count as f64 / total_players_in_game as f64 * 100.0
    } else {
        warn!(
            "No players registered in game {}. Setting solved_percentage to 0.0.",
            game_id
        );
        0.0
    };

    let response_data = ExerciseStatsResponse {
        attempts: total_attempts,
        successful_attempts,
        difficulty,
        solved_percentage,
    };

    info!(
        "Successfully fetched stats for exercise_id: {} in game_id: {}. Attempts: {}, Success: {}, Difficulty: {:.2}, Solved%: {:.2}",
        exercise_id, game_id, total_attempts, successful_attempts, difficulty, solved_percentage
    );
    Ok(ApiResponse::ok(response_data))
}

/// Retrieves a list of submission IDs for a specific exercise within a game, with optional success filter.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
/// Fetches submission IDs for the specified exercise and game. Optionally filters to include
/// only submissions considered successful (result >= 50).
///
/// Parameters:
/// * instructor_id as `i64`: The ID of the instructor requesting the data.
/// * game_id as `i64`: The ID of the game context.
/// * exercise_id as `i64`: The ID of the exercise whose submissions are requested.
/// * success_only as `bool`: If true, only submissions with `result >= 50` are included. Defaults to false.
///
/// Returns (wrapped in `ApiResponse`)
/// * `Vec<i64>`: A list of `Submissions.id` matching the criteria (200). Can be empty.
/// * `None`: If instructor lacks permission, or if game/exercise doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, params))]
pub async fn get_exercise_submissions(
    State(pool): State<Pool>,
    Query(params): Query<GetExerciseSubmissionsParams>,
) -> Result<ApiResponse<Vec<i64>>, AppError> {
    let instructor_id = params.instructor_id;
    let game_id = params.game_id;
    let exercise_id = params.exercise_id;
    let success_only_filter = params.success_only;

    info!(
        "Fetching submissions for exercise_id: {} in game_id: {} requested by instructor_id: {}. Filter: success_only={}",
        exercise_id, game_id, instructor_id, success_only_filter
    );
    debug!("Get exercise submissions params: {:?}", params);

    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        instructor_id, game_id
    );

    let exercise_exists = helper::run_query(&pool, move |conn| {
        diesel::select(exists(exercises_dsl::exercises.find(exercise_id)))
            .get_result::<bool>(conn)
    }).await?;

    if !exercise_exists {
        error!(
            "Cannot get submissions: Exercise with ID {} not found.",
            exercise_id
        );
        return Err(AppError::NotFound(format!(
            "Exercise with ID {} not found.",
            exercise_id
        )));
    }
    info!("Exercise {} confirmed to exist.", exercise_id);

    let submission_ids = helper::run_query(&pool, move |conn_sync| {
        let mut query = sub_dsl::submissions
            .filter(sub_dsl::game_id.eq(game_id))
            .filter(sub_dsl::exercise_id.eq(exercise_id))
            .into_boxed();

        if success_only_filter {
            info!("Applying filter: success_only = true (result >= 50)");
            let success_threshold = BigDecimal::from(50);
            query = query.filter(sub_dsl::result.ge(success_threshold));
        }

        query
            .select(sub_dsl::id)
            .order(sub_dsl::submitted_at.desc())
            .load::<i64>(conn_sync)
    }).await?;

    info!(
        "Successfully fetched {} submission IDs for exercise_id: {} in game_id: {} with applied filters.",
        submission_ids.len(),
        exercise_id,
        game_id
    );
    Ok(ApiResponse::ok(submission_ids))
}

/// Creates a new game and assigns ownership to the requesting instructor.
///
/// Validates instructor existence, course existence, and programming language validity.
/// Calculates the total number of exercises for the chosen course and language.
/// Performs the creation within a database transaction.
///
/// Request Body: `CreateGamePayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `i64`: The ID of the newly created game (200).
/// * `None`: If instructor/course not found, or language invalid (404/400).
/// * `None`: If a database error or transaction failure occurs (500).
#[instrument(skip(pool, payload))]
pub async fn create_game(
    State(pool): State<Pool>,
    Json(payload): Json<CreateGamePayload>,
) -> Result<ApiResponse<i64>, AppError> {
    info!(
        "Attempting to create game '{}' for course {} by instructor {}",
        payload.title, payload.course_id, payload.instructor_id
    );
    debug!("Create game payload: {:?}", payload);

    let instructor_exists = helper::run_query(&pool, {
        let instructor_id = payload.instructor_id;
        move |conn| {
            diesel::select(exists(instructors_dsl::instructors.find(instructor_id)))
                .get_result::<bool>(conn)
        }
    })
        .await?;

    if !instructor_exists {
        error!(
            "Cannot create game: Instructor with ID {} not found.",
            payload.instructor_id
        );
        return Err(AppError::NotFound(format!(
            "Instructor with ID {} not found.",
            payload.instructor_id
        )));
    }
    info!("Instructor {} confirmed to exist.", payload.instructor_id);

    let course_languages = helper::run_query(&pool, {
        let course_id = payload.course_id;
        move |conn| {
            courses_dsl::courses
                .find(course_id)
                .select(courses_dsl::programming_languages)
                .first::<String>(conn)
        }
    }).await;

    let allowed_languages_str = match course_languages {
        Ok(langs) => langs,
        Err(AppError::DieselError(DieselError::NotFound)) => {
            error!(
                "Cannot create game: Course with ID {} not found.",
                payload.course_id
            );
            return Err(AppError::NotFound(format!(
                "Course with ID {} not found.",
                payload.course_id
            )));
        }
        Err(e) => return Err(e),
    };

    let allowed_languages: Vec<&str> = allowed_languages_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if !allowed_languages.contains(&payload.programming_language.as_str()) {
        warn!(
            "Invalid programming language '{}' for course {}. Allowed: {:?}",
            payload.programming_language, payload.course_id, allowed_languages
        );
        return Err(AppError::Internal(anyhow!(
            "Programming language '{}' is not allowed for course {}. Allowed: {:?}",
            payload.programming_language, payload.course_id, allowed_languages
        )).into());
    }
    info!(
        "Programming language '{}' validated for course {}.",
        payload.programming_language, payload.course_id
    );

    let total_exercises_count = helper::run_query(&pool, {
        let course_id = payload.course_id;
        let language = payload.programming_language.clone();
        move |conn| {
            exercises_dsl::exercises
                .inner_join(modules_dsl::modules.on(exercises_dsl::module_id.eq(modules_dsl::id)))
                .filter(modules_dsl::course_id.eq(course_id))
                .filter(exercises_dsl::programming_language.eq(language))
                .count()
                .get_result::<i64>(conn)
        }
    }).await?;
    info!(
        "Calculated {} total exercises for course {} and language {}.",
        total_exercises_count, payload.course_id, payload.programming_language
    );

    let conn = pool.get().await.map_err(AppError::PoolError)?;
    let creation_result = conn
        .interact(move |conn_sync| {
            conn_sync.transaction(|transaction_conn| {
                let now = Utc::now();
                let new_game = NewGame {
                    title: payload.title,
                    public: payload.public,
                    active: payload.active,
                    description: payload.description,
                    course_id: payload.course_id,
                    programming_language: payload.programming_language,
                    module_lock: payload.module_lock,
                    exercise_lock: payload.exercise_lock,
                    total_exercises: total_exercises_count as i32,
                    start_date: now,
                    end_date: now + Duration::days(365),
                };

                let inserted_game_id = diesel::insert_into(games_dsl::games)
                    .values(&new_game)
                    .returning(games_dsl::id)
                    .get_result::<i64>(transaction_conn)?;

                let new_ownership = NewGameOwnership {
                    game_id: inserted_game_id,
                    instructor_id: payload.instructor_id,
                    owner: true,
                };

                diesel::insert_into(go_dsl::game_ownership)
                    .values(&new_ownership)
                    .execute(transaction_conn)?;

                Ok(inserted_game_id)
            })
        }).await;

    match creation_result {
        Ok(Ok(new_game_id)) => {
            info!(
                "Successfully created game with ID: {} and assigned ownership to instructor {}",
                new_game_id, payload.instructor_id
            );
            Ok(ApiResponse::ok(new_game_id))
        }
        Ok(Err(diesel_err)) => {
            error!("Game creation transaction failed: {:?}", diesel_err);
            Err(AppError::DieselError(diesel_err))
        }
        Err(interact_err) => {
            error!("Interaction error during game creation: {:?}", interact_err);
            Err(AppError::InteractError(interact_err))
        }
    }
}

/// Modifies settings of an existing game.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
/// Allows updating title, public status, active status, description, module lock, and exercise lock.
/// Course and programming language cannot be changed.
/// Fields omitted in the request body will not be updated.
///
/// Request Body: `ModifyGamePayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the update was successful (200).
/// * `None`: If instructor lacks permission, or the game doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, payload))]
pub async fn modify_game(
    State(pool): State<Pool>,
    Json(payload): Json<ModifyGamePayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let instructor_id = payload.instructor_id;
    let game_id = payload.game_id;

    info!(
        "Attempting to modify game_id: {} requested by instructor_id: {}",
        game_id, instructor_id
    );
    debug!("Modify game payload: {:?}", payload);

    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        instructor_id, game_id
    );

    let changeset = GameChangeset {
        title: payload.title,
        public: payload.public,
        active: payload.active,
        description: payload.description,
        module_lock: payload.module_lock,
        exercise_lock: payload.exercise_lock,
        updated_at: Some(Utc::now()),
    };

    let has_updates = changeset.title.is_some()
        || changeset.public.is_some()
        || changeset.active.is_some()
        || changeset.description.is_some()
        || changeset.module_lock.is_some()
        || changeset.exercise_lock.is_some();

    if !has_updates {
        info!("No update fields provided for game {}. Returning success.", game_id);
        return Ok(ApiResponse::ok(true));
    }

    let update_result = helper::run_query(&pool, move |conn| {
        diesel::update(games_dsl::games.find(game_id))
            .set(&changeset)
            .execute(conn)
    }).await;

    match update_result {
        Ok(rows_affected) => {
            if rows_affected == 1 {
                info!("Successfully modified game {}", game_id);
                Ok(ApiResponse::ok(true))
            } else if rows_affected == 0 {
                error!(
                    "Game {} modification failed: 0 rows affected (unexpected state after permission check).",
                    game_id
                );
                Err(AppError::Internal(anyhow!(
                    "Game modification failed unexpectedly (0 rows affected)."
                )))
            } else {
                error!(
                    "Game {} modification failed: {} rows affected (unexpected state).",
                    game_id, rows_affected
                );
                Err(AppError::Internal(anyhow!(
                    "Game modification failed unexpectedly (multiple rows affected)."
                )))
            }
        }
        Err(e) => {
            error!("Database error during game modification: {:?}", e);
            Err(e)
        }
    }
}

/// Adds an instructor to a game's ownership list or updates their owner status.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
/// Validates that the instructor being added exists.
/// If the instructor is already associated with the game, their 'owner' status will be updated.
///
/// Request Body: `AddGameInstructorPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the instructor was successfully added or updated (200).
/// * `None`: If requesting instructor lacks permission, or game/instructor_to_add doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, payload))]
pub async fn add_game_instructor(
    State(pool): State<Pool>,
    Json(payload): Json<AddGameInstructorPayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let requesting_instructor_id = payload.requesting_instructor_id;
    let game_id = payload.game_id;
    let instructor_to_add_id = payload.instructor_to_add_id;
    let is_owner = payload.is_owner;

    info!(
        "Attempting to add instructor {} to game {} (owner={}) requested by instructor {}",
        instructor_to_add_id, game_id, is_owner, requesting_instructor_id
    );
    debug!("Add game instructor payload: {:?}", payload);

    helper::check_instructor_game_permission(&pool, requesting_instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        requesting_instructor_id, game_id
    );

    let instructor_to_add_exists = helper::run_query(&pool, {
        let instructor_to_add_id = instructor_to_add_id;
        move |conn| {
            diesel::select(exists(
                instructors_dsl::instructors.find(instructor_to_add_id),
            )).get_result::<bool>(conn)
        }
    }).await?;

    if !instructor_to_add_exists {
        error!(
            "Cannot add instructor: Instructor with ID {} not found.",
            instructor_to_add_id
        );
        return Err(AppError::NotFound(format!(
            "Instructor with ID {} not found.",
            instructor_to_add_id
        )));
    }
    info!(
        "Instructor to add (ID {}) confirmed to exist.",
        instructor_to_add_id
    );

    let operation_result = helper::run_query(&pool, move |conn| {
        let new_ownership = NewGameOwnership {
            game_id,
            instructor_id: instructor_to_add_id,
            owner: is_owner,
        };

        diesel::insert_into(go_dsl::game_ownership)
            .values(&new_ownership)
            .on_conflict((go_dsl::game_id, go_dsl::instructor_id))
            .do_update()
            .set(go_dsl::owner.eq(is_owner))
            .execute(conn)
    }).await;

    match operation_result {
        Ok(rows_affected) => {
            info!(
                "Successfully added/updated instructor {} for game {}. Owner set to: {}. Rows affected: {}",
                instructor_to_add_id, game_id, is_owner, rows_affected
            );
            Ok(ApiResponse::ok(true))
        }
        Err(e @ AppError::DieselError(DieselError::DatabaseError(
                                          DatabaseErrorKind::ForeignKeyViolation,
                                          _,
                                      ))) => {
            error!("Database constraint violation during instructor addition: {:?}", e);
            Err(AppError::NotFound(
                "Game or Instructor not found (foreign key violation).".to_string(),
            ))
        }
        Err(e) => Err(e)
    }
}

/// Removes an instructor's association (ownership record) from a game.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
/// Attempts to delete the specific `game_ownership` record linking the game and the instructor to be removed.
///
/// Request Body: `RemoveGameInstructorPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the instructor's association was successfully removed (200).
/// * `None`: If requesting instructor lacks permission, or the game doesn't exist (404).
/// * `None`: If the instructor was not associated with the game (target record not found) (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, payload))]
pub async fn remove_game_instructor(
    State(pool): State<Pool>,
    Json(payload): Json<RemoveGameInstructorPayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let requesting_instructor_id = payload.requesting_instructor_id;
    let game_id = payload.game_id;
    let instructor_to_remove_id = payload.instructor_to_remove_id;

    info!(
        "Attempting to remove instructor {} from game {} requested by instructor {}",
        instructor_to_remove_id, game_id, requesting_instructor_id
    );
    debug!("Remove game instructor payload: {:?}", payload);

    helper::check_instructor_game_permission(&pool, requesting_instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        requesting_instructor_id, game_id
    );

    let delete_result = helper::run_query(&pool, move |conn| {
        diesel::delete(
            go_dsl::game_ownership
                .filter(go_dsl::game_id.eq(game_id))
                .filter(go_dsl::instructor_id.eq(instructor_to_remove_id)),
        ).execute(conn)
    }).await;

    match delete_result {
        Ok(rows_affected) => {
            if rows_affected == 1 {
                info!(
                    "Successfully removed instructor {} from game {}",
                    instructor_to_remove_id, game_id
                );
                Ok(ApiResponse::ok(true))
            } else if rows_affected == 0 {
                warn!(
                    "Instructor {} was not associated with game {}. No record removed.",
                    instructor_to_remove_id, game_id
                );
                Err(AppError::NotFound(format!(
                    "Instructor {} is not associated with game {}.",
                    instructor_to_remove_id, game_id
                )))
            } else {
                error!(
                    "Unexpected number of rows ({}) deleted when removing instructor {} from game {}",
                    rows_affected, instructor_to_remove_id, game_id
                );
                Err(AppError::Internal(anyhow!(
                    "Unexpected error during instructor removal."
                )))
            }
        }
        Err(e) => {
            error!(
                "Database error during instructor removal for instructor {} game {}: {:?}",
                instructor_to_remove_id, game_id, e
            );
            Err(e)
        }
    }
}

/// Activates a specific game by setting its 'active' status to true.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
///
/// Request Body: `ActivateGamePayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the game was successfully activated (200).
/// * `None`: If instructor lacks permission, or the game doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, payload))]
pub async fn activate_game(
    State(pool): State<Pool>,
    Json(payload): Json<ActivateGamePayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let instructor_id = payload.instructor_id;
    let game_id = payload.game_id;

    info!(
        "Attempting to activate game_id: {} requested by instructor_id: {}",
        game_id, instructor_id
    );
    debug!("Activate game payload: {:?}", payload);

    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        instructor_id, game_id
    );

    let update_result = helper::run_query(&pool, move |conn| {
        diesel::update(games_dsl::games.find(game_id))
            .set((
                games_dsl::active.eq(true),
                games_dsl::updated_at.eq(diesel::dsl::now),
            ))
            .execute(conn)
    }).await;

    match update_result {
        Ok(rows_affected) => {
            if rows_affected == 1 {
                info!("Successfully activated game {}", game_id);
                Ok(ApiResponse::ok(true))
            } else if rows_affected == 0 {
                error!(
                    "Game {} activation failed: 0 rows affected (unexpected state).",
                    game_id
                );
                Err(AppError::Internal(anyhow!(
                    "Game activation failed unexpectedly (0 rows affected)."
                )))
            } else {
                error!(
                    "Game {} activation failed: {} rows affected (unexpected state).",
                    game_id, rows_affected
                );
                Err(AppError::Internal(anyhow!(
                    "Game activation failed unexpectedly (multiple rows affected)."
                )))
            }
        }
        Err(e) => Err(e)
    }
}

/// Deactivates a specific game by setting its 'active' status to false.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
///
/// Request Body: `StopGamePayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the game was successfully deactivated (200).
/// * `None`: If instructor lacks permission, or the game doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, payload))]
pub async fn stop_game(
    State(pool): State<Pool>,
    Json(payload): Json<StopGamePayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let instructor_id = payload.instructor_id;
    let game_id = payload.game_id;

    info!(
        "Attempting to stop (deactivate) game_id: {} requested by instructor_id: {}",
        game_id, instructor_id
    );
    debug!("Stop game payload: {:?}", payload);

    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        instructor_id, game_id
    );

    let update_result = helper::run_query(&pool, move |conn| {
        diesel::update(games_dsl::games.find(game_id))
            .set((
                games_dsl::active.eq(false),
                games_dsl::updated_at.eq(diesel::dsl::now),
            ))
            .execute(conn)
    }).await;

    match update_result {
        Ok(rows_affected) => {
            if rows_affected == 1 {
                info!("Successfully stopped (deactivated) game {}", game_id);
                Ok(ApiResponse::ok(true))
            } else if rows_affected == 0 {
                error!(
                    "Game {} deactivation failed: 0 rows affected (unexpected state).",
                    game_id
                );
                Err(AppError::Internal(anyhow!(
                    "Game deactivation failed unexpectedly (0 rows affected)."
                )))
            } else {
                error!(
                    "Game {} deactivation failed: {} rows affected (unexpected state).",
                    game_id, rows_affected
                );
                Err(AppError::Internal(anyhow!(
                    "Game deactivation failed unexpectedly (multiple rows affected)."
                )))
            }
        }
        Err(e) => Err(e)
    }
}

/// Removes a student's registration from a specific game.
///
/// Requires the requesting instructor to have permission for the game (admin or listed in game_ownership).
/// Attempts to delete the specific `player_registrations` record linking the game and the student.
///
/// Request Body: `RemoveGameStudentPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the student's registration was successfully removed (200).
/// * `None`: If instructor lacks permission, or the game doesn't exist (404).
/// * `None`: If the student was not registered in the game (target record not found) (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, payload))]
pub async fn remove_game_student(
    State(pool): State<Pool>,
    Json(payload): Json<RemoveGameStudentPayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let instructor_id = payload.instructor_id;
    let game_id = payload.game_id;
    let student_id = payload.student_id;

    info!(
        "Attempting to remove student {} from game {} requested by instructor {}",
        student_id, game_id, instructor_id
    );
    debug!("Remove game student payload: {:?}", payload);

    helper::check_instructor_game_permission(&pool, instructor_id, game_id).await?;
    info!(
        "Permission check passed for instructor {} on game {}",
        instructor_id, game_id
    );

    let delete_result = helper::run_query(&pool, move |conn| {
        diesel::delete(
            pr_dsl::player_registrations
                .filter(pr_dsl::game_id.eq(game_id))
                .filter(pr_dsl::player_id.eq(student_id)),
        ).execute(conn)
    }).await;

    match delete_result {
        Ok(rows_affected) => {
            if rows_affected == 1 {
                info!(
                    "Successfully removed student {} from game {}",
                    student_id, game_id
                );
                Ok(ApiResponse::ok(true))
            } else if rows_affected == 0 {
                warn!(
                    "Student {} was not registered in game {}. No record removed.",
                    student_id, game_id
                );
                Err(AppError::NotFound(format!(
                    "Student {} is not registered in game {}.",
                    student_id, game_id
                )))
            } else {
                error!(
                    "Unexpected number of rows ({}) deleted when removing student {} from game {}",
                    rows_affected, student_id, game_id
                );
                Err(AppError::Internal(anyhow!(
                    "Unexpected error during student removal."
                )))
            }
        }
        Err(e) => Err(e)
    }
}

/// Finds the player ID associated with a given email address.
///
/// Searches the players table for a matching email and returns the corresponding ID.
/// This endpoint is protected by the general teacher authentication layer.
///
/// Query Parameters:
/// * email as `String`: The email address to look up.
///
/// Returns (wrapped in `ApiResponse`)
/// * `i64`: The `Players.id` if found (200).
/// * `None`: If no player with the given email exists (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, params))]
pub async fn translate_email_to_player_id(
    State(pool): State<Pool>,
    Query(params): Query<TranslateEmailParams>,
) -> Result<ApiResponse<i64>, AppError> {
    let email_to_find = params.email;
    let email_cloned = email_to_find.clone();

    info!("Attempting to find player ID for email: {}", &email_to_find);
    debug!("Translate email params: {:?}", &email_to_find);

    let find_result = helper::run_query(&pool, move |conn| {
        players_dsl::players
            .filter(players_dsl::email.eq(email_cloned))
            .select(players_dsl::id)
            .first::<i64>(conn)
    }).await;

    match find_result {
        Ok(player_id) => {
            info!(
                "Successfully found player ID {} for email {}",
                player_id, &email_to_find
            );
            Ok(ApiResponse::ok(player_id))
        }
        Err(AppError::DieselError(DieselError::NotFound)) => {
            warn!("No player found with email: {}", &email_to_find);
            Err(AppError::NotFound(format!(
                "Player with email {} not found.",
                email_to_find
            )))
        }
        Err(e) => Err(e)
    }
}

/// Creates a new group, assigns ownership, and adds initial members.
///
/// Validates instructor existence, group name uniqueness, and member existence.
/// Performs all database operations within a single transaction.
///
/// Request Body: `CreateGroupPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `i64`: The ID of the newly created group (200).
/// * `None`: If validation fails (instructor not found, name taken, members invalid) (404).
/// * `None`: If a database error or transaction failure occurs (500).
#[instrument(skip(pool, payload))]
pub async fn create_group(
    State(pool): State<Pool>,
    Json(payload): Json<CreateGroupPayload>,
) -> Result<ApiResponse<i64>, AppError> {
    let display_name_cloned = payload.display_name.clone();

    info!(
        "Attempting to create group '{}' by instructor {}",
        payload.display_name, payload.instructor_id
    );
    debug!("Create group payload: {:?}", payload);

    let instructor_exists = helper::run_query(&pool, {
        let instructor_id = payload.instructor_id;
        move |conn| {
            diesel::select(exists(instructors_dsl::instructors.find(instructor_id)))
                .get_result::<bool>(conn)
        }
    }).await?;
    if !instructor_exists {
        error!(
            "Cannot create group: Instructor with ID {} not found.",
            payload.instructor_id
        );
        return Err(AppError::NotFound(format!(
            "Instructor with ID {} not found.",
            payload.instructor_id
        )));
    }

    let name_taken = helper::run_query(&pool, {
        let name = payload.display_name.clone();
        move |conn| {
            diesel::select(exists(
                groups_dsl::groups.filter(groups_dsl::display_name.eq(name)),
            )).get_result::<bool>(conn)
        }
    })
        .await?;
    if name_taken {
        warn!("Group name '{}' is already taken.", payload.display_name);
        return Err(AppError::NotFound(format!(
            "Group name '{}' is already taken.",
            payload.display_name
        )));
    }

    let members_to_add = &payload.member_list;
    if !members_to_add.is_empty() {
        let existing_players_count = helper::run_query(&pool, {
            let member_ids = members_to_add.clone();
            move |conn| {
                players_dsl::players
                    .filter(players_dsl::id.eq_any(&member_ids))
                    .count()
                    .get_result::<i64>(conn)
            }
        }).await?;

        if existing_players_count != members_to_add.len() as i64 {
            error!(
                "Cannot create group: One or more player IDs in memberList do not exist. Expected {}, found {}.",
                members_to_add.len(), existing_players_count
            );
            return Err(AppError::NotFound(
                "One or more players listed as members do not exist.".to_string(),
            ));
        }
        info!("All {} specified members validated.", members_to_add.len());
    }

    let conn = pool.get().await.map_err(AppError::PoolError)?;
    let creation_result = conn
        .interact(move |conn_sync| {
            conn_sync.transaction(|transaction_conn| {
                let new_group = NewGroup {
                    display_name: payload.display_name,
                    display_avatar: payload.display_avatar,
                };
                let new_group_id = diesel::insert_into(groups_dsl::groups)
                    .values(&new_group)
                    .returning(groups_dsl::id)
                    .get_result::<i64>(transaction_conn)?;

                let new_ownership = NewGroupOwnership {
                    group_id: new_group_id,
                    instructor_id: payload.instructor_id,
                    owner: true,
                };
                diesel::insert_into(gro_dsl::group_ownership)
                    .values(&new_ownership)
                    .execute(transaction_conn)?;

                if !payload.member_list.is_empty() {
                    let new_members: Vec<NewPlayerGroup> = payload
                        .member_list
                        .iter()
                        .map(|&player_id| NewPlayerGroup {
                            player_id,
                            group_id: new_group_id,
                        })
                        .collect();

                    diesel::insert_into(pg_dsl::player_groups)
                        .values(&new_members)
                        .execute(transaction_conn)?;
                }

                Ok(new_group_id)
            })
        })
        .await;

    match creation_result {
        Ok(Ok(new_group_id)) => {
            info!(
                "Successfully created group '{}' with ID: {} and assigned ownership to instructor {}.",
                &display_name_cloned, new_group_id, payload.instructor_id
            );
            Ok(ApiResponse::ok(new_group_id))
        }
        Ok(Err(diesel_err)) => {
            error!("Group creation transaction failed: {:?}", diesel_err);
            if let DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, info) = &diesel_err {
                warn!("Unique constraint violation during group creation (likely display_name race condition): {}", info.message());
                Err(AppError::Internal(anyhow!("Group name '{}' is already taken (race condition).", display_name_cloned)))
            } else if let DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, info) = &diesel_err {
                error!("Foreign key violation during group creation transaction: {}", info.message());
                Err(AppError::NotFound("Referenced instructor or player not found during transaction.".to_string()))
            }
            else {
                Err(AppError::DieselError(diesel_err))
            }
        }
        Err(interact_err) => {
            error!("Interaction error during group creation: {:?}", interact_err);
            Err(AppError::InteractError(interact_err))
        }
    }
}

/// Dissolves a group, removing all members and ownership records.
///
/// Requires the requesting instructor to be an owner of the group or an admin (ID 0).
/// Performs deletions from `player_groups`, `group_ownership`, and `groups` within a transaction.
///
/// Request Body: `DissolveGroupPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the group was successfully dissolved (200).
/// * `None`: If instructor lacks permission, or the group doesn't exist (404).
/// * `None`: If a database error or transaction failure occurs (500).
#[instrument(skip(pool, payload))]
pub async fn dissolve_group(
    State(pool): State<Pool>,
    Json(payload): Json<DissolveGroupPayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let instructor_id = payload.instructor_id;
    let group_id = payload.group_id;

    info!(
        "Attempting to dissolve group {} requested by instructor {}",
        group_id, instructor_id
    );
    debug!("Dissolve group payload: {:?}", payload);

    helper::check_instructor_group_permission(&pool, instructor_id, group_id).await?;
    info!(
        "Permission check passed for instructor {} on group {}",
        instructor_id, group_id
    );

    let conn = pool.get().await.map_err(AppError::PoolError)?;
    let deletion_result = conn
        .interact(move |conn_sync| {
            conn_sync.transaction(|transaction_conn| {
                let members_deleted = diesel::delete(pg_dsl::player_groups.filter(pg_dsl::group_id.eq(group_id)))
                    .execute(transaction_conn)?;
                info!("Deleted {} member records from player_groups for group {}", members_deleted, group_id);

                let owners_deleted = diesel::delete(gro_dsl::group_ownership.filter(gro_dsl::group_id.eq(group_id)))
                    .execute(transaction_conn)?;
                info!("Deleted {} ownership records from group_ownership for group {}", owners_deleted, group_id);

                let group_deleted = diesel::delete(groups_dsl::groups.find(group_id))
                    .execute(transaction_conn)?;

                if group_deleted == 1 {
                    Ok(())
                } else {
                    error!("Failed to delete group {} itself after deleting dependencies ({} rows affected).", group_id, group_deleted);
                    Err(DieselError::NotFound)
                }
            })
        }).await;

    match deletion_result {
        Ok(Ok(())) => {
            info!("Successfully dissolved group {}", group_id);
            Ok(ApiResponse::ok(true))
        }
        Ok(Err(DieselError::NotFound)) => {
            error!("Group dissolution failed: Group {} not found during final delete step.", group_id);
            Err(AppError::NotFound(format!("Group {} not found during dissolution.", group_id)))
        }
        Ok(Err(diesel_err)) => {
            error!("Group dissolution transaction failed: {:?}", diesel_err);
            Err(AppError::DieselError(diesel_err))
        }
        Err(interact_err) => {
            error!("Interaction error during group dissolution: {:?}", interact_err);
            Err(AppError::InteractError(interact_err))
        }
    }
}

/// Adds a student (player) to a specific group.
///
/// Requires the requesting instructor to be an owner of the group or an admin (ID 0).
/// Validates that the player being added exists.
/// If the player is already a member of the group, the operation succeeds without changes.
///
/// Request Body: `AddGroupMemberPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the student is now a member of the group (either newly added or already present) (200).
/// * `None`: If instructor lacks permission, or the group/player doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, payload))]
pub async fn add_group_member(
    State(pool): State<Pool>,
    Json(payload): Json<AddGroupMemberPayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let instructor_id = payload.instructor_id;
    let group_id = payload.group_id;
    let player_id = payload.player_id;

    info!(
        "Attempting to add player {} to group {} requested by instructor {}",
        player_id, group_id, instructor_id
    );
    debug!("Add group member payload: {:?}", payload);

    helper::check_instructor_group_permission(&pool, instructor_id, group_id).await?;
    info!(
        "Permission check passed for instructor {} on group {}",
        instructor_id, group_id
    );

    let player_exists = helper::run_query(&pool, {
        let player_id = player_id;
        move |conn| {
            diesel::select(exists(players_dsl::players.find(player_id)))
                .get_result::<bool>(conn)
        }
    }).await?;

    if !player_exists {
        error!(
            "Cannot add member: Player with ID {} not found.",
            player_id
        );
        return Err(AppError::NotFound(format!(
            "Player with ID {} not found.",
            player_id
        )));
    }
    info!("Player to add (ID {}) confirmed to exist.", player_id);

    let operation_result = helper::run_query(&pool, move |conn| {
        let new_membership = NewPlayerGroup { player_id, group_id };

        diesel::insert_into(pg_dsl::player_groups)
            .values(&new_membership)
            .on_conflict((pg_dsl::player_id, pg_dsl::group_id))
            .do_nothing()
            .execute(conn)
    }).await;

    match operation_result {
        Ok(rows_affected) => {
            if rows_affected == 1 {
                info!(
                    "Successfully added player {} to group {}",
                    player_id, group_id
                );
            } else {
                info!(
                    "Player {} was already a member of group {}. No changes made.",
                    player_id, group_id
                );
            }
            Ok(ApiResponse::ok(true))
        }
        Err(e @ AppError::DieselError(DieselError::DatabaseError(
                                          DatabaseErrorKind::ForeignKeyViolation,
                                          _,
                                      ))) => {
            error!("Database constraint violation during member addition: {:?}", e);
            Err(AppError::NotFound(
                "Group or Player not found (foreign key violation).".to_string(),
            ))
        }
        Err(e) => Err(e)
    }
}

/// Removes a student (player) from a specific group.
///
/// Requires the requesting instructor to be an owner of the group or an admin (ID 0).
/// Attempts to delete the specific `player_groups` record linking the group and the player.
///
/// Request Body: `RemoveGroupMemberPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the student was successfully removed from the group (200).
/// * `None`: If instructor lacks permission, or the group doesn't exist (404).
/// * `None`: If the student was not a member of the group (target record not found) (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, payload))]
pub async fn remove_group_member(
    State(pool): State<Pool>,
    Json(payload): Json<RemoveGroupMemberPayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let instructor_id = payload.instructor_id;
    let group_id = payload.group_id;
    let player_id = payload.player_id;

    info!(
        "Attempting to remove player {} from group {} requested by instructor {}",
        player_id, group_id, instructor_id
    );
    debug!("Remove group member payload: {:?}", payload);

    helper::check_instructor_group_permission(&pool, instructor_id, group_id).await?;
    info!(
        "Permission check passed for instructor {} on group {}",
        instructor_id, group_id
    );

    let delete_result = helper::run_query(&pool, move |conn| {
        diesel::delete(
            pg_dsl::player_groups
                .filter(pg_dsl::group_id.eq(group_id))
                .filter(pg_dsl::player_id.eq(player_id)),
        ).execute(conn)
    }).await;

    match delete_result {
        Ok(rows_affected) => {
            if rows_affected == 1 {
                info!(
                    "Successfully removed player {} from group {}",
                    player_id, group_id
                );
                Ok(ApiResponse::ok(true))
            } else if rows_affected == 0 {
                warn!(
                    "Player {} was not a member of group {}. No record removed.",
                    player_id, group_id
                );
                Err(AppError::NotFound(format!(
                    "Player {} is not a member of group {}.",
                    player_id, group_id
                )))
            } else {
                error!(
                    "Unexpected number of rows ({}) deleted when removing player {} from group {}",
                    rows_affected, player_id, group_id
                );
                Err(AppError::Internal(anyhow!(
                    "Unexpected error during member removal."
                )))
            }
        }
        Err(e) => Err(e)
    }
}

/// Creates a new player and optionally adds them to a game and/or group.
///
/// Requires the requesting instructor to have permission for the specified game and/or group
/// if those IDs are provided. If neither game nor group is specified, only admin (ID 0) can create players.
/// Validates email uniqueness, and existence of game/group if specified.
/// Performs all database operations within a single transaction.
///
/// Request Body: `CreatePlayerPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `i64`: The ID of the newly created player (200).
/// * `None`: If validation or permission checks fail (404).
/// * `None`: If a database error or transaction failure occurs (500).
#[instrument(skip(pool, payload))]
pub async fn create_player(
    State(pool): State<Pool>,
    Json(payload): Json<CreatePlayerPayload>,
) -> Result<ApiResponse<i64>, AppError> {
    info!(
        "Attempting to create player with email '{}' requested by instructor {}",
        payload.email, payload.instructor_id
    );
    debug!("Create player payload: {:?}", payload);

    if let Some(game_id) = payload.game_id {
        helper::check_instructor_game_permission(&pool, payload.instructor_id, game_id).await?;
        info!(
            "Instructor {} has permission for game {}",
            payload.instructor_id, game_id
        );
    } else if let Some(group_id) = payload.group_id {
        helper::check_instructor_group_permission(&pool, payload.instructor_id, group_id).await?;
        info!(
            "Instructor {} has permission for group {}",
            payload.instructor_id, group_id
        );
    } else {
        if payload.instructor_id != 0 {
            warn!(
                "Permission denied: Instructor {} cannot create player without game/group context.",
                payload.instructor_id
            );
            return Err(AppError::NotFound(
                "Instructor lacks permission to create player without game/group context.".to_string(),
            ));
        }
        info!("Admin instructor creating player without initial game/group context.");
    }

    let email_taken = helper::run_query(&pool, {
        let email = payload.email.clone();
        move |conn| {
            diesel::select(exists(
                players_dsl::players.filter(players_dsl::email.eq(email)),
            ))
                .get_result::<bool>(conn)
        }
    })
        .await?;
    if email_taken {
        warn!("Player email '{}' is already taken.", payload.email);
        return Err(AppError::NotFound(
            "Player email is already taken.".to_string(),
        ));
    }

    let conn = pool.get().await.map_err(AppError::PoolError)?;
    let creation_result = conn
        .interact(move |conn_sync| {
            conn_sync.transaction(|transaction_conn| {
                let new_player = NewPlayer {
                    email: payload.email,
                    display_name: payload.display_name,
                    display_avatar: payload.display_avatar,
                };
                let new_player_id = diesel::insert_into(players_dsl::players)
                    .values(&new_player)
                    .returning(players_dsl::id)
                    .get_result::<i64>(transaction_conn)?;

                if let Some(game_id) = payload.game_id {
                    let language = payload.language.as_deref().unwrap_or("en").to_string();
                    let new_registration = NewPlayerRegistration {
                        player_id: new_player_id,
                        game_id,
                        language,
                        progress: 0,
                        game_state: json!({}),
                    };
                    diesel::insert_into(pr_dsl::player_registrations)
                        .values(&new_registration)
                        .execute(transaction_conn)?;
                }

                if let Some(group_id) = payload.group_id {
                    let new_membership = NewPlayerGroup {
                        player_id: new_player_id,
                        group_id,
                    };
                    diesel::insert_into(pg_dsl::player_groups)
                        .values(&new_membership)
                        .on_conflict((pg_dsl::player_id, pg_dsl::group_id))
                        .do_nothing()
                        .execute(transaction_conn)?;
                }

                Ok(new_player_id)
            })
        })
        .await;

    match creation_result {
        Ok(Ok(new_player_id)) => {
            info!(
                "Successfully created player",
            );
            Ok(ApiResponse::ok(new_player_id))
        }
        Ok(Err(diesel_err)) => {
            error!("Player creation transaction failed: {:?}", diesel_err);
            if let DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, info) = &diesel_err {
                warn!("Unique constraint violation during player creation (likely email race condition): {}", info.message());
                Err(AppError::Internal(anyhow!("Player email is already taken.")))
            } else if let DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, info) = &diesel_err {
                error!("Foreign key violation during player creation transaction: {}", info.message());
                Err(AppError::NotFound("Referenced game or group not found during transaction.".to_string()))
            }
            else {
                Err(AppError::DieselError(diesel_err))
            }
        }
        Err(interact_err) => {
            error!("Interaction error during player creation: {:?}", interact_err);
            Err(AppError::InteractError(interact_err))
        }
    }
}

/// Disables a specific player account by setting their 'disabled' status to true.
///
/// Requires the requesting instructor to be an admin (instructor_id = 0).
///
/// Request Body: `DisablePlayerPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the player was successfully disabled (200).
/// * `None`: If requesting instructor is not admin (404).
/// * `None`: If the target player doesn't exist (404).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, payload))]
pub async fn disable_player(
    State(pool): State<Pool>,
    Json(payload): Json<DisablePlayerPayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let instructor_id = payload.instructor_id;
    let player_id = payload.player_id;

    info!(
        "Attempting to disable player {} requested by instructor {}",
        player_id, instructor_id
    );
    debug!("Disable player payload: {:?}", payload);

    if instructor_id != 0 {
        warn!(
            "Permission denied: Instructor {} is not admin (ID 0) and cannot disable players.",
            instructor_id
        );
        return Err(AppError::NotFound(
            "Only admin users can disable players.".to_string(),
        ));
    }
    info!("Admin permission confirmed for instructor {}", instructor_id);

    let player_exists = helper::run_query(&pool, {
        let player_id = player_id;
        move |conn| {
            diesel::select(exists(players_dsl::players.find(player_id)))
                .get_result::<bool>(conn)
        }
    })
        .await?;

    if !player_exists {
        error!("Cannot disable player: Player with ID {} not found.", player_id);
        return Err(AppError::NotFound(format!(
            "Player with ID {} not found.",
            player_id
        )));
    }
    info!("Player {} confirmed to exist.", player_id);

    let update_result = helper::run_query(&pool, move |conn| {
        diesel::update(players_dsl::players.find(player_id))
            .set((
                players_dsl::disabled.eq(true),
            ))
            .execute(conn)
    }).await;

    match update_result {
        Ok(rows_affected) => {
            if rows_affected == 1 {
                info!("Successfully disabled player {}", player_id);
                Ok(ApiResponse::ok(true))
            } else if rows_affected == 0 {
                error!(
                    "Player {} disable failed: 0 rows affected (unexpected state).",
                    player_id
                );
                Err(AppError::Internal(anyhow!(
                    "Player disable failed unexpectedly (0 rows affected)."
                )))
            } else {
                error!(
                    "Player {} disable failed: {} rows affected (unexpected state).",
                    player_id, rows_affected
                );
                Err(AppError::Internal(anyhow!(
                    "Player disable failed unexpectedly (multiple rows affected)."
                )))
            }
        }
        Err(e) => Err(e)
    }
}

/// Completely deletes a player and all associated data from the platform.
///
/// Requires the requesting instructor to be an admin (instructor_id = 0).
/// Deletes records from submissions, player_registrations, player_groups,
/// player_rewards, player_unlocks, and finally the players table itself,
/// all within a single database transaction. THIS IS A DESTRUCTIVE OPERATION.
///
/// Request Body: `DeletePlayerPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the player and all associated data were successfully deleted (200).
/// * `None`: If requesting instructor is not admin (404).
/// * `None`: If the target player doesn't exist (404).
/// * `None`: If a database error or transaction failure occurs (500).
#[instrument(skip(pool, payload))]
pub async fn delete_player(
    State(pool): State<Pool>,
    Json(payload): Json<DeletePlayerPayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let instructor_id = payload.instructor_id;
    let player_id = payload.player_id;

    info!(
        "Attempting to DELETE player {} requested by instructor {}",
        player_id, instructor_id
    );
    debug!("Delete player payload: {:?}", payload);

    if instructor_id != 0 {
        warn!(
            "Permission denied: Instructor {} is not admin (ID 0) and cannot delete players.",
            instructor_id
        );
        return Err(AppError::NotFound(
                                       "Only admin users can delete players.".to_string(),
        ));
    }
    info!("Admin permission confirmed for instructor {}", instructor_id);

    let player_exists = helper::run_query(&pool, {
        let player_id = player_id;
        move |conn| {
            diesel::select(exists(players_dsl::players.find(player_id)))
                .get_result::<bool>(conn)
        }
    })
        .await?;

    if !player_exists {
        error!("Cannot delete player: Player with ID {} not found.", player_id);
        return Err(AppError::NotFound(format!(
            "Player with ID {} not found.",
            player_id
        )));
    }
    info!("Player {} confirmed to exist. Proceeding with deletion.", player_id);

    let conn = pool.get().await.map_err(AppError::PoolError)?;
    let deletion_result = conn
        .interact(move |conn_sync| {
            conn_sync.transaction(|tx_conn| {
                info!("Deleting submissions for player {}", player_id);
                diesel::delete(sub_dsl::submissions.filter(sub_dsl::player_id.eq(player_id)))
                    .execute(tx_conn)?;

                info!("Deleting player_registrations for player {}", player_id);
                diesel::delete(pr_dsl::player_registrations.filter(pr_dsl::player_id.eq(player_id)))
                    .execute(tx_conn)?;

                info!("Deleting player_groups for player {}", player_id);
                diesel::delete(pg_dsl::player_groups.filter(pg_dsl::player_id.eq(player_id)))
                    .execute(tx_conn)?;

                info!("Deleting player_rewards for player {}", player_id);
                diesel::delete(prw_dsl::player_rewards.filter(prw_dsl::player_id.eq(player_id)))
                    .execute(tx_conn)?;

                info!("Deleting player_unlocks for player {}", player_id);
                diesel::delete(pu_dsl::player_unlocks.filter(pu_dsl::player_id.eq(player_id)))
                    .execute(tx_conn)?;

                info!("Deleting player record for player {}", player_id);
                let player_deleted_count = diesel::delete(players_dsl::players.find(player_id))
                    .execute(tx_conn)?;

                if player_deleted_count == 1 {
                    Ok(())
                } else {
                    error!("Failed to delete player {} itself after deleting dependencies ({} rows affected).", player_id, player_deleted_count);
                    Err(DieselError::NotFound)
                }
            })
        })
        .await;

    match deletion_result {
        Ok(Ok(())) => {
            info!("Successfully deleted player {} and all associated data.", player_id);
            Ok(ApiResponse::ok(true))
        }
        Ok(Err(DieselError::NotFound)) => {
            error!("Player deletion failed: Player {} not found during final delete step.", player_id);
            Err(AppError::NotFound(format!("Player {} not found during deletion.", player_id)))
        }
        Ok(Err(diesel_err)) => {
            error!("Player deletion transaction failed: {:?}", diesel_err);
            Err(AppError::DieselError(diesel_err))
        }
        Err(interact_err) => {
            error!("Interaction error during player deletion: {:?}", interact_err);
            Err(AppError::InteractError(interact_err))
        }
    }
}