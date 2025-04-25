use diesel::{QueryDsl, RunQueryDsl};
use crate::errors::AppError;
use tracing::log::{debug, error, info, warn};
use crate::schema::{
    game_ownership::dsl as go_dsl,
    games::dsl as games_dsl,
    groups::dsl as groups_dsl,
    group_ownership::dsl as group_owner_dsl,
    courses::dsl as courses_dsl,
    course_ownership::dsl as course_owner_dsl
};
use diesel::ExpressionMethods;

pub(super) async fn run_query<T, F>(
    pool: &deadpool_diesel::postgres::Pool,
    query: F,
) -> Result<T, AppError>
where
    F: FnOnce(&mut diesel::PgConnection) -> Result<T, diesel::result::Error> + Send + 'static,
    T: Send + 'static,
{
    let conn = pool.get().await.map_err(|pool_err| {
        error!(
            "Failed to get DB connection object from pool: {:?}",
            pool_err
        );
        AppError::PoolError(pool_err)
    })?;
    debug!("DB connection object obtained from pool for interaction");

    let res = conn.interact(query).await;

    match res {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(diesel_err)) => {
            error!("Diesel query failed within interaction: {:?}", diesel_err);
            Err(AppError::DieselError(diesel_err))
        }
        Err(interact_err) => {
            error!("Deadpool interact error: {:?}", interact_err);
            Err(AppError::InteractError(interact_err))
        }
    }
}

/// Checks if an instructor has permission to access a specific game.
///
/// Permission is granted if:
/// 1. The `instructor_id` is 0 (representing an admin user) AND the `game_id` exists.
/// 2. A record exists in the `game_ownership` table linking the non-admin `instructor_id` and the `game_id`.
///
/// Returns `Ok(())` if permission is granted.
/// Returns `Err(AppError::NotFound)` if permission is denied, or if the instructor/game
/// relevant to the permission check does not exist.
/// Returns other `AppError` variants for database pool or query errors.
///
/// # Arguments
/// * `pool` - The database connection pool.
/// * `instructor_id` - The ID of the instructor requesting access.
/// * `game_id` - The ID of the game being accessed.
pub async fn check_instructor_game_permission(
    pool: &deadpool_diesel::postgres::Pool,
    instructor_id: i64,
    game_id: i64,
) -> Result<(), AppError> {
    info!(
        "Checking permission for instructor_id: {} on game_id: {}",
        instructor_id, game_id
    );

    if instructor_id == 0 { // admin check
        info!("Admin instructor (ID 0) detected. Checking game existence.");
        let game_exists = run_query(pool, move |conn| {
            diesel::select(diesel::dsl::exists(games_dsl::games.find(game_id)))
                .get_result::<bool>(conn)
        }).await?;

        if game_exists {
            info!("Admin permission granted for existing game_id: {}", game_id);
            Ok(())
        } else {
            error!(
                "Admin permission check failed: Game with ID {} not found.",
                game_id
            );
            Err(AppError::NotFound(format!(
                "Game with ID {} not found.",
                game_id
            )))
        }
    } else { // non admin check
        info!("Non-admin instructor. Checking game_ownership table.");
        let ownership_exists = run_query(pool, move |conn| {
            diesel::select(diesel::dsl::exists(
                go_dsl::game_ownership
                    .filter(go_dsl::instructor_id.eq(instructor_id))
                    .filter(go_dsl::game_id.eq(game_id)),
            )).get_result::<bool>(conn)
        }).await?;

        if ownership_exists {
            info!(
                "Permission granted via game_ownership for instructor_id: {} on game_id: {}",
                instructor_id, game_id
            );
            Ok(())
        } else {
            warn!(
                "Permission denied for instructor_id: {}. No matching record in game_ownership for game_id: {}.",
                instructor_id, game_id
            );
            Err(AppError::NotFound(format!(
                "Instructor {} does not have permission for game {}, or one/both entities do not exist.",
                instructor_id, game_id
            )))
        }
    }
}

