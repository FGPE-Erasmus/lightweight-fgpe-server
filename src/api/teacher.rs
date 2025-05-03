use super::helper;
use anyhow::anyhow;

use crate::model::student::NewPlayerRegistration;
use crate::model::teacher::{
    ExerciseStatsResponse, GameChangeset, InstructorGameMetadataResponse, Invite,
    InviteLinkResponse, NewGame, NewGameOwnership, NewGroup, NewGroupOwnership, NewInvite,
    NewPlayer, NewPlayerGroup, StudentExercisesResponse, StudentProgressResponse,
    SubmissionDataResponse,
};
use crate::payloads::teacher::{
    ActivateGamePayload, AddGameInstructorPayload, AddGroupMemberPayload, CreateGamePayload,
    CreateGroupPayload, CreatePlayerPayload, DeletePlayerPayload, DisablePlayerPayload,
    DissolveGroupPayload, GenerateInviteLinkPayload, GetExerciseStatsParams,
    GetExerciseSubmissionsParams, GetInstructorGameMetadataParams, GetStudentExercisesParams,
    GetStudentProgressParams, GetStudentSubmissionsParams, GetSubmissionDataParams,
    ListStudentsParams, ModifyGamePayload, ProcessInviteLinkPayload, RemoveGameInstructorPayload,
    RemoveGameStudentPayload, RemoveGroupMemberPayload, StopGamePayload, TranslateEmailParams,
};
use crate::{
    errors::AppError,
    payloads::teacher::GetInstructorGamesParams,
    response::ApiResponse,
    schema::{
        courses::dsl as courses_dsl, exercises::dsl as exercises_dsl,
        game_ownership::dsl as go_dsl, games::dsl as games_dsl, group_ownership::dsl as gro_dsl,
        groups::dsl as groups_dsl, instructors::dsl as instructors_dsl,
        invites::dsl as invites_dsl, modules::dsl as modules_dsl, player_groups::dsl as pg_dsl,
        player_registrations::dsl as pr_dsl, player_rewards::dsl as prw_dsl,
        player_unlocks::dsl as pu_dsl, players::dsl as players_dsl, submissions::dsl as sub_dsl,
    },
};
use axum::{
    Json,
    extract::{Query, State},
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::{exists, select};
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use serde_json::json;
use tracing::log::warn;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

/// Retrieves all game IDs associated with a specific instructor.
///
/// Query Parameters:
/// * `instructor_id`: The ID of the instructor.
///
/// Returns (wrapped in `ApiResponse`)
/// * `Vec<i64>`: List of game IDs (200 OK).
/// * `404 Not Found`: If the specified instructor ID does not exist.
/// * `500 Internal Server Error`: If a database error occurs.
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

    let instructor_exists = helper::run_query(&pool, move |conn| {
        diesel::select(exists(instructors_dsl::instructors.find(instructor_id)))
            .get_result::<bool>(conn)
    })
    .await?;

    if !instructor_exists {
        error!("Instructor with ID {} not found.", instructor_id);
        return Err(AppError::NotFound(format!(
            "Instructor with ID {} not found.",
            instructor_id
        )));
    }
    info!(
        "Instructor {} found. Fetching associated games...",
        instructor_id
    );

    let game_ids = helper::run_query(&pool, move |conn_sync| {
        go_dsl::game_ownership
            .filter(go_dsl::instructor_id.eq(instructor_id))
            .select(go_dsl::game_id)
            .load::<i64>(conn_sync)
    })
    .await?;

    info!(
        "Successfully fetched {} game IDs for instructor_id: {}",
        game_ids.len(),
        instructor_id
    );
    Ok(ApiResponse::ok(game_ids))
}

/// Retrieves detailed metadata for a specific game if the instructor has access.
///
/// Query Parameters:
/// * `instructor_id`: The ID of the instructor requesting the data.
/// * `game_id`: The ID of the game.
///
/// Returns (wrapped in `ApiResponse`)
/// * `InstructorGameMetadataResponse`: Game details, ownership, and player count (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the game.
/// * `404 Not Found`: If the game doesn't exist.
/// * `500 Internal Server Error`: If a database error occurs.
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
        String,
        DateTime<Utc>,
        DateTime<Utc>,
        bool,
        bool,
        i32,
        String,
    ); // title, start, end, active, public, total_ex, desc

    let (title, start_date, end_date, active, public, total_exercises, description) =
        helper::run_query(&pool, {
            move |conn| {
                games_dsl::games
                    .find(game_id)
                    .select((
                        games_dsl::title,
                        games_dsl::start_date,
                        games_dsl::end_date,
                        games_dsl::active,
                        games_dsl::public,
                        games_dsl::total_exercises,
                        games_dsl::description,
                    ))
                    .first::<GameDetailsTuple>(conn)
            }
        })
        .await?;

    let mut is_owner = false;
    if instructor_id != 0 {
        is_owner = helper::run_query(&pool, {
            move |conn| {
                go_dsl::game_ownership
                    .filter(go_dsl::instructor_id.eq(instructor_id))
                    .filter(go_dsl::game_id.eq(game_id))
                    .select(go_dsl::owner)
                    .first::<bool>(conn)
            }
        })
        .await?;
    }

    let player_count = helper::run_query(&pool, {
        move |conn| {
            pr_dsl::player_registrations
                .filter(pr_dsl::game_id.eq(game_id))
                .count()
                .get_result::<i64>(conn)
        }
    })
    .await?;

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
/// Query Parameters:
/// * `instructor_id`: The ID of the instructor requesting the list.
/// * `game_id`: The ID of the game.
/// * `group_id`: Optional group ID to filter by.
/// * `only_active`: If true, filter for non-disabled players.
///
/// Returns (wrapped in `ApiResponse`)
/// * `Vec<i64>`: List of player IDs matching criteria (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the game.
/// * `404 Not Found`: If the game or the optional filter group doesn't exist.
/// * `500 Internal Server Error`: If a database error occurs.
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

    if let Some(gid) = group_id_filter {
        let group_exists = helper::run_query(&pool, {
            move |conn| {
                diesel::select(exists(groups_dsl::groups.find(gid))).get_result::<bool>(conn)
            }
        })
        .await?;
        if !group_exists {
            error!("Filter group with ID {} not found.", gid);
            return Err(AppError::NotFound(format!(
                "Filter group with ID {} not found.",
                gid
            )));
        }
        info!("Filter group {} confirmed to exist.", gid);
    }

    let student_ids = helper::run_query(&pool, move |conn_sync| {
        let game_id = game_id;
        let group_id_filter = group_id_filter;
        let only_active_filter = only_active_filter;

        if let Some(gid) = group_id_filter {
            info!("Applying filter: group_id = {}", gid);
            let mut query = pr_dsl::player_registrations
                .filter(pr_dsl::game_id.eq(game_id))
                .inner_join(players_dsl::players.on(pr_dsl::player_id.eq(players_dsl::id)))
                .inner_join(pg_dsl::player_groups.on(pg_dsl::player_id.eq(players_dsl::id)))
                .filter(pg_dsl::group_id.eq(gid))
                .select(players_dsl::id)
                .distinct()
                .into_boxed();

            if only_active_filter {
                info!("Applying filter: only_active = true (players.disabled = false)");
                query = query.filter(players_dsl::disabled.eq(false));
            }

            query.load::<i64>(conn_sync)
        } else {
            let mut query = pr_dsl::player_registrations
                .filter(pr_dsl::game_id.eq(game_id))
                .inner_join(players_dsl::players.on(pr_dsl::player_id.eq(players_dsl::id)))
                .select(players_dsl::id)
                .distinct()
                .into_boxed();

            if only_active_filter {
                info!("Applying filter: only_active = true (players.disabled = false)");
                query = query.filter(players_dsl::disabled.eq(false));
            }

            query.load::<i64>(conn_sync)
        }
    })
    .await?;

    info!(
        "Successfully fetched {} student IDs for game_id: {} with applied filters.",
        student_ids.len(),
        game_id
    );
    Ok(ApiResponse::ok(student_ids))
}

