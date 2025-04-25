use diesel::{QueryDsl, RunQueryDsl, PgConnection, SelectableHelper};
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
use deadpool_diesel::postgres::Pool;
use diesel::dsl::exists;

pub(super) async fn run_query<T, F>(
    pool: &Pool,
    query: F,
) -> Result<T, AppError>
where
    F: FnOnce(&mut PgConnection) -> Result<T, diesel::result::Error> + Send + 'static,
    T: Send + 'static,
{
    let conn = pool.get().await?;
    debug!("DB connection object obtained from pool for interaction");

    let result = conn.interact(query).await?;

    result.map_err(AppError::from)
}

/// Checks if an instructor has permission for a specific entity.
/// Distinguishes between the entity not existing (404) and permission being denied (403).
/// Admin instructor (ID 0) gets access if the entity exists.
async fn check_permission_generic<CheckExistence, CheckPermission>(
    pool: &Pool,
    instructor_id: i64,
    entity_id: i64,
    entity_name: &str,
    existence_check: CheckExistence,
    permission_check: CheckPermission,
) -> Result<(), AppError>
where
    CheckExistence: FnOnce(i64, &mut PgConnection) -> Result<bool, diesel::result::Error> + Send + 'static,
    CheckPermission: FnOnce(i64, i64, &mut PgConnection) -> Result<bool, diesel::result::Error> + Send + 'static,
{
    info!(
        "Checking existence and permission for instructor_id: {} on {}_id: {}",
        instructor_id, entity_name, entity_id
    );

    let entity_exists = run_query(pool, {
        let entity_id_for_closure = entity_id;
        move |conn| existence_check(entity_id_for_closure, conn)
    }).await?;

    if !entity_exists {
        error!(
            "Permission check failed: {} with ID {} not found.",
            entity_name, entity_id
        );
        return Err(AppError::NotFound(format!(
            "{} with ID {} not found.",
            entity_name, entity_id
        )));
    }
    info!("{} with ID {} confirmed to exist.", entity_name, entity_id);

    if instructor_id == 0 {
        info!("Admin permission granted for existing {}_id: {}", entity_name, entity_id);
        Ok(())
    } else {
        info!("Non-admin instructor. Checking {} ownership/permission.", entity_name);
        let has_permission = run_query(pool, {
            let instructor_id_for_closure = instructor_id;
            let entity_id_for_closure = entity_id;
            move |conn| permission_check(instructor_id_for_closure, entity_id_for_closure, conn)
        }).await?;

        if has_permission {
            info!(
                "Permission granted via ownership for instructor_id: {} on {}_id: {}",
                instructor_id, entity_name, entity_id
            );
            Ok(())
        } else {
            warn!(
                "Permission denied for instructor_id: {} on existing {}_id: {}.",
                instructor_id, entity_name, entity_id
            );
            Err(AppError::Forbidden(format!(
                "Instructor {} does not have permission for {} {}.",
                instructor_id, entity_name, entity_id
            )))
        }
    }
}


/// Checks if an instructor has permission for a game.
/// Returns Ok(()) if permission granted.
/// Returns AppError::NotFound if the game doesn't exist.
/// Returns AppError::Forbidden if the instructor lacks permission for an existing game.
/// Returns AppError::InternalServerError for database issues.
pub async fn check_instructor_game_permission(
    pool: &Pool,
    instructor_id: i64,
    game_id: i64,
) -> Result<(), AppError> {
    check_permission_generic(
        pool,
        instructor_id,
        game_id,
        "game",
        |id, conn| diesel::select(exists(games_dsl::games.find(id))).get_result::<bool>(conn),
        |instr_id, ent_id, conn| diesel::select(exists(
            go_dsl::game_ownership
                .filter(go_dsl::instructor_id.eq(instr_id))
                .filter(go_dsl::game_id.eq(ent_id)),
        )).get_result::<bool>(conn)
    ).await
}

/// Checks if an instructor has owner permission for a group.
/// Returns Ok(()) if permission granted.
/// Returns AppError::NotFound if the group doesn't exist.
/// Returns AppError::Forbidden if the instructor lacks owner permission for an existing group.
/// Returns AppError::InternalServerError for database issues.
pub async fn check_instructor_group_permission(
    pool: &Pool,
    instructor_id: i64,
    group_id: i64,
) -> Result<(), AppError> {
    check_permission_generic(
        pool,
        instructor_id,
        group_id,
        "group",
        |id, conn| diesel::select(exists(groups_dsl::groups.find(id))).get_result::<bool>(conn),
        |instr_id, ent_id, conn| diesel::select(exists(
            group_owner_dsl::group_ownership
                .filter(group_owner_dsl::instructor_id.eq(instr_id))
                .filter(group_owner_dsl::group_id.eq(ent_id))
                .filter(group_owner_dsl::owner.eq(true)),
        )).get_result::<bool>(conn)
    ).await
}

/// Checks if an instructor has owner permission for a course.
/// Returns Ok(()) if permission granted.
/// Returns AppError::NotFound if the course doesn't exist.
/// Returns AppError::Forbidden if the instructor lacks owner permission for an existing course.
/// Returns AppError::InternalServerError for database issues.
pub async fn check_instructor_course_permission(
    pool: &Pool,
    instructor_id: i64,
    course_id: i64,
) -> Result<(), AppError> {
    check_permission_generic(
        pool,
        instructor_id,
        course_id,
        "course",
        |id, conn| diesel::select(exists(courses_dsl::courses.find(id))).get_result::<bool>(conn),
        |instr_id, ent_id, conn| diesel::select(exists(
            course_owner_dsl::course_ownership
                .filter(course_owner_dsl::instructor_id.eq(instr_id))
                .filter(course_owner_dsl::course_id.eq(ent_id))
                .filter(course_owner_dsl::owner.eq(true)),
        )).get_result::<bool>(conn)
    ).await
}