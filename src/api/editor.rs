use std::collections::HashMap;
use anyhow::anyhow;
use axum::extract::{Query, State};
use axum::Json;
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, Duration, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::exists;
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use diesel::result::Error as DieselError;
use tracing::instrument;
use tracing::log::{debug, error, info, warn};
use crate::errors::AppError;
use crate::payloads::editor::{ExportCourseParams, ImportCoursePayload};
use crate::response::ApiResponse;
use crate::{
    schema::{
        courses::dsl as courses_dsl,
        course_ownership::dsl as course_owner_dsl,
        modules::dsl as modules_dsl,
        exercises::dsl as exercises_dsl,
        instructors::dsl as instructors_dsl,
    },
};
use crate::model::editor::{CourseQueryResult, ExerciseQueryResult, ExportCourseResponse, ExportExerciseResponse, ExportModuleResponse, ModuleQueryResult, NewCourse, NewCourseOwnership, NewExercise, NewModule};

/// Imports a complete course structure from JSON data.
///
/// Creates course, modules, and exercises based on the provided payload.
/// Assigns ownership of the new course to the requesting instructor.
/// Requires the requesting instructor to exist.
/// Performs all database operations within a single transaction.
///
/// Request Body: `ImportCoursePayload`
///
/// Returns (wrapped in `ApiResponse`)
/// * `bool`: true if the course was successfully imported (200 OK).
/// * `404 Not Found`: If the requesting instructor specified in the payload does not exist.
/// * `500 Internal Server Error`: If a database error (pool, interaction, query) or transaction failure occurs.
#[instrument(skip(pool, payload))]
pub async fn import_course(
    State(pool): State<Pool>,
    Json(payload): Json<ImportCoursePayload>,
) -> Result<ApiResponse<bool>, AppError> {
    let instructor_id = payload.instructor_id;
    let course_title = payload.course_data.title.clone();

    info!(
        "Attempting to import course '{}' requested by instructor {}",
        course_title, instructor_id
    );
    debug!("Import course payload: {:?}", payload);

    let instructor_exists = super::helper::run_query(&pool, {
        let instructor_id = instructor_id;
        move |conn| {
            diesel::select(exists(instructors_dsl::instructors.find(instructor_id)))
                .get_result::<bool>(conn)
        }
    }).await?;

    if !instructor_exists {
        error!(
            "Cannot import course: Requesting instructor with ID {} not found.",
            instructor_id
        );
        return Err(AppError::NotFound(format!(
            "Requesting instructor with ID {} not found.",
            instructor_id
        )));
    }
    info!("Requesting instructor {} confirmed to exist.", instructor_id);

    let conn = pool.get().await?;
    let import_result = conn
        .interact(move |conn_sync| {
            conn_sync.transaction(|tx_conn| {
                let course_data = payload.course_data;
                let new_course = NewCourse {
                    title: course_data.title,
                    description: course_data.description,
                    languages: course_data.languages,
                    programming_languages: course_data.programming_languages,
                    gamification_rule_conditions: course_data.gamification_rule_conditions,
                    gamification_complex_rules: course_data.gamification_complex_rules,
                    gamification_rule_results: course_data.gamification_rule_results,
                    public: payload.public,
                };
                let new_course_id = diesel::insert_into(courses_dsl::courses)
                    .values(&new_course)
                    .returning(courses_dsl::id)
                    .get_result::<i64>(tx_conn)?;
                info!("Inserted course with ID: {}", new_course_id);

                let new_ownership = NewCourseOwnership {
                    course_id: new_course_id,
                    instructor_id: payload.instructor_id,
                    owner: true,
                };
                diesel::insert_into(course_owner_dsl::course_ownership)
                    .values(&new_ownership)
                    .execute(tx_conn)?;
                info!("Inserted course ownership for instructor {}", payload.instructor_id);

                let now = Utc::now();
                let default_end_date = now + Duration::days(365);

                for module_data in course_data.modules {
                    let new_module = NewModule {
                        course_id: new_course_id,
                        order: module_data.order,
                        title: module_data.title,
                        description: module_data.description,
                        language: module_data.language,
                        start_date: module_data.start_date.unwrap_or(now),
                        end_date: module_data.end_date.unwrap_or(default_end_date),
                    };
                    let new_module_id = diesel::insert_into(modules_dsl::modules)
                        .values(&new_module)
                        .returning(modules_dsl::id)
                        .get_result::<i64>(tx_conn)?;
                    info!("Inserted module '{}' with ID: {}", new_module.title, new_module_id);

                    for exercise_data in module_data.exercises {
                        let new_exercise = NewExercise {
                            version: BigDecimal::from_f64(1.0).unwrap_or_else(|| BigDecimal::from(1)),
                            module_id: new_module_id,
                            order: exercise_data.order,
                            title: exercise_data.title,
                            description: exercise_data.description,
                            language: exercise_data.language,
                            programming_language: exercise_data.programming_language,
                            init_code: exercise_data.init_code,
                            pre_code: exercise_data.pre_code,
                            post_code: exercise_data.post_code,
                            test_code: exercise_data.test_code,
                            check_source: exercise_data.check_source,
                            hidden: exercise_data.hidden,
                            locked: exercise_data.locked,
                            mode: exercise_data.mode,
                            mode_parameters: exercise_data.mode_parameters,
                            difficulty: exercise_data.difficulty,
                        };
                        diesel::insert_into(exercises_dsl::exercises)
                            .values(&new_exercise)
                            .execute(tx_conn)?;
                    }
                    info!("Inserted exercises for module ID {}", new_module_id);
                }
                Ok::<(), DieselError>(())
            })
        })
        .await?;

    match import_result {
        Ok(()) => {
            info!(
                "Successfully imported course '{}' for instructor {}",
                course_title, instructor_id
            );
            Ok(ApiResponse::ok(true))
        }
        Err(diesel_err) => {
            error!("Course import transaction failed: {:?}", diesel_err);
            Err(AppError::from(diesel_err))
        }
    }
}