/// Retrieves progress metrics for a specific student within a specific game.
///
/// Query Parameters:
/// * `instructor_id`: The ID of the instructor.
/// * `game_id`: The ID of the game.
/// * `player_id`: The ID of the student.
///
/// Returns (wrapped in `ApiResponse`)
/// * `StudentProgressResponse`: Attempts, solved count, and progress percentage (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the game.
/// * `404 Not Found`: If the game/player doesn't exist, or player not registered in game.
/// * `500 Internal Server Error`: If a database error occurs.
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

    let registration_info = helper::run_query(&pool, {
        move |conn| {
            pr_dsl::player_registrations
                .filter(pr_dsl::player_id.eq(player_id))
                .filter(pr_dsl::game_id.eq(game_id))
                .inner_join(games_dsl::games.on(pr_dsl::game_id.eq(games_dsl::id)))
                .select((pr_dsl::id, games_dsl::total_exercises))
                .first::<(i64, i32)>(conn)
                .optional()
        }
    })
    .await?;

    let game_total_exercises = match registration_info {
        Some((_reg_id, total_ex)) => {
            info!(
                "Player {} confirmed registered in game {}.",
                player_id, game_id
            );
            total_ex
        }
        None => {
            warn!(
                "Player {} is not registered in game {}. Cannot fetch progress.",
                player_id, game_id
            );
            return Err(AppError::NotFound(format!(
                "Player with ID {} is not registered in game with ID {}.",
                player_id, game_id
            )));
        }
    };

    let total_attempts = helper::run_query(&pool, {
        move |conn| {
            sub_dsl::submissions
                .filter(sub_dsl::player_id.eq(player_id))
                .filter(sub_dsl::game_id.eq(game_id))
                .count()
                .get_result::<i64>(conn)
        }
    })
    .await?;

    let solved_exercises_count = helper::run_query(&pool, {
        move |conn| {
            sub_dsl::submissions
                .filter(sub_dsl::player_id.eq(player_id))
                .filter(sub_dsl::game_id.eq(game_id))
                .filter(sub_dsl::first_solution.eq(true))
                .select(sub_dsl::exercise_id)
                .distinct()
                .count()
                .get_result::<i64>(conn)
        }
    })
    .await?;

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
/// Query Parameters:
/// * `instructor_id`: The ID of the instructor.
/// * `game_id`: The ID of the game.
/// * `player_id`: The ID of the student.
///
/// Returns (wrapped in `ApiResponse`)
/// * `StudentExercisesResponse`: Lists of attempted and solved exercise IDs (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the game.
/// * `404 Not Found`: If the game/player doesn't exist, or player not registered in game.
/// * `500 Internal Server Error`: If a database error occurs.
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

    let is_registered = helper::run_query(&pool, {
        move |conn| {
            diesel::select(exists(
                pr_dsl::player_registrations
                    .filter(pr_dsl::player_id.eq(player_id))
                    .filter(pr_dsl::game_id.eq(game_id)),
            ))
            .get_result::<bool>(conn)
        }
    })
    .await?;

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
    info!(
        "Player {} confirmed registered in game {}.",
        player_id, game_id
    );

    let attempted_exercises_list = helper::run_query(&pool, {
        move |conn| {
            sub_dsl::submissions
                .filter(sub_dsl::player_id.eq(player_id))
                .filter(sub_dsl::game_id.eq(game_id))
                .select(sub_dsl::exercise_id)
                .distinct()
                .load::<i64>(conn)
        }
    })
    .await?;

    let solved_exercises_list = helper::run_query(&pool, {
        move |conn| {
            sub_dsl::submissions
                .filter(sub_dsl::player_id.eq(player_id))
                .filter(sub_dsl::game_id.eq(game_id))
                .filter(sub_dsl::first_solution.eq(true))
                .select(sub_dsl::exercise_id)
                .distinct()
                .load::<i64>(conn)
        }
    })
    .await?;

    let response_data = StudentExercisesResponse {
        attempted_exercises: attempted_exercises_list,
        solved_exercises: solved_exercises_list,
    };

    info!(
        "Successfully fetched exercise lists for player_id: {} in game_id: {}. Attempted: {}, Solved: {}",
        player_id,
        game_id,
        response_data.attempted_exercises.len(),
        response_data.solved_exercises.len()
    );
    Ok(ApiResponse::ok(response_data))
}

