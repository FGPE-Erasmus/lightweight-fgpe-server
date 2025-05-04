use axum::http::StatusCode;
use bigdecimal::BigDecimal;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};
use lightweight_fgpe_server::model::editor::ExportCourseResponse;
use lightweight_fgpe_server::payloads::editor::{
    ImportCourseData, ImportCoursePayload, ImportExerciseData, ImportModuleData,
};
use lightweight_fgpe_server::response::ApiResponse;
use serde_json::{Value, json};

mod helpers;
use helpers::{
    check_course_ownership, count_courses, count_exercises_for_module, count_modules_for_course,
    create_test_course, create_test_course_ownership, create_test_exercise, create_test_instructor,
    create_test_module, setup_test_environment,
};

// import_course

fn create_valid_import_payload(instructor_id: i64) -> ImportCoursePayload {
    ImportCoursePayload {
        instructor_id,
        public: false,
        course_data: ImportCourseData {
            title: "Imported Course".to_string(),
            description: "A course imported via test".to_string(),
            languages: "en,fr".to_string(),
            programming_languages: "py,rs".to_string(),
            gamification_rule_conditions: "{}".to_string(),
            gamification_complex_rules: "{}".to_string(),
            gamification_rule_results: "{}".to_string(),
            modules: vec![
                ImportModuleData {
                    order: 1,
                    title: "Module 1".to_string(),
                    description: "First module".to_string(),
                    language: "en".to_string(),
                    start_date: None,
                    end_date: None,
                    exercises: vec![ImportExerciseData {
                        _version: BigDecimal::from(1),
                        order: 1,
                        title: "Exercise 1.1".to_string(),
                        description: "First exercise".to_string(),
                        language: "en".to_string(),
                        programming_language: "py".to_string(),
                        init_code: "init()".to_string(),
                        pre_code: "pre()".to_string(),
                        post_code: "post()".to_string(),
                        test_code: "test()".to_string(),
                        check_source: "check()".to_string(),
                        hidden: false,
                        locked: false,
                        mode: "code".to_string(),
                        mode_parameters: json!({"param": "value"}),
                        difficulty: "easy".to_string(),
                    }],
                },
                ImportModuleData {
                    order: 2,
                    title: "Module 2".to_string(),
                    description: "Second module".to_string(),
                    language: "fr".to_string(),
                    start_date: None,
                    end_date: None,
                    exercises: vec![],
                },
            ],
        },
    }
}

#[tokio::test]
async fn test_import_course_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 1;
    create_test_instructor(&pool, instructor_id, "importer@test.com", "Importer").await;

    let initial_course_count = count_courses(&pool).await;
    let payload = create_valid_import_payload(instructor_id);

    let response = server.post("/editor/import_course").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.unwrap_or(false));

    assert_eq!(
        count_courses(&pool).await,
        initial_course_count + 1,
        "Course count should increase by 1"
    );

    let conn = pool.get().await.unwrap();
    let new_course_id_opt: Option<i64> = conn
        .interact(move |conn| {
            use lightweight_fgpe_server::schema::courses::dsl::*;
            courses
                .filter(title.eq("Imported Course"))
                .select(id)
                .first::<i64>(conn)
                .optional()
        })
        .await
        .unwrap()
        .unwrap();

    assert!(
        new_course_id_opt.is_some(),
        "Failed to find the imported course by title"
    );
    let new_course_id = new_course_id_opt.unwrap();

    assert!(
        check_course_ownership(&pool, instructor_id, new_course_id).await,
        "Instructor should own the new course"
    );
    assert_eq!(
        count_modules_for_course(&pool, new_course_id).await,
        2,
        "Should have imported 2 modules"
    );

    let module_ids: Vec<i64> = conn
        .interact(move |conn| {
            use lightweight_fgpe_server::schema::modules::dsl::*;
            modules
                .filter(course_id.eq(new_course_id))
                .order_by(order.asc())
                .select(id)
                .load::<i64>(conn)
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(module_ids.len(), 2, "Should find 2 module IDs");
    assert_eq!(
        count_exercises_for_module(&pool, module_ids[0]).await,
        1,
        "Module 1 should have 1 exercise"
    );
    assert_eq!(
        count_exercises_for_module(&pool, module_ids[1]).await,
        0,
        "Module 2 should have 0 exercises"
    );
}

#[tokio::test]
async fn test_import_course_instructor_not_found() {
    let (server, pool) = setup_test_environment().await;
    let non_existent_instructor_id = 99;

    let initial_course_count = count_courses(&pool).await;
    let payload = create_valid_import_payload(non_existent_instructor_id);

    let response = server.post("/editor/import_course").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(
        body.status_message
            .contains("Requesting instructor with ID 99 not found")
    );

    assert_eq!(
        count_courses(&pool).await,
        initial_course_count,
        "Course count should not change"
    );
}

#[tokio::test]
async fn test_import_course_minimal_payload() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 2;
    create_test_instructor(
        &pool,
        instructor_id,
        "importer_min@test.com",
        "Importer Min",
    )
    .await;

    let payload = ImportCoursePayload {
        instructor_id,
        public: true,
        course_data: ImportCourseData {
            title: "Minimal Course".to_string(),
            description: "".to_string(),
            languages: "".to_string(),
            programming_languages: "".to_string(),
            gamification_rule_conditions: "".to_string(),
            gamification_complex_rules: "".to_string(),
            gamification_rule_results: "".to_string(),
            modules: vec![],
        },
    };

    let response = server.post("/editor/import_course").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));

    let conn = pool.get().await.unwrap();
    let (new_course_id, is_public): (i64, bool) = conn
        .interact(move |conn| {
            use lightweight_fgpe_server::schema::courses::dsl::*;
            courses
                .filter(title.eq("Minimal Course"))
                .select((id, public))
                .first::<(i64, bool)>(conn)
        })
        .await
        .unwrap()
        .unwrap();

    assert!(check_course_ownership(&pool, instructor_id, new_course_id).await);
    assert!(is_public, "Course should be public");
    assert_eq!(count_modules_for_course(&pool, new_course_id).await, 0);
}