/// Checks if an instructor has permission to manage a specific group.
///
/// Permission is granted if:
/// 1. The `instructor_id` is 0 (representing an admin user) AND the `group_id` exists.
/// 2. A record exists in the `group_ownership` table linking the non-admin `instructor_id`
///    and the `group_id` where `owner` is true.
///
/// Returns `Ok(())` if permission is granted.
/// Returns `Err(AppError::NotFound)` if permission is denied, or if the instructor/group
/// relevant to the permission check does not exist.
/// Returns other `AppError` variants for database pool or query errors.
///
/// # Arguments
/// * `pool` - The database connection pool.
/// * `instructor_id` - The ID of the instructor requesting access.
/// * `group_id` - The ID of the group being managed.
pub async fn check_instructor_group_permission(
    pool: &deadpool_diesel::postgres::Pool,
    instructor_id: i64,
    group_id: i64,
) -> Result<(), AppError> {
    info!(
        "Checking permission for instructor_id: {} on group_id: {}",
        instructor_id, group_id
    );

    if instructor_id == 0 {
        info!("Admin instructor (ID 0) detected. Checking group existence.");
        let group_exists = run_query(pool, move |conn| {
            diesel::select(diesel::dsl::exists(groups_dsl::groups.find(group_id)))
                .get_result::<bool>(conn)
        })
            .await?;

        if group_exists {
            info!("Admin permission granted for existing group_id: {}", group_id);
            Ok(())
        } else {
            error!(
                "Admin permission check failed: Group with ID {} not found.",
                group_id
            );
            Err(AppError::NotFound(format!(
                "Group with ID {} not found.",
                group_id
            )))
        }
    } else {
        info!("Non-admin instructor. Checking group_ownership table for ownership.");
        let is_owner = run_query(pool, move |conn| {
            diesel::select(diesel::dsl::exists(
                group_owner_dsl::group_ownership
                    .filter(group_owner_dsl::instructor_id.eq(instructor_id))
                    .filter(group_owner_dsl::group_id.eq(group_id))
                    .filter(group_owner_dsl::owner.eq(true)),
            )).get_result::<bool>(conn)
        }).await?;

        if is_owner {
            info!(
                "Permission granted via group_ownership (owner=true) for instructor_id: {} on group_id: {}",
                instructor_id, group_id
            );
            Ok(())
        } else {
            warn!(
                "Permission denied for instructor_id: {}. Not an owner in group_ownership for group_id: {}.",
                instructor_id, group_id
            );
            Err(AppError::NotFound(format!(
                "Instructor {} does not have owner permission for group {}, or one/both entities do not exist.",
                instructor_id, group_id
            )))
        }
    }
}

/// Checks if an instructor has owner permission for a specific course.
///
/// Permission is granted if:
/// 1. The `instructor_id` is 0 (representing an admin user) AND the `course_id` exists.
/// 2. A record exists in the `course_ownership` table linking the non-admin `instructor_id`
///    and the `course_id` where `owner` is true.
///
/// Returns `Ok(())` if permission is granted.
/// Returns `Err(AppError::NotFound)` if permission is denied, or if the instructor/course
/// relevant to the permission check does not exist.
/// Returns other `AppError` variants for database pool or query errors.
pub async fn check_instructor_course_permission(
    pool: &deadpool_diesel::postgres::Pool,
    instructor_id: i64,
    course_id: i64,
) -> Result<(), AppError> {
    info!(
        "Checking owner permission for instructor_id: {} on course_id: {}",
        instructor_id, course_id
    );

    if instructor_id == 0 {
        info!("Admin instructor (ID 0) detected. Checking course existence.");
        let course_exists = run_query(pool, move |conn| {
            diesel::select(diesel::dsl::exists(courses_dsl::courses.find(course_id)))
                .get_result::<bool>(conn)
        }).await?;

        if course_exists {
            info!("Admin permission granted for existing course_id: {}", course_id);
            Ok(())
        } else {
            error!(
                "Admin permission check failed: Course with ID {} not found.",
                course_id
            );
            Err(AppError::NotFound(format!(
                "Course with ID {} not found.",
                course_id
            )))
        }
    } else {
        info!("Non-admin instructor. Checking course_ownership table for ownership.");
        let is_owner = run_query(pool, move |conn| {
            diesel::select(diesel::dsl::exists(
                course_owner_dsl::course_ownership
                    .filter(course_owner_dsl::instructor_id.eq(instructor_id))
                    .filter(course_owner_dsl::course_id.eq(course_id))
                    .filter(course_owner_dsl::owner.eq(true)),
            ))
                .get_result::<bool>(conn)
        }).await?;

        if is_owner {
            info!(
                "Permission granted via course_ownership (owner=true) for instructor_id: {} on course_id: {}",
                instructor_id, course_id
            );
            Ok(())
        } else {
            warn!(
                "Permission denied for instructor_id: {}. Not an owner in course_ownership for course_id: {}.",
                instructor_id, course_id
            );
            Err(AppError::NotFound(format!(
                "Instructor {} does not have owner permission for course {}, or one/both entities do not exist.",
                instructor_id, course_id
            )))
        }
    }
}