/// Retrieves a list of submission IDs for a specific student within a game, with optional success filter.
///
/// Query Parameters:
/// * `instructor_id`: The ID of the instructor.
/// * `game_id`: The ID of the game.
/// * `player_id`: The ID of the student.
/// * `success_only`: If true, filter for submissions with result >= 50.
///
/// Returns (wrapped in `ApiResponse`)
/// * `Vec<i64>`: List of submission IDs matching criteria (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the game.
/// * `404 Not Found`: If the game/player doesn't exist, or player not registered in game.
/// * `500 Internal Server Error`: If a database error occurs.
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

    let is_registered = helper::run_query(&pool, {
        move |conn| {
            diesel::select(exists(
                pr_dsl::player_registrations
                    .filter(pr_dsl::player_id.eq(player_id))
                    .filter(pr_dsl::game_id.eq(game_id)),
            ))
            .get_result::<bool>(conn)
        }
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
    info!(
        "Player {} confirmed registered in game {}.",
        player_id, game_id
    );

    let submission_ids = helper::run_query(&pool, move |conn_sync| {
        let player_id = player_id;
        let game_id = game_id;
        let success_only_filter = success_only_filter;

        let mut query = sub_dsl::submissions
            .filter(sub_dsl::player_id.eq(player_id))
            .filter(sub_dsl::game_id.eq(game_id))
            .select(sub_dsl::id)
            .order(sub_dsl::submitted_at.desc())
            .into_boxed();

        if success_only_filter {
            info!("Applying filter: success_only = true (result >= 50)");
            let success_threshold = BigDecimal::from(50);
            query = query.filter(sub_dsl::result.ge(success_threshold));
        }

        query.load::<i64>(conn_sync)
    })
    .await?;

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
/// Query Parameters:
/// * `instructor_id`: The ID of the instructor.
/// * `submission_id`: The ID of the submission.
///
/// Returns (wrapped in `ApiResponse`)
/// * `SubmissionDataResponse`: Full submission data (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the associated game.
/// * `404 Not Found`: If the submission is not found or the associated game does not exist.
/// * `500 Internal Server Error`: If a database error occurs.
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

    let submission_data = helper::run_query(&pool, {
        move |conn| {
            sub_dsl::submissions
                .find(submission_id)
                .first::<SubmissionDataResponse>(conn)
        }
    })
    .await?;

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
/// Query Parameters:
/// * `instructor_id`: The ID of the instructor.
/// * `game_id`: The ID of the game.
/// * `exercise_id`: The ID of the exercise.
///
/// Returns (wrapped in `ApiResponse`)
/// * `ExerciseStatsResponse`: Calculated exercise statistics (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the game.
/// * `404 Not Found`: If the game or exercise doesn't exist.
/// * `500 Internal Server Error`: If a database error occurs.
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

    let exercise_exists = helper::run_query(&pool, {
        move |conn| {
            diesel::select(exists(exercises_dsl::exercises.find(exercise_id)))
                .get_result::<bool>(conn)
        }
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
        move |conn| {
            sub_dsl::submissions
                .filter(sub_dsl::game_id.eq(game_id))
                .filter(sub_dsl::exercise_id.eq(exercise_id))
                .count()
                .get_result::<i64>(conn)
        }
    })
    .await?;

    let successful_attempts = helper::run_query(&pool, {
        let success_threshold = success_threshold.clone();
        move |conn| {
            sub_dsl::submissions
                .filter(sub_dsl::game_id.eq(game_id))
                .filter(sub_dsl::exercise_id.eq(exercise_id))
                .filter(sub_dsl::result.ge(success_threshold))
                .count()
                .get_result::<i64>(conn)
        }
    })
    .await?;

    let first_solutions_count = helper::run_query(&pool, {
        move |conn| {
            sub_dsl::submissions
                .filter(sub_dsl::game_id.eq(game_id))
                .filter(sub_dsl::exercise_id.eq(exercise_id))
                .filter(sub_dsl::first_solution.eq(true))
                .select(sub_dsl::player_id)
                .distinct()
                .count()
                .get_result::<i64>(conn)
        }
    })
    .await?;

    let total_players_in_game = helper::run_query(&pool, {
        move |conn| {
            pr_dsl::player_registrations
                .filter(pr_dsl::game_id.eq(game_id))
                .count()
                .get_result::<i64>(conn)
        }
    })
    .await?;

    let difficulty = if total_attempts > 0 {
        100.0 - (successful_attempts as f64 / total_attempts as f64 * 100.0)
    } else {
        0.0
    };

    let solved_percentage = if total_players_in_game > 0 {
        first_solutions_count as f64 / total_players_in_game as f64 * 100.0
    } else {
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
/// Query Parameters:
/// * `instructor_id`: The ID of the instructor.
/// * `game_id`: The ID of the game.
/// * `exercise_id`: The ID of the exercise.
/// * `success_only`: If true, filter for submissions with result >= 50.
///
/// Returns (wrapped in `ApiResponse`)
/// * `Vec<i64>`: List of submission IDs matching criteria (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the game.
/// * `404 Not Found`: If the game or exercise doesn't exist.
/// * `500 Internal Server Error`: If a database error occurs.
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

    let exercise_exists = helper::run_query(&pool, {
        move |conn| {
            diesel::select(exists(exercises_dsl::exercises.find(exercise_id)))
                .get_result::<bool>(conn)
        }
    })
    .await?;

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
        let game_id = game_id;
        let exercise_id = exercise_id;
        let success_only_filter = success_only_filter;

        let mut query = sub_dsl::submissions
            .filter(sub_dsl::game_id.eq(game_id))
            .filter(sub_dsl::exercise_id.eq(exercise_id))
            .select(sub_dsl::id)
            .order(sub_dsl::submitted_at.desc())
            .into_boxed();

        if success_only_filter {
            info!("Applying filter: success_only = true (result >= 50)");
            let success_threshold = BigDecimal::from(50);
            query = query.filter(sub_dsl::result.ge(success_threshold));
        }

        query.load::<i64>(conn_sync)
    })
    .await?;

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
/// Request Body: `CreateGamePayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `i64`: The ID of the newly created game (200 OK).
/// * `404 Not Found`: If the specified instructor or course does not exist.
/// * `422 Unprocessable Entity`: If the specified programming language is not allowed for the course.
/// * `500 Internal Server Error`: If a database error or transaction failure occurs.
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
    })
    .await;

    let allowed_languages_str = match course_languages {
        Ok(langs) => langs,
        Err(AppError::NotFound(_)) => {
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
        return Err(AppError::UnprocessableEntity(format!(
            "Programming language '{}' is not allowed for course {}. Allowed: {:?}",
            payload.programming_language, payload.course_id, allowed_languages
        )));
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
    })
    .await?;
    info!(
        "Calculated {} total exercises for course {} and language {}.",
        total_exercises_count, payload.course_id, payload.programming_language
    );

    let conn = pool.get().await?;
    let creation_result: Result<i64, AppError> = conn
        .interact(move |conn_sync| {
            let payload = payload;
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
                    .get_result::<i64>(transaction_conn)
                    .map_err(|e| {
                        if let DieselError::DatabaseError(
                            DatabaseErrorKind::ForeignKeyViolation,
                            _,
                        ) = e
                        {
                            AppError::NotFound(
                                "Referenced course not found during transaction.".to_string(),
                            )
                        } else {
                            AppError::from(e)
                        }
                    })?;

                let new_ownership = NewGameOwnership {
                    game_id: inserted_game_id,
                    instructor_id: payload.instructor_id,
                    owner: true,
                };

                diesel::insert_into(go_dsl::game_ownership)
                    .values(&new_ownership)
                    .execute(transaction_conn)
                    .map_err(|e| {
                        if let DieselError::DatabaseError(
                            DatabaseErrorKind::ForeignKeyViolation,
                            _,
                        ) = e
                        {
                            AppError::NotFound(
                                "Referenced instructor not found during transaction.".to_string(),
                            )
                        } else {
                            AppError::from(e)
                        }
                    })?;

                Ok(inserted_game_id)
            })
        })
        .await?;

    creation_result.map(ApiResponse::ok)
}

/// Modifies settings of an existing game.
///
/// Request Body: `ModifyGamePayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the update was successful (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the game.
/// * `404 Not Found`: If the game doesn't exist.
/// * `500 Internal Server Error`: If a database error occurs or the update affects an unexpected number of rows.
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
        info!(
            "No update fields provided for game {}. Returning success.",
            game_id
        );
        return Ok(ApiResponse::ok(true));
    }

    let rows_affected = helper::run_query(&pool, {
        move |conn| {
            diesel::update(games_dsl::games.find(game_id))
                .set(&changeset)
                .execute(conn)
        }
    })
    .await?;

    match rows_affected {
        1 => {
            info!("Successfully modified game {}", game_id);
            Ok(ApiResponse::ok(true))
        }
        0 => {
            error!(
                "Game {} modification failed: 0 rows affected (game not found after permission check).",
                game_id
            );
            Err(AppError::NotFound(format!(
                "Game with ID {} not found during update.",
                game_id
            )))
        }
        n => {
            error!(
                "Game {} modification failed: {} rows affected (unexpected state).",
                game_id, n
            );
            Err(AppError::InternalServerError(anyhow!(
                "Game modification failed unexpectedly (multiple rows affected)."
            )))
        }
    }
}