// export_course

#[tokio::test]
async fn test_export_course_success_owner() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 10;
    let course_id = create_test_course(&pool, "Export Course").await;
    let module1_id = create_test_module(&pool, course_id, 1, "Export Mod 1").await;
    let module2_id = create_test_module(&pool, course_id, 2, "Export Mod 2").await;
    let ex1_id = create_test_exercise(&pool, module1_id, 1, "Export Ex 1.1").await;
    let ex2_id = create_test_exercise(&pool, module1_id, 2, "Export Ex 1.2").await;

    create_test_instructor(&pool, instructor_id, "exporter@test.com", "Exporter").await;
    create_test_course_ownership(&pool, instructor_id, course_id, true).await;

    let response = server
        .get(&format!(
            "/editor/export_course?instructor_id={}&course_id={}",
            instructor_id, course_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExportCourseResponse> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());

    let export_data = body.data.unwrap();
    assert_eq!(export_data.title, "Export Course");
    assert_eq!(export_data.modules.len(), 2);
    assert_eq!(export_data.modules[0].title, "Export Mod 1");
    assert_eq!(export_data.modules[0].exercises.len(), 2);
    assert_eq!(export_data.modules[0].exercises[0].title, "Export Ex 1.1");
    assert_eq!(export_data.modules[0].exercises[1].title, "Export Ex 1.2");
    assert_eq!(export_data.modules[1].title, "Export Mod 2");
    assert_eq!(export_data.modules[1].exercises.len(), 0);
}

#[tokio::test]
async fn test_export_course_success_admin() {
    let (server, pool) = setup_test_environment().await;
    let admin_instructor_id = 0;
    let course_id = create_test_course(&pool, "Export Course Admin").await;
    create_test_module(&pool, course_id, 1, "Export Mod Admin").await;

    create_test_instructor(&pool, admin_instructor_id, "admin@test.com", "Admin").await;

    let response = server
        .get(&format!(
            "/editor/export_course?instructor_id={}&course_id={}",
            admin_instructor_id, course_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExportCourseResponse> = response.json();
    assert!(body.data.is_some());
    assert_eq!(body.data.unwrap().title, "Export Course Admin");
}

#[tokio::test]
async fn test_export_course_success_no_modules() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 11;
    let course_id = create_test_course(&pool, "Export Course No Modules").await;

    create_test_instructor(&pool, instructor_id, "exporter_nm@test.com", "Exporter NM").await;
    create_test_course_ownership(&pool, instructor_id, course_id, true).await;

    let response = server
        .get(&format!(
            "/editor/export_course?instructor_id={}&course_id={}",
            instructor_id, course_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExportCourseResponse> = response.json();
    assert!(body.data.is_some());
    let export_data = body.data.unwrap();
    assert_eq!(export_data.title, "Export Course No Modules");
    assert!(export_data.modules.is_empty());
}

#[tokio::test]
async fn test_export_course_forbidden_non_owner() {
    let (server, pool) = setup_test_environment().await;
    let owner_instructor_id = 12;
    let non_owner_instructor_id = 13;
    let course_id = create_test_course(&pool, "Export Course Forbidden").await;

    create_test_instructor(
        &pool,
        owner_instructor_id,
        "exporter_o@test.com",
        "Exporter O",
    )
    .await;
    create_test_instructor(
        &pool,
        non_owner_instructor_id,
        "exporter_f@test.com",
        "Exporter F",
    )
    .await;
    create_test_course_ownership(&pool, owner_instructor_id, course_id, true).await;

    let response = server
        .get(&format!(
            "/editor/export_course?instructor_id={}&course_id={}",
            non_owner_instructor_id, course_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 403);
    assert!(
        body.status_message
            .contains("does not have permission for course")
    );
}

#[tokio::test]
async fn test_export_course_not_found_course() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 14;
    let non_existent_course_id = 999;

    create_test_instructor(&pool, instructor_id, "exporter_nf@test.com", "Exporter NF").await;

    let response = server
        .get(&format!(
            "/editor/export_course?instructor_id={}&course_id={}",
            instructor_id, non_existent_course_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(body.status_message.contains("course with ID 999 not found"));
}

#[tokio::test]
async fn test_export_course_bad_request_missing_param() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 15;
    create_test_instructor(&pool, instructor_id, "exporter_br@test.com", "Exporter BR").await;

    let response = server
        .get(&format!(
            "/editor/export_course?instructor_id={}",
            instructor_id
        ))
        .await;
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let course_id = create_test_course(&pool, "Dummy Course BR").await;
    let response2 = server
        .get(&format!("/editor/export_course?course_id={}", course_id))
        .await;
    assert_eq!(response2.status_code(), StatusCode::BAD_REQUEST);
}