/// Exports the full structure of a course (details, modules, exercises) as JSON.
///
/// Requires the requesting instructor to be an owner of the course or an admin (ID 0).
/// Fetches all relevant data from the database via separate queries and structures it hierarchically.
///
/// Query Parameters:
/// * instructor_id as `i64`: The ID of the instructor requesting the export.
/// * course_id as `i64`: The ID of the course to export.
///
/// Returns (wrapped in `ApiResponse`)
/// * `ExportCourseResponse`: The structured course data (200 OK).
/// * `403 Forbidden`: If the requesting instructor does not have ownership permission for the specified course.
/// * `404 Not Found`: If the specified course does not exist.
/// * `500 Internal Server Error`: If a database error (pool, interaction, query) occurs during permission checks or data fetching.
#[instrument(skip(pool, params))]
pub async fn export_course(
    State(pool): State<Pool>,
    Query(params): Query<ExportCourseParams>,
) -> Result<ApiResponse<ExportCourseResponse>, AppError> {
    let instructor_id = params.instructor_id;
    let course_id = params.course_id;

    info!(
        "Attempting to export course {} requested by instructor {}",
        course_id, instructor_id
    );
    debug!("Export course params: {:?}", params);

    super::helper::check_instructor_course_permission(&pool, instructor_id, course_id).await?;
    info!(
        "Permission check passed for instructor {} on course {}",
        instructor_id, course_id
    );

    let course_details = super::helper::run_query(&pool, {
        move |conn| {
            courses_dsl::courses
                .find(course_id)
                .select((
                    courses_dsl::id,
                    courses_dsl::title,
                    courses_dsl::description,
                    courses_dsl::languages,
                    courses_dsl::programming_languages,
                    courses_dsl::gamification_rule_conditions,
                    courses_dsl::gamification_complex_rules,
                    courses_dsl::gamification_rule_results,
                ))
                .first::<CourseQueryResult>(conn)
        }
    })
        .await?;

    let modules_db = super::helper::run_query(&pool, {
        move |conn| {
            modules_dsl::modules
                .filter(modules_dsl::course_id.eq(course_id))
                .order_by(modules_dsl::order.asc())
                .load::<ModuleQueryResult>(conn)
        }
    })
        .await?;
    info!("Fetched {} modules for course {}", modules_db.len(), course_id);

    let module_ids: Vec<i64> = modules_db.iter().map(|m| m.id).collect();
    let exercises_db = if !module_ids.is_empty() {
        super::helper::run_query(&pool, {
            move |conn| {
                exercises_dsl::exercises
                    .filter(exercises_dsl::module_id.eq_any(&module_ids))
                    .select((
                        exercises_dsl::id,
                        exercises_dsl::module_id,
                        exercises_dsl::order,
                        exercises_dsl::title,
                        exercises_dsl::description,
                        exercises_dsl::language,
                        exercises_dsl::programming_language,
                        exercises_dsl::init_code,
                        exercises_dsl::pre_code,
                        exercises_dsl::post_code,
                        exercises_dsl::test_code,
                        exercises_dsl::check_source,
                        exercises_dsl::hidden,
                        exercises_dsl::locked,
                        exercises_dsl::mode,
                        exercises_dsl::mode_parameters,
                        exercises_dsl::difficulty,
                    ))
                    .order_by((exercises_dsl::module_id, exercises_dsl::order.asc()))
                    .load::<ExerciseQueryResult>(conn)
            }
        })
            .await?
    } else {
        Vec::new()
    };
    info!("Fetched {} exercises for course {}", exercises_db.len(), course_id);

    let mut exercises_by_module: HashMap<i64, Vec<ExportExerciseResponse>> = HashMap::new();
    for ex_query_res in exercises_db {
        let ex_response = ExportExerciseResponse {
            order: ex_query_res.order,
            title: ex_query_res.title,
            description: ex_query_res.description,
            language: ex_query_res.language,
            programming_language: ex_query_res.programming_language,
            init_code: ex_query_res.init_code,
            pre_code: ex_query_res.pre_code,
            post_code: ex_query_res.post_code,
            test_code: ex_query_res.test_code,
            check_source: ex_query_res.check_source,
            hidden: ex_query_res.hidden,
            locked: ex_query_res.locked,
            mode: ex_query_res.mode,
            mode_parameters: ex_query_res.mode_parameters,
            difficulty: ex_query_res.difficulty,
        };
        exercises_by_module
            .entry(ex_query_res.module_id)
            .or_default()
            .push(ex_response);
    }

    let assembled_modules: Vec<ExportModuleResponse> = modules_db
        .into_iter()
        .map(|mod_query_res| {
            let exercises = exercises_by_module
                .remove(&mod_query_res.id)
                .unwrap_or_default();
            ExportModuleResponse {
                order: mod_query_res.order,
                title: mod_query_res.title,
                description: mod_query_res.description,
                language: mod_query_res.language,
                start_date: mod_query_res.start_date,
                end_date: mod_query_res.end_date,
                exercises,
            }
        })
        .collect();

    let final_response = ExportCourseResponse {
        title: course_details.title,
        description: course_details.description,
        languages: course_details.languages,
        programming_languages: course_details.programming_languages,
        gamification_rule_conditions: course_details.gamification_rule_conditions,
        gamification_complex_rules: course_details.gamification_complex_rules,
        gamification_rule_results: course_details.gamification_rule_results,
        modules: assembled_modules,
    };

    info!(
        "Successfully prepared export data for course {}",
        course_id
    );
    Ok(ApiResponse::ok(final_response))
}