/// Adds an instructor to a game's ownership list or updates their owner status.
///
/// Request Body: `AddGameInstructorPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the instructor was successfully added or updated (200 OK).
/// * `403 Forbidden`: If the requesting instructor lacks permission for the game.
/// * `404 Not Found`: If the game or the instructor_to_add doesn't exist.
/// * `500 Internal Server Error`: If a database error occurs.
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

    helper::check_instructor_game_owner_permission(&pool, requesting_instructor_id, game_id)
        .await?;
    info!(
        "Owner permission check passed for instructor {} on game {}",
        requesting_instructor_id, game_id
    );

    let instructor_to_add_exists = helper::run_query(&pool, {
        move |conn| {
            diesel::select(exists(
                instructors_dsl::instructors.find(instructor_to_add_id),
            ))
            .get_result::<bool>(conn)
        }
    })
    .await?;

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
        let game_id = game_id;
        let instructor_to_add_id = instructor_to_add_id;
        let is_owner = is_owner;

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
    })
    .await;

    match operation_result {
        Ok(rows_affected) => {
            info!(
                "Successfully added/updated instructor {} for game {}. Owner set to: {}. Rows affected: {}",
                instructor_to_add_id, game_id, is_owner, rows_affected
            );
            Ok(ApiResponse::ok(true))
        }
        Err(AppError::InternalServerError(ref err)) => {
            if let Some(db_err) = err.downcast_ref::<DieselError>() {
                if let DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) =
                    db_err
                {
                    error!(
                        "Database constraint violation during instructor addition: {:?}",
                        err
                    );
                    return Err(AppError::NotFound(
                        "Game or Instructor not found (foreign key violation).".to_string(),
                    ));
                }
            }
            Err(operation_result.unwrap_err())
        }
        Err(e) => Err(e),
    }
}

/// Removes an instructor's association (ownership record) from a game.
///
/// Request Body: `RemoveGameInstructorPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the association was successfully removed (200 OK).
/// * `403 Forbidden`: If the requesting instructor lacks permission for the game.
/// * `404 Not Found`: If the game doesn't exist, or the instructor was not associated with the game.
/// * `500 Internal Server Error`: If a database error occurs or multiple records are deleted unexpectedly.
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

    helper::check_instructor_game_owner_permission(&pool, requesting_instructor_id, game_id)
        .await?;
    info!(
        "Owner permission check passed for instructor {} on game {}",
        requesting_instructor_id, game_id
    );

    let rows_affected = helper::run_query(&pool, move |conn| {
        let game_id = game_id;
        let instructor_to_remove_id = instructor_to_remove_id;
        diesel::delete(
            go_dsl::game_ownership
                .filter(go_dsl::game_id.eq(game_id))
                .filter(go_dsl::instructor_id.eq(instructor_to_remove_id)),
        )
        .execute(conn)
    })
    .await?;

    match rows_affected {
        1 => {
            info!(
                "Successfully removed instructor {} from game {}",
                instructor_to_remove_id, game_id
            );
            Ok(ApiResponse::ok(true))
        }
        0 => {
            warn!(
                "Instructor {} was not associated with game {}. No record removed.",
                instructor_to_remove_id, game_id
            );
            Err(AppError::NotFound(format!(
                "Instructor {} is not associated with game {}.",
                instructor_to_remove_id, game_id
            )))
        }
        n => {
            error!(
                "Unexpected number of rows ({}) deleted when removing instructor {} from game {}",
                n, instructor_to_remove_id, game_id
            );
            Err(AppError::InternalServerError(anyhow!(
                "Unexpected error during instructor removal."
            )))
        }
    }
}

/// Activates a specific game by setting its 'active' status to true.
///
/// Request Body: `ActivateGamePayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the game was successfully activated (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the game.
/// * `404 Not Found`: If the game doesn't exist.
/// * `500 Internal Server Error`: If a database error occurs or the update affects an unexpected number of rows.
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

    let rows_affected = helper::run_query(&pool, move |conn| {
        let game_id = game_id;
        diesel::update(games_dsl::games.find(game_id))
            .set((
                games_dsl::active.eq(true),
                games_dsl::updated_at.eq(diesel::dsl::now),
            ))
            .execute(conn)
    })
    .await?;

    match rows_affected {
        1 => {
            info!("Successfully activated game {}", game_id);
            Ok(ApiResponse::ok(true))
        }
        0 => {
            error!(
                "Game {} activation failed: 0 rows affected (game not found after permission check).",
                game_id
            );
            Err(AppError::NotFound(format!(
                "Game with ID {} not found during update.",
                game_id
            )))
        }
        n => {
            error!(
                "Game {} activation failed: {} rows affected (unexpected state).",
                game_id, n
            );
            Err(AppError::InternalServerError(anyhow!(
                "Game activation failed unexpectedly (multiple rows affected)."
            )))
        }
    }
}

/// Deactivates a specific game by setting its 'active' status to false.
///
/// Request Body: `StopGamePayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the game was successfully deactivated (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the game.
/// * `404 Not Found`: If the game doesn't exist.
/// * `500 Internal Server Error`: If a database error occurs or the update affects an unexpected number of rows.
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

    let rows_affected = helper::run_query(&pool, move |conn| {
        let game_id = game_id;
        diesel::update(games_dsl::games.find(game_id))
            .set((
                games_dsl::active.eq(false),
                games_dsl::updated_at.eq(diesel::dsl::now),
            ))
            .execute(conn)
    })
    .await?;

    match rows_affected {
        1 => {
            info!("Successfully stopped (deactivated) game {}", game_id);
            Ok(ApiResponse::ok(true))
        }
        0 => {
            error!(
                "Game {} deactivation failed: 0 rows affected (game not found after permission check).",
                game_id
            );
            Err(AppError::NotFound(format!(
                "Game with ID {} not found during update.",
                game_id
            )))
        }
        n => {
            error!(
                "Game {} deactivation failed: {} rows affected (unexpected state).",
                game_id, n
            );
            Err(AppError::InternalServerError(anyhow!(
                "Game deactivation failed unexpectedly (multiple rows affected)."
            )))
        }
    }
}

/// Removes a student's registration from a specific game.
///
/// Request Body: `RemoveGameStudentPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the registration was successfully removed (200 OK).
/// * `403 Forbidden`: If the instructor lacks permission for the game.
/// * `404 Not Found`: If the game doesn't exist, or the student was not registered in the game.
/// * `500 Internal Server Error`: If a database error occurs or multiple records are deleted unexpectedly.
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

    let rows_affected = helper::run_query(&pool, move |conn| {
        let game_id = game_id;
        let student_id = student_id;
        diesel::delete(
            pr_dsl::player_registrations
                .filter(pr_dsl::game_id.eq(game_id))
                .filter(pr_dsl::player_id.eq(student_id)),
        )
        .execute(conn)
    })
    .await?;

    match rows_affected {
        1 => {
            info!(
                "Successfully removed student {} from game {}",
                student_id, game_id
            );
            Ok(ApiResponse::ok(true))
        }
        0 => {
            warn!(
                "Student {} was not registered in game {}. No record removed.",
                student_id, game_id
            );
            Err(AppError::NotFound(format!(
                "Student {} is not registered in game {}.",
                student_id, game_id
            )))
        }
        n => {
            error!(
                "Unexpected number of rows ({}) deleted when removing student {} from game {}",
                n, student_id, game_id
            );
            Err(AppError::InternalServerError(anyhow!(
                "Unexpected error during student removal."
            )))
        }
    }
}

/// Finds the player ID associated with a given email address.
///
/// Query Parameters:
/// * `email`: The email address to look up.
///
/// Returns (wrapped in `ApiResponse`)
/// * `i64`: The player ID if found (200 OK).
/// * `404 Not Found`: If no player with the given email exists.
/// * `500 Internal Server Error`: If a database error occurs.
#[instrument(skip(pool, params))]
pub async fn translate_email_to_player_id(
    State(pool): State<Pool>,
    Query(params): Query<TranslateEmailParams>,
) -> Result<ApiResponse<i64>, AppError> {
    let email_to_find = params.email;
    let email_cloned = email_to_find.clone();

    info!("Attempting to find player ID for email: {}", &email_to_find);
    debug!("Translate email params: {:?}", &email_to_find);

    let player_id = helper::run_query(&pool, move |conn| {
        players_dsl::players
            .filter(players_dsl::email.eq(email_cloned))
            .select(players_dsl::id)
            .first::<i64>(conn)
    })
    .await?;

    info!(
        "Successfully found player ID {} for email {}",
        player_id, &email_to_find
    );
    Ok(ApiResponse::ok(player_id))
}

/// Creates a new group, assigns ownership, and adds initial members.
///
/// Request Body: `CreateGroupPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `i64`: The ID of the newly created group (200 OK).
/// * `404 Not Found`: If the requesting instructor or any specified member player does not exist.
/// * `409 Conflict`: If the group display name is already taken.
/// * `500 Internal Server Error`: If a database error or transaction failure occurs.
#[instrument(skip(pool, payload))]
pub async fn create_group(
    State(pool): State<Pool>,
    Json(payload): Json<CreateGroupPayload>,
) -> Result<ApiResponse<i64>, AppError> {
    let display_name_cloned = payload.display_name.clone();
    let instructor_id = payload.instructor_id;

    info!(
        "Attempting to create group '{}' by instructor {}",
        &display_name_cloned, instructor_id
    );
    debug!("Create group payload: {:?}", payload);

    let instructor_exists = helper::run_query(&pool, {
        move |conn| {
            diesel::select(exists(instructors_dsl::instructors.find(instructor_id)))
                .get_result::<bool>(conn)
        }
    })
    .await?;
    if !instructor_exists {
        error!(
            "Cannot create group: Instructor with ID {} not found.",
            instructor_id
        );
        return Err(AppError::NotFound(format!(
            "Instructor with ID {} not found.",
            instructor_id
        )));
    }

    let name_taken = helper::run_query(&pool, {
        let name = display_name_cloned.clone();
        move |conn| {
            diesel::select(exists(
                groups_dsl::groups.filter(groups_dsl::display_name.eq(name)),
            ))
            .get_result::<bool>(conn)
        }
    })
    .await?;
    if name_taken {
        warn!("Group name '{}' is already taken.", &display_name_cloned);
        return Err(AppError::Conflict(format!(
            "Group name '{}' is already taken.",
            display_name_cloned
        )));
    }

    let members_to_add = payload.member_list.clone();
    if !members_to_add.is_empty() {
        let existing_players_count = helper::run_query(&pool, {
            let member_ids = members_to_add.clone();
            move |conn| {
                players_dsl::players
                    .filter(players_dsl::id.eq_any(&member_ids))
                    .count()
                    .get_result::<i64>(conn)
            }
        })
        .await?;

        if existing_players_count != members_to_add.len() as i64 {
            error!(
                "Cannot create group: One or more player IDs in memberList do not exist. Expected {}, found {}.",
                members_to_add.len(),
                existing_players_count
            );
            return Err(AppError::NotFound(
                "One or more players listed as members do not exist.".to_string(),
            ));
        }
        info!("All {} specified members validated.", members_to_add.len());
    }

    let conn = pool.get().await?;
    let creation_result: Result<i64, AppError> = conn
        .interact(move |conn_sync| {
            let payload = payload;
            let display_name_cloned = display_name_cloned;
            conn_sync.transaction(|transaction_conn| {
                let new_group = NewGroup {
                    display_name: payload.display_name,
                    display_avatar: payload.display_avatar,
                };
                let new_group_id = diesel::insert_into(groups_dsl::groups)
                    .values(&new_group)
                    .returning(groups_dsl::id)
                    .get_result::<i64>(transaction_conn)
                    .map_err(|e| {
                        if let DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) = e
                        {
                            AppError::Conflict(format!(
                                "Group name '{}' is already taken (race condition).",
                                display_name_cloned
                            ))
                        } else {
                            AppError::from(e)
                        }
                    })?;

                let new_ownership = NewGroupOwnership {
                    group_id: new_group_id,
                    instructor_id: payload.instructor_id,
                    owner: true,
                };
                diesel::insert_into(gro_dsl::group_ownership)
                    .values(&new_ownership)
                    .execute(transaction_conn)
                    .map_err(|e| {
                        if let DieselError::DatabaseError(
                            DatabaseErrorKind::ForeignKeyViolation,
                            _,
                        ) = e
                        {
                            AppError::NotFound(
                                "Referenced instructor not found during transaction.".to_string(),
                            )
                        } else {
                            AppError::from(e)
                        }
                    })?;

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
                        .execute(transaction_conn)
                        .map_err(|e| {
                            if let DieselError::DatabaseError(
                                DatabaseErrorKind::ForeignKeyViolation,
                                _,
                            ) = e
                            {
                                AppError::NotFound(
                                    "Referenced player not found during transaction.".to_string(),
                                )
                            } else {
                                AppError::from(e)
                            }
                        })?;
                }

                Ok(new_group_id)
            })
        })
        .await?;

    creation_result.map(ApiResponse::ok)
}

/// Dissolves a group, removing all members and ownership records.
///
/// Request Body: `DissolveGroupPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the group was successfully dissolved (200 OK).
/// * `403 Forbidden`: If the instructor lacks owner permission for the group.
/// * `404 Not Found`: If the group doesn't exist.
/// * `500 Internal Server Error`: If a database error or transaction failure occurs.
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

    let conn = pool.get().await?;
    let deletion_result: Result<(), AppError> = conn.interact(move |conn_sync| {
        let group_id = group_id;
        conn_sync.transaction(|transaction_conn| {
            info!("Deleting member records from player_groups for group {}", group_id);
            let members_deleted = diesel::delete(pg_dsl::player_groups.filter(pg_dsl::group_id.eq(group_id)))
                .execute(transaction_conn)
                .map_err(AppError::from)?;
            info!("Deleted {} member records from player_groups for group {}", members_deleted, group_id);

            info!("Deleting ownership records from group_ownership for group {}", group_id);
            let owners_deleted = diesel::delete(gro_dsl::group_ownership.filter(gro_dsl::group_id.eq(group_id)))
                .execute(transaction_conn)
                .map_err(AppError::from)?;
            info!("Deleted {} ownership records from group_ownership for group {}", owners_deleted, group_id);

            info!("Deleting group record for group {}", group_id);
            let group_deleted = diesel::delete(groups_dsl::groups.find(group_id))
                .execute(transaction_conn)
                .map_err(AppError::from)?;

            if group_deleted == 1 {
                Ok(())
            } else {
                error!("Failed to delete group {} itself after deleting dependencies ({} rows affected).", group_id, group_deleted);
                Err(AppError::NotFound(format!("Group {} not found during final delete step.", group_id)))
            }
        })
    }).await?;

    deletion_result.map(|_| ApiResponse::ok(true))
}

/// Adds a student (player) to a specific group.
///
/// Request Body: `AddGroupMemberPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the student is now a member (either newly added or already present) (200 OK).
/// * `403 Forbidden`: If the instructor lacks owner permission for the group.
/// * `404 Not Found`: If the group or player doesn't exist.
/// * `500 Internal Server Error`: If a database error occurs.
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
        move |conn| {
            diesel::select(exists(players_dsl::players.find(player_id))).get_result::<bool>(conn)
        }
    })
    .await?;

    if !player_exists {
        error!("Cannot add member: Player with ID {} not found.", player_id);
        return Err(AppError::NotFound(format!(
            "Player with ID {} not found.",
            player_id
        )));
    }
    info!("Player to add (ID {}) confirmed to exist.", player_id);

    let operation_result = helper::run_query(&pool, move |conn| {
        let player_id = player_id;
        let group_id = group_id;
        let new_membership = NewPlayerGroup {
            player_id,
            group_id,
        };

        diesel::insert_into(pg_dsl::player_groups)
            .values(&new_membership)
            .on_conflict((pg_dsl::player_id, pg_dsl::group_id))
            .do_nothing()
            .execute(conn)
    })
    .await;

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
        Err(AppError::InternalServerError(ref err)) => {
            if let Some(db_err) = err.downcast_ref::<DieselError>() {
                if let DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) =
                    db_err
                {
                    error!(
                        "Database constraint violation during member addition: {:?}",
                        err
                    );
                    return Err(AppError::NotFound(
                        "Group or Player not found (foreign key violation).".to_string(),
                    ));
                }
            }
            Err(operation_result.unwrap_err())
        }
        Err(e) => Err(e),
    }
}

/// Removes a student (player) from a specific group.
///
/// Request Body: `RemoveGroupMemberPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the student was successfully removed (200 OK).
/// * `403 Forbidden`: If the instructor lacks owner permission for the group.
/// * `404 Not Found`: If the group doesn't exist, or the student was not a member.
/// * `500 Internal Server Error`: If a database error occurs or multiple records are deleted unexpectedly.
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

    let rows_affected = helper::run_query(&pool, move |conn| {
        let group_id = group_id;
        let player_id = player_id;
        diesel::delete(
            pg_dsl::player_groups
                .filter(pg_dsl::group_id.eq(group_id))
                .filter(pg_dsl::player_id.eq(player_id)),
        )
        .execute(conn)
    })
    .await?;

    match rows_affected {
        1 => {
            info!(
                "Successfully removed player {} from group {}",
                player_id, group_id
            );
            Ok(ApiResponse::ok(true))
        }
        0 => {
            warn!(
                "Player {} was not a member of group {}. No record removed.",
                player_id, group_id
            );
            Err(AppError::NotFound(format!(
                "Player {} is not a member of group {}.",
                player_id, group_id
            )))
        }
        n => {
            error!(
                "Unexpected number of rows ({}) deleted when removing player {} from group {}",
                n, player_id, group_id
            );
            Err(AppError::InternalServerError(anyhow!(
                "Unexpected error during member removal."
            )))
        }
    }
}

/// Creates a new player and optionally adds them to a game and/or group.
///
/// Request Body: `CreatePlayerPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `i64`: The ID of the newly created player (200 OK).
/// * `403 Forbidden`: If a non-admin instructor tries to create a player without game/group context, or lacks permission for the specified game/group.
/// * `404 Not Found`: If the specified game or group does not exist.
/// * `409 Conflict`: If the player email address is already taken.
/// * `500 Internal Server Error`: If a database error or transaction failure occurs.
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
    }
    if let Some(group_id) = payload.group_id {
        helper::check_instructor_group_permission(&pool, payload.instructor_id, group_id).await?;
        info!(
            "Instructor {} has permission for group {}",
            payload.instructor_id, group_id
        );
    }
    if payload.game_id.is_none() && payload.group_id.is_none() && payload.instructor_id != 0 {
        warn!(
            "Permission denied: Instructor {} cannot create player without game/group context.",
            payload.instructor_id
        );
        return Err(AppError::Forbidden(
            "Instructor lacks permission to create player without game/group context.".to_string(),
        ));
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
        return Err(AppError::Conflict(
            "Player email is already taken.".to_string(),
        ));
    }

    let conn = pool.get().await?;
    let creation_result: Result<i64, AppError> = conn
        .interact(move |conn_sync| {
            let payload = payload;
            conn_sync.transaction(|transaction_conn| {
                let new_player = NewPlayer {
                    email: payload.email,
                    display_name: payload.display_name,
                    display_avatar: payload.display_avatar,
                };
                let new_player_id = diesel::insert_into(players_dsl::players)
                    .values(&new_player)
                    .returning(players_dsl::id)
                    .get_result::<i64>(transaction_conn)
                    .map_err(|e| {
                        if let DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) = e
                        {
                            AppError::Conflict(
                                "Player email is already taken (race condition).".to_string(),
                            )
                        } else {
                            AppError::from(e)
                        }
                    })?;

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
                        .execute(transaction_conn)
                        .map_err(|e| {
                            if let DieselError::DatabaseError(
                                DatabaseErrorKind::ForeignKeyViolation,
                                _,
                            ) = e
                            {
                                AppError::NotFound(
                                    "Referenced game not found during transaction.".to_string(),
                                )
                            } else {
                                AppError::from(e)
                            }
                        })?;
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
                        .execute(transaction_conn)
                        .map_err(|e| {
                            if let DieselError::DatabaseError(
                                DatabaseErrorKind::ForeignKeyViolation,
                                _,
                            ) = e
                            {
                                AppError::NotFound(
                                    "Referenced group not found during transaction.".to_string(),
                                )
                            } else {
                                AppError::from(e)
                            }
                        })?;
                }

                Ok(new_player_id)
            })
        })
        .await?;

    creation_result.map(ApiResponse::ok)
}

/// Disables a specific player account by setting their 'disabled' status to true.
///
/// Request Body: `DisablePlayerPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the player was successfully disabled (200 OK).
/// * `403 Forbidden`: If requesting instructor is not admin (ID 0).
/// * `404 Not Found`: If the target player doesn't exist.
/// * `500 Internal Server Error`: If a database error occurs or the update affects an unexpected number of rows.
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
        return Err(AppError::Forbidden(
            "Only admin users can disable players.".to_string(),
        ));
    }
    info!(
        "Admin permission confirmed for instructor {}",
        instructor_id
    );

    let player_exists = helper::run_query(&pool, {
        move |conn| {
            diesel::select(exists(players_dsl::players.find(player_id))).get_result::<bool>(conn)
        }
    })
    .await?;

    if !player_exists {
        error!(
            "Cannot disable player: Player with ID {} not found.",
            player_id
        );
        return Err(AppError::NotFound(format!(
            "Player with ID {} not found.",
            player_id
        )));
    }
    info!("Player {} confirmed to exist.", player_id);

    let rows_affected = helper::run_query(&pool, move |conn| {
        let player_id = player_id;
        diesel::update(players_dsl::players.find(player_id))
            .set(players_dsl::disabled.eq(true))
            .execute(conn)
    })
    .await?;

    match rows_affected {
        1 => {
            info!("Successfully disabled player {}", player_id);
            Ok(ApiResponse::ok(true))
        }
        0 => {
            error!(
                "Player {} disable failed: 0 rows affected (player not found during update).",
                player_id
            );
            Err(AppError::NotFound(format!(
                "Player with ID {} not found during update.",
                player_id
            )))
        }
        n => {
            error!(
                "Player {} disable failed: {} rows affected (unexpected state).",
                player_id, n
            );
            Err(AppError::InternalServerError(anyhow!(
                "Player disable failed unexpectedly (multiple rows affected)."
            )))
        }
    }
}

/// Completely deletes a player and all associated data from the platform.
///
/// Request Body: `DeletePlayerPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the player and all associated data were successfully deleted (200 OK).
/// * `403 Forbidden`: If requesting instructor is not admin (ID 0).
/// * `404 Not Found`: If the target player doesn't exist.
/// * `500 Internal Server Error`: If a database error or transaction failure occurs.
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
        return Err(AppError::Forbidden(
            "Only admin users can delete players.".to_string(),
        ));
    }
    info!(
        "Admin permission confirmed for instructor {}",
        instructor_id
    );

    let player_exists = helper::run_query(&pool, {
        move |conn| {
            diesel::select(exists(players_dsl::players.find(player_id))).get_result::<bool>(conn)
        }
    })
    .await?;

    if !player_exists {
        error!(
            "Cannot delete player: Player with ID {} not found.",
            player_id
        );
        return Err(AppError::NotFound(format!(
            "Player with ID {} not found.",
            player_id
        )));
    }
    info!(
        "Player {} confirmed to exist. Proceeding with deletion.",
        player_id
    );

    let conn = pool.get().await?;
    let deletion_result: Result<(), AppError> = conn.interact(move |conn_sync| {
        let player_id = player_id;
        conn_sync.transaction(|tx_conn| {
            info!("Deleting submissions for player {}", player_id);
            diesel::delete(sub_dsl::submissions.filter(sub_dsl::player_id.eq(player_id)))
                .execute(tx_conn).map_err(AppError::from)?;

            info!("Deleting player_registrations for player {}", player_id);
            diesel::delete(pr_dsl::player_registrations.filter(pr_dsl::player_id.eq(player_id)))
                .execute(tx_conn).map_err(AppError::from)?;

            info!("Deleting player_groups for player {}", player_id);
            diesel::delete(pg_dsl::player_groups.filter(pg_dsl::player_id.eq(player_id)))
                .execute(tx_conn).map_err(AppError::from)?;

            info!("Deleting player_rewards for player {}", player_id);
            diesel::delete(prw_dsl::player_rewards.filter(prw_dsl::player_id.eq(player_id)))
                .execute(tx_conn).map_err(AppError::from)?;

            info!("Deleting player_unlocks for player {}", player_id);
            diesel::delete(pu_dsl::player_unlocks.filter(pu_dsl::player_id.eq(player_id)))
                .execute(tx_conn).map_err(AppError::from)?;

            info!("Deleting player record for player {}", player_id);
            let player_deleted_count = diesel::delete(players_dsl::players.find(player_id))
                .execute(tx_conn).map_err(AppError::from)?;

            if player_deleted_count == 1 {
                Ok(())
            } else {
                error!("Failed to delete player {} itself after deleting dependencies ({} rows affected).", player_id, player_deleted_count);
                Err(AppError::NotFound(format!("Player {} not found during final delete step.", player_id)))
            }
        })
    }).await?;

    deletion_result.map(|_| ApiResponse::ok(true))
}

/// Generates a unique invite link (UUID), optionally associated with a game and/or group.
///
/// Requires the requesting instructor to be an admin (ID 0) OR be listed (owner or not)
/// in `group_ownership` if a `group_id` is specified.
/// Validates existence of instructor, game (if specified), and group (if specified).
///
/// Request Body: `GenerateInviteLinkPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `InviteLinkResponse`: Contains the newly generated UUID (200).
/// * `None`: If validation or permission checks fail (404/403).
/// * `None`: If a database error occurs (500).
#[instrument(skip(pool, payload))]
pub async fn generate_invite_link(
    State(pool): State<Pool>,
    Json(payload): Json<GenerateInviteLinkPayload>,
) -> Result<ApiResponse<InviteLinkResponse>, AppError> {
    let instructor_id = payload.instructor_id;
    let game_id = payload.game_id;
    let group_id = payload.group_id;

    info!(
        "Attempting to generate invite link requested by instructor {}. Game: {:?}, Group: {:?}",
        instructor_id, game_id, group_id
    );
    debug!("Generate invite link payload: {:?}", payload);

    let instructor_exists = helper::run_query(&pool, {
        move |conn| {
            diesel::select(exists(instructors_dsl::instructors.find(instructor_id)))
                .get_result::<bool>(conn)
        }
    })
    .await?;
    if !instructor_exists {
        error!(
            "Cannot generate invite: Requesting instructor with ID {} not found.",
            instructor_id
        );
        return Err(AppError::NotFound(format!(
            "Requesting instructor with ID {} not found.",
            instructor_id
        )));
    }
    info!(
        "Requesting instructor {} confirmed to exist.",
        instructor_id
    );

    if let Some(gid) = group_id {
        let group_permission_ok = helper::run_query(&pool, {
            move |conn| {
                Ok(instructor_id == 0
                    || diesel::select(exists(
                        gro_dsl::group_ownership
                            .filter(gro_dsl::instructor_id.eq(instructor_id))
                            .filter(gro_dsl::group_id.eq(gid)),
                    ))
                    .get_result::<bool>(conn)?)
            }
        })
        .await?;

        let group_exists = helper::run_query(&pool, {
            move |conn| {
                diesel::select(exists(groups_dsl::groups.find(gid))).get_result::<bool>(conn)
            }
        })
        .await?;

        if !group_exists {
            error!("Cannot generate invite: Group with ID {} not found.", gid);
            return Err(AppError::NotFound(format!(
                "Group with ID {} not found.",
                gid
            )));
        }

        if !group_permission_ok {
            warn!(
                "Permission denied: Instructor {} cannot generate invite for group {}.",
                instructor_id, gid
            );
            return Err(AppError::NotFound(
                "Instructor lacks permission for the specified group.".to_string(),
            ));
        }
        info!(
            "Instructor {} has permission for group {}",
            instructor_id, gid
        );
    } else {
        if instructor_id != 0 {
            warn!(
                "Permission denied: Instructor {} cannot generate invite without group context.",
                instructor_id
            );
            return Err(AppError::NotFound(
                "Instructor lacks permission to generate invite without group context.".to_string(),
            ));
        }
        info!("Admin instructor generating invite without group context.");
    }

    if let Some(gid) = game_id {
        let game_exists = helper::run_query(&pool, {
            move |conn| diesel::select(exists(games_dsl::games.find(gid))).get_result::<bool>(conn)
        })
        .await?;
        if !game_exists {
            error!("Cannot generate invite: Game with ID {} not found.", gid);
            return Err(AppError::NotFound(format!(
                "Game with ID {} not found.",
                gid
            )));
        }
        info!("Game {} confirmed to exist.", gid);
    }

    let new_uuid = Uuid::new_v4();
    info!("Generated new invite UUID: {}", new_uuid);

    let insert_result = helper::run_query(&pool, move |conn| {
        let new_invite = NewInvite {
            uuid: new_uuid,
            instructor_id,
            game_id,
            group_id,
        };

        diesel::insert_into(invites_dsl::invites)
            .values(&new_invite)
            .execute(conn)
    })
    .await;

    match insert_result {
        Ok(rows_affected) => {
            if rows_affected == 1 {
                info!(
                    "Successfully inserted invite record with UUID: {}",
                    new_uuid
                );
                let response_data = InviteLinkResponse {
                    invite_uuid: new_uuid,
                };
                Ok(ApiResponse::ok(response_data))
            } else {
                error!(
                    "Invite link generation failed: Insert query affected {} rows (expected 1) for UUID {}",
                    rows_affected, new_uuid
                );
                Err(AppError::InternalServerError(anyhow!(
                    "Database insert for invite link returned unexpected row count: {}",
                    rows_affected
                )))
            }
        }
        Err(AppError::InternalServerError(ref err)) => {
            if let Some(db_err) = err.downcast_ref::<DieselError>() {
                if let DieselError::DatabaseError(kind, info) = db_err {
                    return match kind {
                        DatabaseErrorKind::ForeignKeyViolation => {
                            warn!(
                                "Failed to insert invite link due to foreign key violation (UUID: {}). Details: {}",
                                new_uuid,
                                info.message()
                            );
                            Err(AppError::NotFound(format!(
                                "Referenced instructor, game, or group not found during invite creation (likely deleted concurrently). Details: {}",
                                info.message()
                            )))
                        }
                        _ => {
                            error!(
                                "Database error during invite link insertion (UUID: {}): {:?}",
                                new_uuid, err
                            );
                            Err(insert_result.unwrap_err())
                        }
                    };
                }
            }
            error!(
                "Unhandled internal server error during invite link insertion (UUID: {}): {:?}",
                new_uuid, err
            );
            Err(insert_result.unwrap_err())
        }
        Err(e) => {
            error!(
                "Unexpected error during invite link insertion (UUID: {}): {:?}",
                new_uuid, e
            );
            Err(e)
        }
    }
}

/// Processes an invite link for a specific player.
///
/// Finds the invite by UUID, validates the player exists, adds the player
/// to the associated game and/or group (if specified in the invite and not already present).
///
/// Request Body: `ProcessInviteLinkPayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the invite was successfully processed (200 OK).
/// * `404 Not Found`: If the invite UUID, player ID, or associated game/group ID (at time of use) is invalid.
/// * `500 Internal Server Error`: If a database error occurs.
#[instrument(skip(pool, payload))]
pub async fn process_invite_link(
    State(pool): State<Pool>,
    Json(payload): Json<ProcessInviteLinkPayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let player_id = payload.player_id;
    let invite_uuid = payload.uuid;
    info!(player_id, %invite_uuid, "[Handler] Received request to process invite link");

    pool
        .get()
        .await?
        .interact(move |conn| {
            info!("[Handler] Starting database transaction");
            conn.transaction::<_, DieselError, _>(|tx_conn| {
                info!(uuid = %invite_uuid, "[Handler Tx] Attempting to find invite by UUID");
                let invite = invites_dsl::invites
                    .filter(invites_dsl::uuid.eq(invite_uuid))
                    .get_result::<Invite>(tx_conn)
                    .map_err(|e| {
                        error!(uuid = %invite_uuid, error = %e, "[Handler Tx] Invite UUID query failed");
                        if matches!(e, DieselError::NotFound) {
                            DieselError::NotFound
                        } else {
                            e
                        }
                    })?;
                info!(invite_id = invite.id, "[Handler Tx] Invite found");

                debug!(player_id, "[Handler Tx] Validating player existence and status");
                let player_exists: bool = select(exists(
                    players_dsl::players
                        .filter(players_dsl::id.eq(player_id))
                        .filter(players_dsl::disabled.eq(false)),
                ))
                    .get_result(tx_conn)?;

                if !player_exists {
                    error!(player_id, "[Handler Tx] Player not found or is disabled");
                    return Err(DieselError::NotFound);
                }
                debug!(player_id, "[Handler Tx] Player validation successful");

                let target_game_id = invite.game_id;
                let target_group_id = invite.group_id;

                if let Some(game_id) = target_game_id {
                    info!(game_id, "[Handler Tx] Checking existence of associated game");
                    let game_exists: bool = select(exists(games_dsl::games.find(game_id)))
                        .get_result(tx_conn)?;
                    if !game_exists {
                        error!(game_id, "[Handler Tx] Associated game determined NOT FOUND during pre-check");
                        return Err(DieselError::NotFound);
                    }
                    info!(game_id, "[Handler Tx] Associated game determined FOUND during pre-check");
                }
                if let Some(group_id) = target_group_id {
                    info!(group_id, "[Handler Tx] Checking existence of associated group");
                    let group_exists: bool = select(exists(groups_dsl::groups.find(group_id)))
                        .get_result(tx_conn)?;
                    if !group_exists {
                        error!(group_id, "[Handler Tx] Associated group determined NOT FOUND during pre-check");
                        return Err(DieselError::NotFound);
                    }
                    info!(group_id, "[Handler Tx] Associated group determined FOUND during pre-check");
                }

                if let Some(game_id) = target_game_id {
                    info!(game_id, player_id, "[Handler Tx] Processing game association for invite");
                    let already_registered: bool = select(exists(
                        pr_dsl::player_registrations
                            .filter(pr_dsl::player_id.eq(player_id))
                            .filter(pr_dsl::game_id.eq(game_id))
                            .filter(pr_dsl::left_at.is_null()),
                    ))
                        .get_result(tx_conn)?;

                    if !already_registered {
                        info!(player_id, game_id, "[Handler Tx] Player not registered in game, adding registration");
                        let new_registration = NewPlayerRegistration {
                            player_id,
                            game_id,
                            language: "en".to_string(),
                            progress: 0,
                            game_state: json!({}),
                        };
                        diesel::insert_into(pr_dsl::player_registrations)
                            .values(&new_registration)
                            .execute(tx_conn)?;
                        info!(player_id, game_id, "[Handler Tx] Player successfully registered in game");
                    } else {
                        info!(player_id, game_id, "[Handler Tx] Player already registered in game, skipping registration");
                    }
                }

                if let Some(group_id) = target_group_id {
                    info!(group_id, player_id, "[Handler Tx] Processing group association for invite");
                    let already_member: bool = select(exists(
                        pg_dsl::player_groups
                            .filter(pg_dsl::player_id.eq(player_id))
                            .filter(pg_dsl::group_id.eq(group_id))
                            .filter(pg_dsl::left_at.is_null()),
                    ))
                        .get_result(tx_conn)?;

                    if !already_member {
                        info!(player_id, group_id, "[Handler Tx] Player not member of group, adding membership");
                        let new_player_group = NewPlayerGroup {
                            player_id,
                            group_id,
                        };
                        diesel::insert_into(pg_dsl::player_groups)
                            .values(&new_player_group)
                            .on_conflict((pg_dsl::player_id, pg_dsl::group_id))
                            .do_update()
                            .set(pg_dsl::left_at.eq(None::<chrono::NaiveDateTime>))
                            .execute(tx_conn)?;
                        info!(player_id, group_id, "[Handler Tx] Player successfully added to group");
                    } else {
                        info!(player_id, group_id, "[Handler Tx] Player already member of group, skipping membership update");
                    }
                }

                info!(uuid = %invite_uuid, player_id, "[Handler Tx] Invite processing completed successfully within transaction");
                Ok(())
            })
        })
        .await??;

    info!(player_id, %invite_uuid, "[Handler] Invite processed successfully, returning 200 OK");
    Ok(ApiResponse::ok(true))
}
