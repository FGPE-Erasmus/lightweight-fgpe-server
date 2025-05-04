use axum::http::StatusCode;
use diesel::ExpressionMethods;
use diesel::{QueryDsl, RunQueryDsl};
use float_cmp::approx_eq;
use lightweight_fgpe_server::model::teacher::{
    ExerciseStatsResponse, InstructorGameMetadataResponse, InviteLinkResponse,
    StudentExercisesResponse, StudentProgressResponse, SubmissionDataResponse,
};
use lightweight_fgpe_server::payloads::teacher::{
    ActivateGamePayload, AddGameInstructorPayload, AddGroupMemberPayload, CreateGamePayload,
    CreateGroupPayload, CreatePlayerPayload, DeletePlayerPayload, DisablePlayerPayload,
    DissolveGroupPayload, GenerateInviteLinkPayload, ModifyGamePayload, ProcessInviteLinkPayload,
    RemoveGameInstructorPayload, RemoveGameStudentPayload, RemoveGroupMemberPayload,
    StopGamePayload,
};
use lightweight_fgpe_server::response::ApiResponse;
use serde_json::{Value, json};
use uuid::Uuid;

mod helpers;
use crate::helpers::{
    check_player_in_game, check_player_in_group, count_player_game_registrations,
    count_player_group_memberships,
};
use helpers::{
    add_player_to_group, create_test_course, create_test_exercise, create_test_game,
    create_test_game_ownership, create_test_group_ownership, create_test_group_with_id,
    create_test_instructor, create_test_invite, create_test_module, create_test_player,
    create_test_player_registration, create_test_submission, setup_test_environment,
    update_player_status,
};
use lightweight_fgpe_server::schema;

// get_instructor_games

#[tokio::test]
async fn test_get_instructor_games_success_multiple_games() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 1001;
    let course_id = create_test_course(&pool, "Test Course 1").await;
    let game_id1 = create_test_game(&pool, course_id, "Game 1", 0).await;
    let game_id2 = create_test_game(&pool, course_id, "Game 2", 0).await;
    let _other_game = create_test_game(&pool, course_id, "Other Game", 0).await;

    create_test_instructor(&pool, instructor_id, "teacher1@test.com", "Teacher One").await;
    create_test_game_ownership(&pool, instructor_id, game_id1, true).await;
    create_test_game_ownership(&pool, instructor_id, game_id2, false).await;

    let response = server
        .get(&format!(
            "/teacher/get_instructor_games?instructor_id={}",
            instructor_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: ApiResponse<Vec<i64>> = response.json();

    assert_eq!(body.status_code, 200);
    assert!(body.status_message.contains("OK"));
    assert!(body.data.is_some());

    let mut game_ids = body.data.unwrap();
    game_ids.sort();

    assert_eq!(game_ids, vec![game_id1, game_id2]);
}

#[tokio::test]
async fn test_get_instructor_games_success_no_games() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 1002;
    let course_id = create_test_course(&pool, "Test Course 2").await;
    let _game_id1 = create_test_game(&pool, course_id, "Unassigned Game", 0).await;

    create_test_instructor(&pool, instructor_id, "teacher2@test.com", "Teacher Two").await;

    let response = server
        .get(&format!(
            "/teacher/get_instructor_games?instructor_id={}",
            instructor_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: ApiResponse<Vec<i64>> = response.json();

    assert_eq!(body.status_code, 200);
    assert!(body.status_message.contains("OK"));
    assert!(body.data.is_some());
    assert!(body.data.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_instructor_games_not_found() {
    let (server, _pool) = setup_test_environment().await;

    let non_existent_instructor_id = 9999;

    let response = server
        .get(&format!(
            "/teacher/get_instructor_games?instructor_id={}",
            non_existent_instructor_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);

    let body: ApiResponse<Value> = response.json();

    assert_eq!(body.status_code, 404);
    assert!(body.status_message.contains(&format!(
        "Instructor with ID {} not found",
        non_existent_instructor_id
    )));
    assert!(body.data.is_none());
}

#[tokio::test]
async fn test_get_instructor_games_bad_request_missing_param() {
    let (server, _pool) = setup_test_environment().await;

    let response = server.get("/teacher/get_instructor_games").await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_instructor_games_bad_request_invalid_param_type() {
    let (server, _pool) = setup_test_environment().await;

    let response = server
        .get("/teacher/get_instructor_games?instructor_id=not_an_integer")
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

// get_instructor_game_metadata

#[tokio::test]
async fn test_get_instructor_game_metadata_success_owner() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 2001;
    let player1_id = 2101;
    let player2_id = 2102;
    let course_id = create_test_course(&pool, "Course For Meta").await;
    let game_id = create_test_game(&pool, course_id, "Owned Game", 0).await;

    create_test_instructor(&pool, instructor_id, "owner@test.com", "Owner Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player(&pool, player1_id, "p1@test.com", "Player One").await;
    create_test_player(&pool, player2_id, "p2@test.com", "Player Two").await;
    create_test_player_registration(&pool, player1_id, game_id).await;
    create_test_player_registration(&pool, player2_id, game_id).await;

    let response = server
        .get(&format!(
            "/teacher/get_instructor_game_metadata?instructor_id={}&game_id={}",
            instructor_id, game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<InstructorGameMetadataResponse> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());

    let metadata = body.data.unwrap();
    assert_eq!(metadata.title, "Owned Game");
    assert!(metadata.active);
    assert!(!metadata.public);
    assert_eq!(metadata.player_count, 2);
    assert!(metadata.is_owner);
}

#[tokio::test]
async fn test_get_instructor_game_metadata_success_non_owner() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 2002;
    let player1_id = 2103;
    let course_id = create_test_course(&pool, "Course For Meta 2").await;
    let game_id = create_test_game(&pool, course_id, "Permitted Game", 0).await;

    create_test_instructor(&pool, instructor_id, "nonowner@test.com", "NonOwner Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, false).await;
    create_test_player(&pool, player1_id, "p3@test.com", "Player Three").await;
    create_test_player_registration(&pool, player1_id, game_id).await;

    let response = server
        .get(&format!(
            "/teacher/get_instructor_game_metadata?instructor_id={}&game_id={}",
            instructor_id, game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<InstructorGameMetadataResponse> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());

    let metadata = body.data.unwrap();
    assert_eq!(metadata.title, "Permitted Game");
    assert_eq!(metadata.player_count, 1);
    assert!(!metadata.is_owner);
}

#[tokio::test]
async fn test_get_instructor_game_metadata_success_admin() {
    let (server, pool) = setup_test_environment().await;

    let admin_instructor_id = 0;
    let player1_id = 2104;
    let course_id = create_test_course(&pool, "Course For Admin").await;
    let game_id = create_test_game(&pool, course_id, "Admin Accessible Game", 0).await;

    create_test_player(&pool, player1_id, "p4@test.com", "Player Four").await;
    create_test_player_registration(&pool, player1_id, game_id).await;

    let response = server
        .get(&format!(
            "/teacher/get_instructor_game_metadata?instructor_id={}&game_id={}",
            admin_instructor_id, game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<InstructorGameMetadataResponse> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());

    let metadata = body.data.unwrap();
    assert_eq!(metadata.title, "Admin Accessible Game");
    assert_eq!(metadata.player_count, 1);
    assert!(!metadata.is_owner);
}

#[tokio::test]
async fn test_get_instructor_game_metadata_forbidden() {
    let (server, pool) = setup_test_environment().await;

    let permitted_instructor_id = 2003;
    let forbidden_instructor_id = 2004;
    let course_id = create_test_course(&pool, "Course For Forbidden").await;
    let game_id = create_test_game(&pool, course_id, "Forbidden Game", 0).await;

    create_test_instructor(
        &pool,
        permitted_instructor_id,
        "perm@test.com",
        "Permitted Inst",
    )
    .await;
    create_test_instructor(
        &pool,
        forbidden_instructor_id,
        "forbid@test.com",
        "Forbidden Inst",
    )
    .await;
    create_test_game_ownership(&pool, permitted_instructor_id, game_id, true).await;

    let response = server
        .get(&format!(
            "/teacher/get_instructor_game_metadata?instructor_id={}&game_id={}",
            forbidden_instructor_id, game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 403);
    assert!(
        body.status_message
            .contains("does not have permission for game")
    );
    assert!(body.data.is_none());
}

#[tokio::test]
async fn test_get_instructor_game_metadata_not_found_game() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 2005;
    let non_existent_game_id = 99001;

    create_test_instructor(&pool, instructor_id, "find@test.com", "Finding Inst").await;

    let response = server
        .get(&format!(
            "/teacher/get_instructor_game_metadata?instructor_id={}&game_id={}",
            instructor_id, non_existent_game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(
        body.status_message
            .contains(&format!("game with ID {} not found", non_existent_game_id))
    );
    assert!(body.data.is_none());
}

#[tokio::test]
async fn test_get_instructor_game_metadata_bad_request_missing_game_id() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 2006;
    create_test_instructor(&pool, instructor_id, "badreq@test.com", "BadReq Inst").await;

    let response = server
        .get(&format!(
            "/teacher/get_instructor_game_metadata?instructor_id={}",
            instructor_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_instructor_game_metadata_bad_request_invalid_instructor_id() {
    let (server, pool) = setup_test_environment().await;
    let course_id = create_test_course(&pool, "Course BadReq").await;
    let game_id = create_test_game(&pool, course_id, "Game BadReq", 0).await;

    let response = server
        .get(&format!(
            "/teacher/get_instructor_game_metadata?instructor_id=invalid&game_id={}",
            game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

// list_students

#[tokio::test]
async fn test_list_students_success_no_filters() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 3001;
    let player1_id = 3101;
    let player2_id = 3102;
    let player3_id = 3103;
    let course_id = create_test_course(&pool, "Course For List").await;
    let game_id = create_test_game(&pool, course_id, "List Game 1", 0).await;

    create_test_instructor(&pool, instructor_id, "list@test.com", "List Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player(&pool, player1_id, "s1@test.com", "Student One").await;
    create_test_player(&pool, player2_id, "s2@test.com", "Student Two").await;
    create_test_player(&pool, player3_id, "s3@test.com", "Student Three").await;
    create_test_player_registration(&pool, player1_id, game_id).await;
    create_test_player_registration(&pool, player2_id, game_id).await;

    let response = server
        .get(&format!(
            "/teacher/list_students?instructor_id={}&game_id={}",
            instructor_id, game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let mut student_ids = body.data.unwrap();
    student_ids.sort();
    assert_eq!(student_ids, vec![player1_id, player2_id]);
}

#[tokio::test]
async fn test_list_students_success_only_active_filter() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 3002;
    let player_active1_id = 3104;
    let player_active2_id = 3105;
    let player_disabled_id = 3106;
    let course_id = create_test_course(&pool, "Course For Active").await;
    let game_id = create_test_game(&pool, course_id, "Active Game", 0).await;

    create_test_instructor(&pool, instructor_id, "active@test.com", "Active Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player(&pool, player_active1_id, "sa1@test.com", "Student Active 1").await;
    create_test_player(&pool, player_active2_id, "sa2@test.com", "Student Active 2").await;
    create_test_player(
        &pool,
        player_disabled_id,
        "sd1@test.com",
        "Student Disabled",
    )
    .await;
    create_test_player_registration(&pool, player_active1_id, game_id).await;
    create_test_player_registration(&pool, player_active2_id, game_id).await;
    create_test_player_registration(&pool, player_disabled_id, game_id).await;
    update_player_status(&pool, player_disabled_id, true).await;

    let response = server
        .get(&format!(
            "/teacher/list_students?instructor_id={}&game_id={}&only_active=true",
            instructor_id, game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let mut student_ids = body.data.unwrap();
    student_ids.sort();
    assert_eq!(student_ids, vec![player_active1_id, player_active2_id]);
}

#[tokio::test]
async fn test_list_students_success_group_filter() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 3003;
    let group1_id = 10;
    let group2_id = 11;
    let player_g1_1 = 3107;
    let player_g1_2 = 3108;
    let player_g2_1 = 3109;
    let player_nogrp = 3110;
    let course_id = create_test_course(&pool, "Course For Group").await;
    let game_id = create_test_game(&pool, course_id, "Group Game", 0).await;

    create_test_instructor(&pool, instructor_id, "group@test.com", "Group Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_group_with_id(&pool, group1_id, "Group A").await;
    create_test_group_with_id(&pool, group2_id, "Group B").await;
    create_test_player(&pool, player_g1_1, "sg11@test.com", "Student G1-1").await;
    create_test_player(&pool, player_g1_2, "sg12@test.com", "Student G1-2").await;
    create_test_player(&pool, player_g2_1, "sg21@test.com", "Student G2-1").await;
    create_test_player(&pool, player_nogrp, "sng@test.com", "Student No Group").await;

    create_test_player_registration(&pool, player_g1_1, game_id).await;
    create_test_player_registration(&pool, player_g1_2, game_id).await;
    create_test_player_registration(&pool, player_g2_1, game_id).await;
    create_test_player_registration(&pool, player_nogrp, game_id).await;

    add_player_to_group(&pool, player_g1_1, group1_id).await;
    add_player_to_group(&pool, player_g1_2, group1_id).await;
    add_player_to_group(&pool, player_g2_1, group2_id).await;

    let response = server
        .get(&format!(
            "/teacher/list_students?instructor_id={}&game_id={}&group_id={}",
            instructor_id, game_id, group1_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let mut student_ids = body.data.unwrap();
    student_ids.sort();
    assert_eq!(student_ids, vec![player_g1_1, player_g1_2]);
}

#[tokio::test]
async fn test_list_students_success_group_and_active_filters() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 3004;
    let group_id = 12;
    let player_g_active = 3111;
    let player_g_disabled = 3112;
    let player_active_nogrp = 3113;
    let course_id = create_test_course(&pool, "Course For Combo").await;
    let game_id = create_test_game(&pool, course_id, "Combo Game", 0).await;

    create_test_instructor(&pool, instructor_id, "combo@test.com", "Combo Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_group_with_id(&pool, group_id, "Combo Group").await;
    create_test_player(&pool, player_g_active, "sga@test.com", "Student G Active").await;
    create_test_player(
        &pool,
        player_g_disabled,
        "sgd@test.com",
        "Student G Disabled",
    )
    .await;
    create_test_player(
        &pool,
        player_active_nogrp,
        "san@test.com",
        "Student Active NoGrp",
    )
    .await;

    create_test_player_registration(&pool, player_g_active, game_id).await;
    create_test_player_registration(&pool, player_g_disabled, game_id).await;
    create_test_player_registration(&pool, player_active_nogrp, game_id).await;

    add_player_to_group(&pool, player_g_active, group_id).await;
    add_player_to_group(&pool, player_g_disabled, group_id).await;

    update_player_status(&pool, player_g_disabled, true).await;

    let response = server
        .get(&format!(
            "/teacher/list_students?instructor_id={}&game_id={}&group_id={}&only_active=true",
            instructor_id, game_id, group_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let student_ids = body.data.unwrap();
    assert_eq!(student_ids, vec![player_g_active]);
}

#[tokio::test]
async fn test_list_students_success_no_students_match() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 3005;
    let course_id = create_test_course(&pool, "Course For Empty").await;
    let game_id = create_test_game(&pool, course_id, "Empty Game", 0).await;

    create_test_instructor(&pool, instructor_id, "empty@test.com", "Empty Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;

    let response = server
        .get(&format!(
            "/teacher/list_students?instructor_id={}&game_id={}",
            instructor_id, game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    assert!(body.data.unwrap().is_empty());
}

#[tokio::test]
async fn test_list_students_forbidden() {
    let (server, pool) = setup_test_environment().await;

    let owner_instructor_id = 3006;
    let forbidden_instructor_id = 3007;
    let course_id = create_test_course(&pool, "Course For Forbidden 2").await;
    let game_id = create_test_game(&pool, course_id, "Forbidden Game 2", 0).await;

    create_test_instructor(
        &pool,
        owner_instructor_id,
        "owner2@test.com",
        "Owner Inst 2",
    )
    .await;
    create_test_instructor(
        &pool,
        forbidden_instructor_id,
        "forbid2@test.com",
        "Forbidden Inst 2",
    )
    .await;
    create_test_game_ownership(&pool, owner_instructor_id, game_id, true).await;

    let response = server
        .get(&format!(
            "/teacher/list_students?instructor_id={}&game_id={}",
            forbidden_instructor_id, game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 403);
    assert!(
        body.status_message
            .contains("does not have permission for game")
    );
}

#[tokio::test]
async fn test_list_students_not_found_game() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 3008;
    let non_existent_game_id = 9001;

    create_test_instructor(&pool, instructor_id, "find2@test.com", "Finding Inst 2").await;

    let response = server
        .get(&format!(
            "/teacher/list_students?instructor_id={}&game_id={}",
            instructor_id, non_existent_game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(
        body.status_message
            .contains(&format!("game with ID {} not found", non_existent_game_id))
    );
}

#[tokio::test]
async fn test_list_students_not_found_group_filter() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 3009;
    let course_id = create_test_course(&pool, "Course For Find Group").await;
    let game_id = create_test_game(&pool, course_id, "Find Group Game", 1).await;
    let non_existent_group_id = 9002;

    create_test_instructor(&pool, instructor_id, "findgrp@test.com", "Find Group Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;

    let response = server
        .get(&format!(
            "/teacher/list_students?instructor_id={}&game_id={}&group_id={}",
            instructor_id, game_id, non_existent_group_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(body.status_message.contains(&format!(
        "Filter group with ID {} not found",
        non_existent_group_id
    )));
}

#[tokio::test]
async fn test_list_students_bad_request_missing_game_id() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 3010;
    create_test_instructor(&pool, instructor_id, "badreq2@test.com", "BadReq Inst 2").await;

    let response = server
        .get(&format!(
            "/teacher/list_students?instructor_id={}",
            instructor_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

// get_student_progress

#[tokio::test]
async fn test_get_student_progress_success() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 4001;
    let player_id = 4101;
    let course_id = create_test_course(&pool, "Course For Progress").await;
    let game_id = create_test_game(&pool, course_id, "Progress Game", 3).await;
    let module_id = create_test_module(&pool, course_id, 1, "Progress Module").await;
    let ex1_id = create_test_exercise(&pool, module_id, 1, "Ex 1").await;
    let ex2_id = create_test_exercise(&pool, module_id, 2, "Ex 2").await;
    let _ex3_id = create_test_exercise(&pool, module_id, 3, "Ex 3").await;

    create_test_instructor(&pool, instructor_id, "progress@test.com", "Progress Inst").await;
    create_test_player(&pool, player_id, "stud_prog@test.com", "Progress Student").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player_id, game_id).await;

    create_test_submission(&pool, player_id, game_id, ex1_id, false, 0.5).await;
    create_test_submission(&pool, player_id, game_id, ex1_id, true, 1.0).await;
    create_test_submission(&pool, player_id, game_id, ex2_id, true, 1.0).await;
    create_test_submission(&pool, player_id, game_id, ex2_id, false, 1.0).await;

    let response = server
        .get(&format!(
            "/teacher/get_student_progress?instructor_id={}&game_id={}&player_id={}",
            instructor_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<StudentProgressResponse> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());

    let progress = body.data.unwrap();
    assert_eq!(progress.attempts, 4);
    assert_eq!(progress.solved_exercises, 2);
    assert!(approx_eq!(
        f64,
        progress.progress,
        66.66666666666666,
        ulps = 2
    ));
}

#[tokio::test]
async fn test_get_student_progress_success_no_submissions() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 4002;
    let player_id = 4102;
    let course_id = create_test_course(&pool, "Course Progress None").await;
    let game_id = create_test_game(&pool, course_id, "Progress Game None", 5).await;

    create_test_instructor(
        &pool,
        instructor_id,
        "progress0@test.com",
        "Progress Inst 0",
    )
    .await;
    create_test_player(
        &pool,
        player_id,
        "stud_prog0@test.com",
        "Progress Student 0",
    )
    .await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let response = server
        .get(&format!(
            "/teacher/get_student_progress?instructor_id={}&game_id={}&player_id={}",
            instructor_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<StudentProgressResponse> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());

    let progress = body.data.unwrap();
    assert_eq!(progress.attempts, 0);
    assert_eq!(progress.solved_exercises, 0);
    assert!(approx_eq!(f64, progress.progress, 0.0, ulps = 2));
}

#[tokio::test]
async fn test_get_student_progress_success_zero_total_exercises() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 4003;
    let player_id = 4103;
    let course_id = create_test_course(&pool, "Course Progress Zero").await;
    let game_id = create_test_game(&pool, course_id, "Progress Game Zero", 0).await;

    create_test_instructor(
        &pool,
        instructor_id,
        "progressZ@test.com",
        "Progress Inst Z",
    )
    .await;
    create_test_player(
        &pool,
        player_id,
        "stud_progZ@test.com",
        "Progress Student Z",
    )
    .await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let response = server
        .get(&format!(
            "/teacher/get_student_progress?instructor_id={}&game_id={}&player_id={}",
            instructor_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<StudentProgressResponse> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());

    let progress = body.data.unwrap();
    assert_eq!(progress.attempts, 0);
    assert_eq!(progress.solved_exercises, 0);
    assert!(approx_eq!(f64, progress.progress, 0.0, ulps = 2));
}

#[tokio::test]
async fn test_get_student_progress_forbidden() {
    let (server, pool) = setup_test_environment().await;

    let owner_instructor_id = 4004;
    let forbidden_instructor_id = 4005;
    let player_id = 4104;
    let course_id = create_test_course(&pool, "Course Progress Forbidden").await;
    let game_id = create_test_game(&pool, course_id, "Progress Game Forbidden", 1).await;

    create_test_instructor(
        &pool,
        owner_instructor_id,
        "owner_prog@test.com",
        "Owner Prog",
    )
    .await;
    create_test_instructor(
        &pool,
        forbidden_instructor_id,
        "forbid_prog@test.com",
        "Forbid Prog",
    )
    .await;
    create_test_player(
        &pool,
        player_id,
        "stud_progF@test.com",
        "Progress Student F",
    )
    .await;
    create_test_game_ownership(&pool, owner_instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let response = server
        .get(&format!(
            "/teacher/get_student_progress?instructor_id={}&game_id={}&player_id={}",
            forbidden_instructor_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 403);
}

#[tokio::test]
async fn test_get_student_progress_not_found_game() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 4006;
    let player_id = 4105;
    let non_existent_game_id = 9010;

    create_test_instructor(&pool, instructor_id, "findG_prog@test.com", "FindG Prog").await;
    create_test_player(
        &pool,
        player_id,
        "stud_progFG@test.com",
        "Progress Student FG",
    )
    .await;

    let response = server
        .get(&format!(
            "/teacher/get_student_progress?instructor_id={}&game_id={}&player_id={}",
            instructor_id, non_existent_game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(
        body.status_message
            .contains(&format!("game with ID {} not found", non_existent_game_id))
    );
}

#[tokio::test]
async fn test_get_student_progress_not_found_player_not_registered() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 4007;
    let player_id = 4106;
    let other_player_id = 4107;
    let course_id = create_test_course(&pool, "Course Progress NotReg").await;
    let game_id = create_test_game(&pool, course_id, "Progress Game NotReg", 2).await;

    create_test_instructor(&pool, instructor_id, "notreg_prog@test.com", "NotReg Prog").await;
    create_test_player(
        &pool,
        player_id,
        "stud_progR@test.com",
        "Progress Student R",
    )
    .await;
    create_test_player(
        &pool,
        other_player_id,
        "stud_progNR@test.com",
        "Progress Student NR",
    )
    .await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let response = server
        .get(&format!(
            "/teacher/get_student_progress?instructor_id={}&game_id={}&player_id={}",
            instructor_id, game_id, other_player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(body.status_message.contains(&format!(
        "Player with ID {} is not registered",
        other_player_id
    )));
}

#[tokio::test]
async fn test_get_student_progress_bad_request_missing_player_id() {
    let (server, pool) = setup_test_environment().await;

    let instructor_id = 4008;
    let course_id = create_test_course(&pool, "Course Progress BadReq").await;
    let game_id = create_test_game(&pool, course_id, "Progress Game BadReq", 1).await;
    create_test_instructor(&pool, instructor_id, "badreq_prog@test.com", "BadReq Prog").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;

    let response = server
        .get(&format!(
            "/teacher/get_student_progress?instructor_id={}&game_id={}",
            instructor_id, game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

// get_student_exercises
#[tokio::test]
async fn test_get_student_exercises_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 5001;
    let player_id = 5101;
    let course_id = create_test_course(&pool, "Course ExList").await;
    let game_id = create_test_game(&pool, course_id, "ExList Game", 3).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExList Module").await;
    let ex1_id = create_test_exercise(&pool, module_id, 1, "ExL 1").await;
    let ex2_id = create_test_exercise(&pool, module_id, 2, "ExL 2").await;
    let ex3_id = create_test_exercise(&pool, module_id, 3, "ExL 3").await;

    create_test_instructor(&pool, instructor_id, "exlist@test.com", "ExList Inst").await;
    create_test_player(&pool, player_id, "stud_exlist@test.com", "ExList Student").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player_id, game_id).await;

    create_test_submission(&pool, player_id, game_id, ex1_id, true, 1.0).await;
    create_test_submission(&pool, player_id, game_id, ex2_id, true, 1.0).await;
    create_test_submission(&pool, player_id, game_id, ex3_id, false, 0.5).await;

    let response = server
        .get(&format!(
            "/teacher/get_student_exercises?instructor_id={}&game_id={}&player_id={}",
            instructor_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<StudentExercisesResponse> = response.json();
    assert_eq!(body.status_code, 200);
    let data = body.data.unwrap();

    let mut attempted = data.attempted_exercises;
    attempted.sort();
    assert_eq!(attempted, vec![ex1_id, ex2_id, ex3_id]);

    let mut solved = data.solved_exercises;
    solved.sort();
    assert_eq!(solved, vec![ex1_id, ex2_id]);
}

#[tokio::test]
async fn test_get_student_exercises_not_registered() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 5002;
    let player_id = 5102;
    let course_id = create_test_course(&pool, "Course ExList NR").await;
    let game_id = create_test_game(&pool, course_id, "ExList Game NR", 1).await;

    create_test_instructor(&pool, instructor_id, "exlistnr@test.com", "ExListNR Inst").await;
    create_test_player(
        &pool,
        player_id,
        "stud_exlistnr@test.com",
        "ExListNR Student",
    )
    .await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;

    let response = server
        .get(&format!(
            "/teacher/get_student_exercises?instructor_id={}&game_id={}&player_id={}",
            instructor_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(
        body.status_message
            .contains(&format!("Player with ID {} is not registered", player_id))
    );
}

//  get_student_submissions
#[tokio::test]
async fn test_get_student_submissions_success_all() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 6001;
    let player_id = 6101;
    let course_id = create_test_course(&pool, "Course SubList").await;
    let game_id = create_test_game(&pool, course_id, "SubList Game", 2).await;
    let module_id = create_test_module(&pool, course_id, 1, "SubList Module").await;
    let ex1_id = create_test_exercise(&pool, module_id, 1, "SubL 1").await;

    create_test_instructor(&pool, instructor_id, "sublist@test.com", "SubList Inst").await;
    create_test_player(&pool, player_id, "stud_sublist@test.com", "SubList Student").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let sub1_id = create_test_submission(&pool, player_id, game_id, ex1_id, false, 0.5).await;
    let sub2_id = create_test_submission(&pool, player_id, game_id, ex1_id, true, 1.0).await;

    let response = server
        .get(&format!(
            "/teacher/get_student_submissions?instructor_id={}&game_id={}&player_id={}",
            instructor_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert_eq!(body.status_code, 200);
    let mut sub_ids = body.data.unwrap();
    sub_ids.sort();
    assert_eq!(sub_ids, vec![sub1_id, sub2_id]);
}

#[tokio::test]
async fn test_get_student_submissions_success_only() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 6002;
    let player_id = 6102;
    let course_id = create_test_course(&pool, "Course SubList Succ").await;
    let game_id = create_test_game(&pool, course_id, "SubList Game Succ", 2).await;
    let module_id = create_test_module(&pool, course_id, 1, "SubList Module Succ").await;
    let ex1_id = create_test_exercise(&pool, module_id, 1, "SubL Succ 1").await;

    create_test_instructor(&pool, instructor_id, "sublists@test.com", "SubListS Inst").await;
    create_test_player(
        &pool,
        player_id,
        "stud_sublists@test.com",
        "SubListS Student",
    )
    .await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let _sub1_id = create_test_submission(&pool, player_id, game_id, ex1_id, false, 0.4).await;
    let sub2_id = create_test_submission(&pool, player_id, game_id, ex1_id, true, 1.0).await;
    let sub3_id = create_test_submission(&pool, player_id, game_id, ex1_id, false, 1.0).await;

    let response = server
        .get(&format!(
            "/teacher/get_student_submissions?instructor_id={}&game_id={}&player_id={}&success_only=true",
            instructor_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert_eq!(body.status_code, 200);
    let mut sub_ids = body.data.unwrap();
    sub_ids.sort();
    assert_eq!(sub_ids, vec![sub2_id, sub3_id]);
}

// get_submission_data
#[tokio::test]
async fn test_get_submission_data_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 7001;
    let player_id = 7101;
    let course_id = create_test_course(&pool, "Course SubData").await;
    let game_id = create_test_game(&pool, course_id, "SubData Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "SubData Module").await;
    let ex1_id = create_test_exercise(&pool, module_id, 1, "SubD 1").await;

    create_test_instructor(&pool, instructor_id, "subdata@test.com", "SubData Inst").await;
    create_test_player(&pool, player_id, "stud_subdata@test.com", "SubData Student").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let sub_id = create_test_submission(&pool, player_id, game_id, ex1_id, true, 1.0).await;

    let response = server
        .get(&format!(
            "/teacher/get_submission_data?instructor_id={}&submission_id={}",
            instructor_id, sub_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<SubmissionDataResponse> = response.json();
    assert_eq!(body.status_code, 200);
    let data = body.data.unwrap();
    assert_eq!(data.id, sub_id);
    assert_eq!(data.player_id, player_id);
    assert_eq!(data.game_id, game_id);
    assert_eq!(data.exercise_id, ex1_id);
    assert!(data.first_solution);
}

#[tokio::test]
async fn test_get_submission_data_forbidden() {
    let (server, pool) = setup_test_environment().await;
    let owner_instructor_id = 7002;
    let forbidden_instructor_id = 7003;
    let player_id = 7102;
    let course_id = create_test_course(&pool, "Course SubData F").await;
    let game_id = create_test_game(&pool, course_id, "SubData Game F", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "SubData Module F").await;
    let ex1_id = create_test_exercise(&pool, module_id, 1, "SubD F 1").await;

    create_test_instructor(
        &pool,
        owner_instructor_id,
        "subdatao@test.com",
        "SubDataO Inst",
    )
    .await;
    create_test_instructor(
        &pool,
        forbidden_instructor_id,
        "subdataf@test.com",
        "SubDataF Inst",
    )
    .await;
    create_test_player(
        &pool,
        player_id,
        "stud_subdataf@test.com",
        "SubDataF Student",
    )
    .await;
    create_test_game_ownership(&pool, owner_instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player_id, game_id).await;
    let sub_id = create_test_submission(&pool, player_id, game_id, ex1_id, true, 1.0).await;

    let response = server
        .get(&format!(
            "/teacher/get_submission_data?instructor_id={}&submission_id={}",
            forbidden_instructor_id, sub_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_submission_data_not_found() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 7004;
    let non_existent_sub_id = 99999;
    create_test_instructor(&pool, instructor_id, "subdatanf@test.com", "SubDataNF Inst").await;

    let response = server
        .get(&format!(
            "/teacher/get_submission_data?instructor_id={}&submission_id={}",
            instructor_id, non_existent_sub_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// get_exercise_stats
#[tokio::test]
async fn test_get_exercise_stats_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 8001;
    let player1_id = 8101;
    let player2_id = 8102;
    let player3_id = 8103;
    let course_id = create_test_course(&pool, "Course ExStats").await;
    let game_id = create_test_game(&pool, course_id, "ExStats Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExStats Module").await;
    let ex_id = create_test_exercise(&pool, module_id, 1, "ExS 1").await;

    create_test_instructor(&pool, instructor_id, "exstats@test.com", "ExStats Inst").await;
    create_test_player(&pool, player1_id, "stud_exs1@test.com", "ExStats S1").await;
    create_test_player(&pool, player2_id, "stud_exs2@test.com", "ExStats S2").await;
    create_test_player(&pool, player3_id, "stud_exs3@test.com", "ExStats S3").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player1_id, game_id).await;
    create_test_player_registration(&pool, player2_id, game_id).await;
    create_test_player_registration(&pool, player3_id, game_id).await;

    create_test_submission(&pool, player1_id, game_id, ex_id, false, 0.4).await;
    create_test_submission(&pool, player1_id, game_id, ex_id, true, 0.9).await;
    create_test_submission(&pool, player2_id, game_id, ex_id, false, 0.2).await;
    create_test_submission(&pool, player2_id, game_id, ex_id, false, 0.3).await;

    let response = server
        .get(&format!(
            "/teacher/get_exercise_stats?instructor_id={}&game_id={}&exercise_id={}",
            instructor_id, game_id, ex_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExerciseStatsResponse> = response.json();
    assert_eq!(body.status_code, 200);
    let stats = body.data.unwrap();

    assert_eq!(stats.attempts, 4);
    assert_eq!(stats.successful_attempts, 1);
    assert!(approx_eq!(f64, stats.difficulty, 75.0, ulps = 2));
    assert!(approx_eq!(
        f64,
        stats.solved_percentage,
        33.33333333333333,
        ulps = 2
    ));
}

#[tokio::test]
async fn test_get_exercise_stats_no_attempts() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 8002;
    let course_id = create_test_course(&pool, "Course ExStats NA").await;
    let game_id = create_test_game(&pool, course_id, "ExStats Game NA", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExStats Module NA").await;
    let ex_id = create_test_exercise(&pool, module_id, 1, "ExS NA 1").await;

    create_test_instructor(&pool, instructor_id, "exstatsna@test.com", "ExStatsNA Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;

    let response = server
        .get(&format!(
            "/teacher/get_exercise_stats?instructor_id={}&game_id={}&exercise_id={}",
            instructor_id, game_id, ex_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExerciseStatsResponse> = response.json();
    let stats = body.data.unwrap();
    assert_eq!(stats.attempts, 0);
    assert_eq!(stats.successful_attempts, 0);
    assert!(approx_eq!(f64, stats.difficulty, 0.0, ulps = 2));
    assert!(approx_eq!(f64, stats.solved_percentage, 0.0, ulps = 2));
}

#[tokio::test]
async fn test_get_exercise_stats_not_found_exercise() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 8003;
    let course_id = create_test_course(&pool, "Course ExStats NFE").await;
    let game_id = create_test_game(&pool, course_id, "ExStats Game NFE", 0).await;
    let non_existent_ex_id = 99001;

    create_test_instructor(
        &pool,
        instructor_id,
        "exstatsnfe@test.com",
        "ExStatsNFE Inst",
    )
    .await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;

    let response = server
        .get(&format!(
            "/teacher/get_exercise_stats?instructor_id={}&game_id={}&exercise_id={}",
            instructor_id, game_id, non_existent_ex_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(body.status_message.contains(&format!(
        "Exercise with ID {} not found",
        non_existent_ex_id
    )));
}

// get_exercise_submissions
#[tokio::test]
async fn test_get_exercise_submissions_success_all() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 9001;
    let player1_id = 9101;
    let player2_id = 9102;
    let course_id = create_test_course(&pool, "Course ExSubs").await;
    let game_id = create_test_game(&pool, course_id, "ExSubs Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExSubs Module").await;
    let ex_id = create_test_exercise(&pool, module_id, 1, "ExSub 1").await;

    create_test_instructor(&pool, instructor_id, "exsubs@test.com", "ExSubs Inst").await;
    create_test_player(&pool, player1_id, "stud_exsub1@test.com", "ExSub S1").await;
    create_test_player(&pool, player2_id, "stud_exsub2@test.com", "ExSub S2").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player1_id, game_id).await;
    create_test_player_registration(&pool, player2_id, game_id).await;

    let sub1_id = create_test_submission(&pool, player1_id, game_id, ex_id, true, 1.0).await;
    let sub2_id = create_test_submission(&pool, player2_id, game_id, ex_id, false, 0.3).await;

    let response = server
        .get(&format!(
            "/teacher/get_exercise_submissions?instructor_id={}&game_id={}&exercise_id={}",
            instructor_id, game_id, ex_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    let mut sub_ids = body.data.unwrap();
    sub_ids.sort();
    assert_eq!(sub_ids, vec![sub1_id, sub2_id]);
}

#[tokio::test]
async fn test_get_exercise_submissions_success_only() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 9002;
    let player1_id = 9103;
    let player2_id = 9104;
    let course_id = create_test_course(&pool, "Course ExSubs S").await;
    let game_id = create_test_game(&pool, course_id, "ExSubs Game S", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExSubs Module S").await;
    let ex_id = create_test_exercise(&pool, module_id, 1, "ExSub S 1").await;

    create_test_instructor(&pool, instructor_id, "exsubss@test.com", "ExSubsS Inst").await;
    create_test_player(&pool, player1_id, "stud_exsubs1@test.com", "ExSubS S1").await;
    create_test_player(&pool, player2_id, "stud_exsubs2@test.com", "ExSubS S2").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, player1_id, game_id).await;
    create_test_player_registration(&pool, player2_id, game_id).await;

    let sub1_id = create_test_submission(&pool, player1_id, game_id, ex_id, true, 0.8).await;
    let _sub2_id = create_test_submission(&pool, player2_id, game_id, ex_id, false, 0.1).await;

    let response = server
        .get(&format!(
            "/teacher/get_exercise_submissions?instructor_id={}&game_id={}&exercise_id={}&success_only=true",
            instructor_id, game_id, ex_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    let sub_ids = body.data.unwrap();
    assert_eq!(sub_ids, vec![sub1_id]);
}

// create_game
#[tokio::test]
async fn test_create_game_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 10001;
    let course_id = create_test_course(&pool, "Course For Create Game").await;

    create_test_instructor(&pool, instructor_id, "createg@test.com", "CreateG Inst").await;

    let payload = CreateGamePayload {
        instructor_id,
        title: "My New Rust Game".to_string(),
        public: false,
        active: true,
        description: "A game about Rust".to_string(),
        course_id,
        programming_language: "rust".to_string(),
        module_lock: 0.0,
        exercise_lock: false,
    };

    let response = server.post("/teacher/create_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<i64> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let _new_game_id = body.data.unwrap();
}

#[tokio::test]
async fn test_create_game_instructor_not_found() {
    let (server, pool) = setup_test_environment().await;
    let non_existent_instructor_id = 99001;
    let course_id = create_test_course(&pool, "Course CreateG NF Inst").await;

    let payload = json!({
        "instructor_id": non_existent_instructor_id,
        "title": "Game NF Inst",
        "course_id": course_id,
        "programming_language": "py"
    });

    let response = server.post("/teacher/create_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(body.status_message.contains(&format!(
        "Instructor with ID {} not found",
        non_existent_instructor_id
    )));
}

#[tokio::test]
async fn test_create_game_course_not_found() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 10002;
    let non_existent_course_id = 99002;
    create_test_instructor(&pool, instructor_id, "creategnf@test.com", "CreateGNF Inst").await;

    let payload = json!({
        "instructor_id": instructor_id,
        "title": "Game NF Course",
        "course_id": non_existent_course_id,
        "programming_language": "py"
    });

    let response = server.post("/teacher/create_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(body.status_message.contains(&format!(
        "Course with ID {} not found",
        non_existent_course_id
    )));
}

#[tokio::test]
async fn test_create_game_language_not_allowed() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 10003;
    let course_id = create_test_course(&pool, "Course Lang NA").await;
    create_test_instructor(
        &pool,
        instructor_id,
        "createlang@test.com",
        "CreateLang Inst",
    )
    .await;

    let payload = json!({
        "instructor_id": instructor_id,
        "title": "Game Lang NA",
        "course_id": course_id,
        "programming_language": "java"
    });

    let response = server.post("/teacher/create_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: ApiResponse<Value> = response.json();
    assert!(body.status_message.contains("not allowed for course"));
}

// modify_game
#[tokio::test]
async fn test_modify_game_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 11001;
    let course_id = create_test_course(&pool, "Course Modify").await;
    let game_id = create_test_game(&pool, course_id, "Original Title", 5).await;
    create_test_instructor(&pool, instructor_id, "modifyg@test.com", "ModifyG Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;

    let payload = ModifyGamePayload {
        instructor_id,
        game_id,
        title: Some("Updated Title".to_string()),
        description: Some("New description.".to_string()),
        active: Some(false),
        public: None,
        module_lock: None,
        exercise_lock: None,
    };

    let response = server.post("/teacher/modify_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_modify_game_forbidden() {
    let (server, pool) = setup_test_environment().await;
    let owner_instructor_id = 11002;
    let forbidden_instructor_id = 11003;
    let course_id = create_test_course(&pool, "Course Modify F").await;
    let game_id = create_test_game(&pool, course_id, "Modify F Title", 1).await;
    create_test_instructor(
        &pool,
        owner_instructor_id,
        "modifygo@test.com",
        "ModifyGO Inst",
    )
    .await;
    create_test_instructor(
        &pool,
        forbidden_instructor_id,
        "modifygf@test.com",
        "ModifyGF Inst",
    )
    .await;
    create_test_game_ownership(&pool, owner_instructor_id, game_id, true).await;

    let payload = ModifyGamePayload {
        instructor_id: forbidden_instructor_id,
        game_id,
        title: Some("Attempted Update".to_string()),
        public: None,
        active: None,
        description: None,
        module_lock: None,
        exercise_lock: None,
    };

    let response = server.post("/teacher/modify_game").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_modify_game_not_found() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 11004;
    let non_existent_game_id = 99101;
    create_test_instructor(&pool, instructor_id, "modifygnf@test.com", "ModifyGNF Inst").await;

    let payload = ModifyGamePayload {
        instructor_id,
        game_id: non_existent_game_id,
        title: Some("Attempted Update NF".to_string()),
        public: None,
        active: None,
        description: None,
        module_lock: None,
        exercise_lock: None,
    };

    let response = server.post("/teacher/modify_game").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// add_game_instructor
#[tokio::test]
async fn test_add_game_instructor_success() {
    let (server, pool) = setup_test_environment().await;
    let requesting_instructor_id = 12001;
    let instructor_to_add_id = 12002;
    let course_id = create_test_course(&pool, "Course AddInst").await;
    let game_id = create_test_game(&pool, course_id, "AddInst Game", 1).await;
    create_test_instructor(
        &pool,
        requesting_instructor_id,
        "addgireq@test.com",
        "AddGIReq Inst",
    )
    .await;
    create_test_instructor(
        &pool,
        instructor_to_add_id,
        "addgiadd@test.com",
        "AddGIAdd Inst",
    )
    .await;
    create_test_game_ownership(&pool, requesting_instructor_id, game_id, true).await;

    let payload = AddGameInstructorPayload {
        requesting_instructor_id,
        game_id,
        instructor_to_add_id,
        is_owner: false,
    };

    let response = server
        .post("/teacher/add_game_instructor")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_add_game_instructor_update_owner_status() {
    let (server, pool) = setup_test_environment().await;
    let requesting_instructor_id = 12003;
    let instructor_to_add_id = 12004;
    let course_id = create_test_course(&pool, "Course AddInst Upd").await;
    let game_id = create_test_game(&pool, course_id, "AddInst Game Upd", 1).await;
    create_test_instructor(
        &pool,
        requesting_instructor_id,
        "addgirequ@test.com",
        "AddGIReqU Inst",
    )
    .await;
    create_test_instructor(
        &pool,
        instructor_to_add_id,
        "addgiaddu@test.com",
        "AddGIAddU Inst",
    )
    .await;
    create_test_game_ownership(&pool, requesting_instructor_id, game_id, true).await;
    create_test_game_ownership(&pool, instructor_to_add_id, game_id, false).await;

    let payload = AddGameInstructorPayload {
        requesting_instructor_id,
        game_id,
        instructor_to_add_id,
        is_owner: true,
    };

    let response = server
        .post("/teacher/add_game_instructor")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_add_game_instructor_forbidden() {
    let (server, pool) = setup_test_environment().await;
    let owner_instructor_id = 12005;
    let non_owner_requesting_id = 12006;
    let instructor_to_add_id = 12007;
    let course_id = create_test_course(&pool, "Course AddInst F").await;
    let game_id = create_test_game(&pool, course_id, "AddInst Game F", 1).await;
    create_test_instructor(&pool, owner_instructor_id, "addgio@test.com", "AddGIO Inst").await;
    create_test_instructor(
        &pool,
        non_owner_requesting_id,
        "addginon@test.com",
        "AddGINON Inst",
    )
    .await;
    create_test_instructor(
        &pool,
        instructor_to_add_id,
        "addgiaddf@test.com",
        "AddGIAddF Inst",
    )
    .await;
    create_test_game_ownership(&pool, owner_instructor_id, game_id, true).await;
    create_test_game_ownership(&pool, non_owner_requesting_id, game_id, false).await;

    let payload = AddGameInstructorPayload {
        requesting_instructor_id: non_owner_requesting_id,
        game_id,
        instructor_to_add_id,
        is_owner: false,
    };

    let response = server
        .post("/teacher/add_game_instructor")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_add_game_instructor_instructor_to_add_not_found() {
    let (server, pool) = setup_test_environment().await;
    let requesting_instructor_id = 12008;
    let non_existent_instructor_id = 99201;
    let course_id = create_test_course(&pool, "Course AddInst NF").await;
    let game_id = create_test_game(&pool, course_id, "AddInst Game NF", 1).await;
    create_test_instructor(
        &pool,
        requesting_instructor_id,
        "addginf@test.com",
        "AddGINF Inst",
    )
    .await;
    create_test_game_ownership(&pool, requesting_instructor_id, game_id, true).await;

    let payload = AddGameInstructorPayload {
        requesting_instructor_id,
        game_id,
        instructor_to_add_id: non_existent_instructor_id,
        is_owner: false,
    };

    let response = server
        .post("/teacher/add_game_instructor")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(body.status_message.contains(&format!(
        "Instructor with ID {} not found",
        non_existent_instructor_id
    )));
}

// remove_game_instructor
#[tokio::test]
async fn test_remove_game_instructor_success() {
    let (server, pool) = setup_test_environment().await;
    let requesting_instructor_id = 13001;
    let instructor_to_remove_id = 13002;
    let course_id = create_test_course(&pool, "Course RemInst").await;
    let game_id = create_test_game(&pool, course_id, "RemInst Game", 1).await;
    create_test_instructor(
        &pool,
        requesting_instructor_id,
        "remgireq@test.com",
        "RemGIReq Inst",
    )
    .await;
    create_test_instructor(
        &pool,
        instructor_to_remove_id,
        "remgirem@test.com",
        "RemGIRem Inst",
    )
    .await;
    create_test_game_ownership(&pool, requesting_instructor_id, game_id, true).await;
    create_test_game_ownership(&pool, instructor_to_remove_id, game_id, false).await;

    let payload = RemoveGameInstructorPayload {
        requesting_instructor_id,
        game_id,
        instructor_to_remove_id,
    };

    let response = server
        .post("/teacher/remove_game_instructor")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_remove_game_instructor_forbidden() {
    let (server, pool) = setup_test_environment().await;
    let owner_instructor_id = 13003;
    let non_owner_requesting_id = 13004;
    let instructor_to_remove_id = 13005;
    let course_id = create_test_course(&pool, "Course RemInst F").await;
    let game_id = create_test_game(&pool, course_id, "RemInst Game F", 1).await;
    create_test_instructor(&pool, owner_instructor_id, "remgio@test.com", "RemGIO Inst").await;
    create_test_instructor(
        &pool,
        non_owner_requesting_id,
        "remginon@test.com",
        "RemGINON Inst",
    )
    .await;
    create_test_instructor(
        &pool,
        instructor_to_remove_id,
        "remgiremf@test.com",
        "RemGIRemF Inst",
    )
    .await;
    create_test_game_ownership(&pool, owner_instructor_id, game_id, true).await;
    create_test_game_ownership(&pool, non_owner_requesting_id, game_id, false).await;
    create_test_game_ownership(&pool, instructor_to_remove_id, game_id, false).await;

    let payload = RemoveGameInstructorPayload {
        requesting_instructor_id: non_owner_requesting_id,
        game_id,
        instructor_to_remove_id,
    };

    let response = server
        .post("/teacher/remove_game_instructor")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_remove_game_instructor_not_associated() {
    let (server, pool) = setup_test_environment().await;
    let requesting_instructor_id = 13006;
    let instructor_not_associated_id = 13007;
    let course_id = create_test_course(&pool, "Course RemInst NA").await;
    let game_id = create_test_game(&pool, course_id, "RemInst Game NA", 1).await;
    create_test_instructor(
        &pool,
        requesting_instructor_id,
        "remgina@test.com",
        "RemGINA Inst",
    )
    .await;
    create_test_instructor(
        &pool,
        instructor_not_associated_id,
        "remginarem@test.com",
        "RemGINARem Inst",
    )
    .await;
    create_test_game_ownership(&pool, requesting_instructor_id, game_id, true).await;

    let payload = RemoveGameInstructorPayload {
        requesting_instructor_id,
        game_id,
        instructor_to_remove_id: instructor_not_associated_id,
    };

    let response = server
        .post("/teacher/remove_game_instructor")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(body.status_message.contains(&format!(
        "Instructor {} is not associated with game",
        instructor_not_associated_id
    )));
}

// activate_game
#[tokio::test]
async fn test_activate_game_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 14001;
    let course_id = create_test_course(&pool, "Course Activate").await;
    let game_id = create_test_game(&pool, course_id, "Activate Game", 1).await;
    create_test_instructor(&pool, instructor_id, "activateg@test.com", "ActivateG Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::games::table.find(game_id))
            .set(schema::games::active.eq(false))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let payload = ActivateGamePayload {
        instructor_id,
        game_id,
    };
    let response = server.post("/teacher/activate_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

// stop_game
#[tokio::test]
async fn test_stop_game_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 15001;
    let course_id = create_test_course(&pool, "Course Stop").await;
    let game_id = create_test_game(&pool, course_id, "Stop Game", 1).await;
    create_test_instructor(&pool, instructor_id, "stopg@test.com", "StopG Inst").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;

    let payload = StopGamePayload {
        instructor_id,
        game_id,
    };
    let response = server.post("/teacher/stop_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

// remove_game_student
#[tokio::test]
async fn test_remove_game_student_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 16001;
    let student_id = 16101;
    let course_id = create_test_course(&pool, "Course RemStud").await;
    let game_id = create_test_game(&pool, course_id, "RemStud Game", 1).await;
    create_test_instructor(&pool, instructor_id, "remstud@test.com", "RemStud Inst").await;
    create_test_player(&pool, student_id, "remstuds@test.com", "RemStud S").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_player_registration(&pool, student_id, game_id).await;

    let payload = RemoveGameStudentPayload {
        instructor_id,
        game_id,
        student_id,
    };
    let response = server
        .post("/teacher/remove_game_student")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_remove_game_student_not_registered() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 16002;
    let student_id = 16102;
    let course_id = create_test_course(&pool, "Course RemStud NR").await;
    let game_id = create_test_game(&pool, course_id, "RemStud Game NR", 1).await;
    create_test_instructor(&pool, instructor_id, "remstudnr@test.com", "RemStudNR Inst").await;
    create_test_player(&pool, student_id, "remstudsnr@test.com", "RemStud SNR").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;

    let payload = RemoveGameStudentPayload {
        instructor_id,
        game_id,
        student_id,
    };
    let response = server
        .post("/teacher/remove_game_student")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(
        body.status_message
            .contains(&format!("Student {} is not registered in game", student_id))
    );
}

// translate_email_to_player_id
#[tokio::test]
async fn test_translate_email_success() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 17101;
    let email = "translate@test.com";
    create_test_player(&pool, player_id, email, "Translate Me").await;

    let response = server
        .get(&format!(
            "/teacher/translate_email_to_player_id?email={}",
            email
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<i64> = response.json();
    assert_eq!(body.data.unwrap(), player_id);
}

#[tokio::test]
async fn test_translate_email_not_found() {
    let (server, _pool) = setup_test_environment().await;
    let email = "notfound@test.com";

    let response = server
        .get(&format!(
            "/teacher/translate_email_to_player_id?email={}",
            email
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// create_group
#[tokio::test]
async fn test_create_group_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 18001;
    let player1_id = 18101;
    let player2_id = 18102;
    create_test_instructor(
        &pool,
        instructor_id,
        "creategroup@test.com",
        "CreateGrp Inst",
    )
    .await;
    create_test_player(&pool, player1_id, "grp_p1@test.com", "Grp P1").await;
    create_test_player(&pool, player2_id, "grp_p2@test.com", "Grp P2").await;

    let payload = CreateGroupPayload {
        instructor_id,
        display_name: "My New Group".to_string(),
        display_avatar: None,
        member_list: vec![player1_id, player2_id],
    };

    let response = server.post("/teacher/create_group").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<i64> = response.json();
    assert!(body.data.is_some());
    let _new_group_id = body.data.unwrap();
}

#[tokio::test]
async fn test_create_group_name_conflict() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 18002;
    let group_name = "Existing Group Name";
    create_test_instructor(
        &pool,
        instructor_id,
        "creategroupc@test.com",
        "CreateGrpC Inst",
    )
    .await;
    let _group_id = create_test_group_with_id(&pool, 50, group_name).await;

    let payload = CreateGroupPayload {
        instructor_id,
        display_name: group_name.to_string(),
        display_avatar: None,
        member_list: vec![],
    };

    let response = server.post("/teacher/create_group").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_create_group_member_not_found() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 18003;
    let player1_id = 18103;
    let non_existent_player_id = 99181;
    create_test_instructor(
        &pool,
        instructor_id,
        "creategroupnf@test.com",
        "CreateGrpNF Inst",
    )
    .await;
    create_test_player(&pool, player1_id, "grp_p1nf@test.com", "Grp P1NF").await;

    let payload = CreateGroupPayload {
        instructor_id,
        display_name: "Group With NF Member".to_string(),
        display_avatar: None,
        member_list: vec![player1_id, non_existent_player_id],
    };

    let response = server.post("/teacher/create_group").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(
        body.status_message
            .contains("players listed as members do not exist")
    );
}

// dissolve_group
#[tokio::test]
async fn test_dissolve_group_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 19001;
    let group_id = 60;
    let player_id = 19101;
    create_test_instructor(&pool, instructor_id, "dissolveg@test.com", "DissolveG Inst").await;
    create_test_group_with_id(&pool, group_id, "Group To Dissolve").await;
    create_test_player(&pool, player_id, "diss_p1@test.com", "Diss P1").await;
    create_test_group_ownership(&pool, instructor_id, group_id, true).await;
    add_player_to_group(&pool, player_id, group_id).await;

    let payload = DissolveGroupPayload {
        instructor_id,
        group_id,
    };
    let response = server.post("/teacher/dissolve_group").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_dissolve_group_forbidden() {
    let (server, pool) = setup_test_environment().await;
    let owner_instructor_id = 19002;
    let non_owner_instructor_id = 19003;
    let group_id = 61;
    create_test_instructor(
        &pool,
        owner_instructor_id,
        "dissolvego@test.com",
        "DissolveGO Inst",
    )
    .await;
    create_test_instructor(
        &pool,
        non_owner_instructor_id,
        "dissolvegnon@test.com",
        "DissolveGNON Inst",
    )
    .await;
    create_test_group_with_id(&pool, group_id, "Group Dissolve F").await;
    create_test_group_ownership(&pool, owner_instructor_id, group_id, true).await;

    let payload = DissolveGroupPayload {
        instructor_id: non_owner_instructor_id,
        group_id,
    };
    let response = server.post("/teacher/dissolve_group").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_dissolve_group_not_found() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 19004;
    let non_existent_group_id = 99060;
    create_test_instructor(
        &pool,
        instructor_id,
        "dissolvegnf@test.com",
        "DissolveGNF Inst",
    )
    .await;

    let payload = DissolveGroupPayload {
        instructor_id,
        group_id: non_existent_group_id,
    };
    let response = server.post("/teacher/dissolve_group").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// add_group_member
#[tokio::test]
async fn test_add_group_member_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 20001;
    let group_id = 70;
    let player_id = 20101;
    create_test_instructor(&pool, instructor_id, "addgm@test.com", "AddGM Inst").await;
    create_test_group_with_id(&pool, group_id, "Group Add Member").await;
    create_test_player(&pool, player_id, "addgm_p1@test.com", "AddGM P1").await;
    create_test_group_ownership(&pool, instructor_id, group_id, true).await;

    let payload = AddGroupMemberPayload {
        instructor_id,
        group_id,
        player_id,
    };
    let response = server
        .post("/teacher/add_group_member")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_add_group_member_already_exists() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 20002;
    let group_id = 71;
    let player_id = 20102;
    create_test_instructor(&pool, instructor_id, "addgmae@test.com", "AddGMAE Inst").await;
    create_test_group_with_id(&pool, group_id, "Group Add Member AE").await;
    create_test_player(&pool, player_id, "addgmae_p1@test.com", "AddGMAE P1").await;
    create_test_group_ownership(&pool, instructor_id, group_id, true).await;
    add_player_to_group(&pool, player_id, group_id).await;

    let payload = AddGroupMemberPayload {
        instructor_id,
        group_id,
        player_id,
    };
    let response = server
        .post("/teacher/add_group_member")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_add_group_member_player_not_found() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 20003;
    let group_id = 72;
    let non_existent_player_id = 99201;
    create_test_instructor(&pool, instructor_id, "addgmpnf@test.com", "AddGMPNF Inst").await;
    create_test_group_with_id(&pool, group_id, "Group Add Member PNF").await;
    create_test_group_ownership(&pool, instructor_id, group_id, true).await;

    let payload = AddGroupMemberPayload {
        instructor_id,
        group_id,
        player_id: non_existent_player_id,
    };
    let response = server
        .post("/teacher/add_group_member")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(body.status_message.contains(&format!(
        "Player with ID {} not found",
        non_existent_player_id
    )));
}

// remove_group_member
#[tokio::test]
async fn test_remove_group_member_success() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 21001;
    let group_id = 80;
    let player_id = 21101;
    create_test_instructor(&pool, instructor_id, "remgm@test.com", "RemGM Inst").await;
    create_test_group_with_id(&pool, group_id, "Group Rem Member").await;
    create_test_player(&pool, player_id, "remgm_p1@test.com", "RemGM P1").await;
    create_test_group_ownership(&pool, instructor_id, group_id, true).await;
    add_player_to_group(&pool, player_id, group_id).await;

    let payload = RemoveGroupMemberPayload {
        instructor_id,
        group_id,
        player_id,
    };
    let response = server
        .post("/teacher/remove_group_member")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_remove_group_member_not_member() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 21002;
    let group_id = 81;
    let player_id = 21102;
    create_test_instructor(&pool, instructor_id, "remgmnm@test.com", "RemGMNM Inst").await;
    create_test_group_with_id(&pool, group_id, "Group Rem Member NM").await;
    create_test_player(&pool, player_id, "remgmnm_p1@test.com", "RemGMNM P1").await;
    create_test_group_ownership(&pool, instructor_id, group_id, true).await;

    let payload = RemoveGroupMemberPayload {
        instructor_id,
        group_id,
        player_id,
    };
    let response = server
        .post("/teacher/remove_group_member")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(
        body.status_message
            .contains(&format!("Player {} is not a member of group", player_id))
    );
}

// create_player
#[tokio::test]
async fn test_create_player_success_admin() {
    let (server, _pool) = setup_test_environment().await;
    let admin_instructor_id = 0;

    let payload = CreatePlayerPayload {
        instructor_id: admin_instructor_id,
        email: "newplayer_admin@test.com".to_string(),
        display_name: "Admin Created Player".to_string(),
        display_avatar: None,
        game_id: None,
        group_id: None,
        language: None,
    };

    let response = server.post("/teacher/create_player").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<i64> = response.json();
    assert!(body.data.is_some());
}

#[tokio::test]
async fn test_create_player_success_with_game_and_group() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 22001;
    let course_id = create_test_course(&pool, "Course CreateP GG").await;
    let game_id = create_test_game(&pool, course_id, "CreateP Game GG", 1).await;
    let group_id = 90;
    create_test_instructor(&pool, instructor_id, "createpgg@test.com", "CreatePGG Inst").await;
    create_test_group_with_id(&pool, group_id, "CreateP Group GG").await;
    create_test_game_ownership(&pool, instructor_id, game_id, true).await;
    create_test_group_ownership(&pool, instructor_id, group_id, true).await;

    let payload = CreatePlayerPayload {
        instructor_id,
        email: "newplayer_gg@test.com".to_string(),
        display_name: "GG Created Player".to_string(),
        display_avatar: None,
        game_id: Some(game_id),
        group_id: Some(group_id),
        language: Some("fr".to_string()),
    };

    let response = server.post("/teacher/create_player").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<i64> = response.json();
    assert!(body.data.is_some());
    let _new_player_id = body.data.unwrap();
}

#[tokio::test]
async fn test_create_player_forbidden_no_context() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 22002;
    create_test_instructor(&pool, instructor_id, "createpf@test.com", "CreatePF Inst").await;

    let payload = CreatePlayerPayload {
        instructor_id,
        email: "newplayer_f@test.com".to_string(),
        display_name: "F Created Player".to_string(),
        display_avatar: None,
        game_id: None,
        group_id: None,
        language: None,
    };

    let response = server.post("/teacher/create_player").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_player_email_conflict() {
    let (server, pool) = setup_test_environment().await;
    let admin_instructor_id = 0;
    let existing_email = "existing_player@test.com";
    create_test_player(&pool, 22101, existing_email, "Existing Player").await;

    let payload = CreatePlayerPayload {
        instructor_id: admin_instructor_id,
        email: existing_email.to_string(),
        display_name: "Conflict Player".to_string(),
        display_avatar: None,
        game_id: None,
        group_id: None,
        language: None,
    };

    let response = server.post("/teacher/create_player").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::CONFLICT);
}

// disable_player
#[tokio::test]
async fn test_disable_player_success_admin() {
    let (server, pool) = setup_test_environment().await;
    let admin_instructor_id = 0;
    let player_id = 23101;
    create_test_player(&pool, player_id, "disablep@test.com", "Disable Me").await;

    let payload = DisablePlayerPayload {
        instructor_id: admin_instructor_id,
        player_id,
    };
    let response = server.post("/teacher/disable_player").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_disable_player_forbidden_non_admin() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 23001;
    let player_id = 23102;
    create_test_instructor(&pool, instructor_id, "disablepf@test.com", "DisablePF Inst").await;
    create_test_player(&pool, player_id, "disablep_f@test.com", "Disable Me F").await;

    let payload = DisablePlayerPayload {
        instructor_id,
        player_id,
    };
    let response = server.post("/teacher/disable_player").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_disable_player_not_found() {
    let (server, _pool) = setup_test_environment().await;
    let admin_instructor_id = 0;
    let non_existent_player_id = 99231;

    let payload = DisablePlayerPayload {
        instructor_id: admin_instructor_id,
        player_id: non_existent_player_id,
    };
    let response = server.post("/teacher/disable_player").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// delete_player
#[tokio::test]
async fn test_delete_player_success_admin() {
    let (server, pool) = setup_test_environment().await;
    let admin_instructor_id = 0;
    let player_id = 24101;
    let course_id = create_test_course(&pool, "Course DelP").await;
    let game_id = create_test_game(&pool, course_id, "DelP Game", 1).await;
    let group_id = 100;
    let module_id = create_test_module(&pool, course_id, 1, "DelP Mod").await;
    let ex_id = create_test_exercise(&pool, module_id, 1, "DelP Ex").await;

    create_test_player(&pool, player_id, "deletep@test.com", "Delete Me").await;
    create_test_group_with_id(&pool, group_id, "DelP Group").await;
    create_test_player_registration(&pool, player_id, game_id).await;
    add_player_to_group(&pool, player_id, group_id).await;
    create_test_submission(&pool, player_id, game_id, ex_id, true, 1.0).await;

    let payload = DeletePlayerPayload {
        instructor_id: admin_instructor_id,
        player_id,
    };
    let response = server.post("/teacher/delete_player").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_delete_player_forbidden_non_admin() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 24001;
    let player_id = 24102;
    create_test_instructor(&pool, instructor_id, "deletepf@test.com", "DeletePF Inst").await;
    create_test_player(&pool, player_id, "deletep_f@test.com", "Delete Me F").await;

    let payload = DeletePlayerPayload {
        instructor_id,
        player_id,
    };
    let response = server.post("/teacher/delete_player").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_delete_player_not_found() {
    let (server, _pool) = setup_test_environment().await;
    let admin_instructor_id = 0;
    let non_existent_player_id = 99241;

    let payload = DeletePlayerPayload {
        instructor_id: admin_instructor_id,
        player_id: non_existent_player_id,
    };
    let response = server.post("/teacher/delete_player").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// generate_invite_link
#[tokio::test]
async fn test_generate_invite_link_success_admin_no_context() {
    let (server, pool) = setup_test_environment().await;
    let admin_instructor_id = 0;

    create_test_instructor(&pool, admin_instructor_id, "admin@test.com", "Admin User").await;

    let payload = GenerateInviteLinkPayload {
        instructor_id: admin_instructor_id,
        game_id: None,
        group_id: None,
    };

    let response = server
        .post("/teacher/generate_invite_link")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<InviteLinkResponse> = response.json();
    assert!(body.data.is_some());
}

#[tokio::test]
async fn test_generate_invite_link_success_with_game_group() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 25001;
    let course_id = create_test_course(&pool, "Course Invite GG").await;
    let game_id = create_test_game(&pool, course_id, "Invite Game GG", 1).await;
    let group_id = 110;
    create_test_instructor(&pool, instructor_id, "invitegg@test.com", "InviteGG Inst").await;
    create_test_group_with_id(&pool, group_id, "Invite Group GG").await;
    create_test_group_ownership(&pool, instructor_id, group_id, true).await;

    let payload = GenerateInviteLinkPayload {
        instructor_id,
        game_id: Some(game_id),
        group_id: Some(group_id),
    };

    let response = server
        .post("/teacher/generate_invite_link")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<InviteLinkResponse> = response.json();
    assert!(body.data.is_some());
}

#[tokio::test]
async fn test_generate_invite_link_forbidden_no_group_permission() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 25002;
    let group_id = 111;
    create_test_instructor(&pool, instructor_id, "invitef@test.com", "InviteF Inst").await;
    create_test_group_with_id(&pool, group_id, "Invite Group F").await;

    let payload = GenerateInviteLinkPayload {
        instructor_id,
        game_id: None,
        group_id: Some(group_id),
    };

    let response = server
        .post("/teacher/generate_invite_link")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(
        body.status_message
            .contains("lacks permission for the specified group")
    );
}

#[tokio::test]
async fn test_generate_invite_link_game_not_found() {
    let (server, pool) = setup_test_environment().await;
    let admin_instructor_id = 0;
    let non_existent_game_id = 99251;
    create_test_instructor(&pool, admin_instructor_id, "admin@test.com", "Admin").await;

    let payload = GenerateInviteLinkPayload {
        instructor_id: admin_instructor_id,
        game_id: Some(non_existent_game_id),
        group_id: None,
    };

    let response = server
        .post("/teacher/generate_invite_link")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert!(
        body.status_message
            .contains(&format!("Game with ID {} not found", non_existent_game_id))
    );
}

// process_invite_link
#[tokio::test]
async fn test_process_invite_link_success_add_to_game_group() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 26001;
    let player_id = 26101;
    let course_id = create_test_course(&pool, "Course Process").await;
    let game_id = create_test_game(&pool, course_id, "Process Game", 1).await;
    let group_id = 120;
    create_test_instructor(&pool, instructor_id, "process@test.com", "Process Inst").await;
    create_test_group_with_id(&pool, group_id, "Process Group").await;
    create_test_player(&pool, player_id, "process_p@test.com", "Process P").await;
    let invite_uuid = create_test_invite(&pool, instructor_id, Some(game_id), Some(group_id)).await;

    let payload = ProcessInviteLinkPayload {
        player_id,
        uuid: invite_uuid,
    };
    let response = server
        .post("/teacher/process_invite_link")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_process_invite_link_success_already_member() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 26002;
    let player_id = 26102;
    let course_id = create_test_course(&pool, "Course Process AE").await;
    let game_id = create_test_game(&pool, course_id, "Process Game AE", 1).await;
    let group_id = 121;
    create_test_instructor(&pool, instructor_id, "processae@test.com", "ProcessAE Inst").await;
    create_test_group_with_id(&pool, group_id, "Process Group AE").await;
    create_test_player(&pool, player_id, "processae_p@test.com", "ProcessAE P").await;
    create_test_player_registration(&pool, player_id, game_id).await;
    add_player_to_group(&pool, player_id, group_id).await;
    let invite_uuid = create_test_invite(&pool, instructor_id, Some(game_id), Some(group_id)).await;

    let payload = ProcessInviteLinkPayload {
        player_id,
        uuid: invite_uuid,
    };
    let response = server
        .post("/teacher/process_invite_link")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(body.data.unwrap_or(false));
}

#[tokio::test]
async fn test_process_invite_link_invite_not_found() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 26103;
    let non_existent_uuid = Uuid::new_v4();
    create_test_player(&pool, player_id, "processnf_p@test.com", "ProcessNF P").await;

    let payload = ProcessInviteLinkPayload {
        player_id,
        uuid: non_existent_uuid,
    };
    let response = server
        .post("/teacher/process_invite_link")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_process_invite_link_player_not_found() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 26003;
    let non_existent_player_id = 99261;
    create_test_instructor(
        &pool,
        instructor_id,
        "processpnf@test.com",
        "ProcessPNF Inst",
    )
    .await;
    let invite_uuid = create_test_invite(&pool, instructor_id, None, None).await;

    let payload = ProcessInviteLinkPayload {
        player_id: non_existent_player_id,
        uuid: invite_uuid,
    };
    let response = server
        .post("/teacher/process_invite_link")
        .json(&payload)
        .await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_process_invite_link_partial_add_to_group() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 27001;
    let player_id = 27101;
    let course_id = create_test_course(&pool, "Course Partial G").await;
    let game_id = create_test_game(&pool, course_id, "Partial Game G", 1).await;
    let group_id = 130;
    create_test_instructor(&pool, instructor_id, "partial_g@test.com", "PartialG Inst").await;
    create_test_group_with_id(&pool, group_id, "Partial Group G").await;
    create_test_player(&pool, player_id, "partial_g_p@test.com", "PartialG P").await;

    create_test_player_registration(&pool, player_id, game_id).await;
    assert!(
        check_player_in_game(&pool, player_id, game_id).await,
        "Pre-condition failed: Player should be in game"
    );
    assert!(
        !check_player_in_group(&pool, player_id, group_id).await,
        "Pre-condition failed: Player should NOT be in group"
    );

    let invite_uuid = create_test_invite(&pool, instructor_id, Some(game_id), Some(group_id)).await;

    let payload = ProcessInviteLinkPayload {
        player_id,
        uuid: invite_uuid,
    };
    let response = server
        .post("/teacher/process_invite_link")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(
        body.data.unwrap_or(false),
        "API response data should be true"
    );

    assert!(
        check_player_in_game(&pool, player_id, game_id).await,
        "Post-condition failed: Player should still be in game"
    );
    assert!(
        check_player_in_group(&pool, player_id, group_id).await,
        "Post-condition failed: Player should NOW be in group"
    );
    assert_eq!(
        count_player_game_registrations(&pool, player_id).await,
        1,
        "Player should only have 1 game registration"
    );
    assert_eq!(
        count_player_group_memberships(&pool, player_id).await,
        1,
        "Player should only have 1 group membership"
    );
}

#[tokio::test]
async fn test_process_invite_link_partial_add_to_game() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 27002;
    let player_id = 27102;
    let course_id = create_test_course(&pool, "Course Partial P").await;
    let game_id = create_test_game(&pool, course_id, "Partial Game P", 1).await;
    let group_id = 131;
    create_test_instructor(&pool, instructor_id, "partial_p@test.com", "PartialP Inst").await;
    create_test_group_with_id(&pool, group_id, "Partial Group P").await;
    create_test_player(&pool, player_id, "partial_p_p@test.com", "PartialP P").await;

    add_player_to_group(&pool, player_id, group_id).await;
    assert!(
        !check_player_in_game(&pool, player_id, game_id).await,
        "Pre-condition failed: Player should NOT be in game"
    );
    assert!(
        check_player_in_group(&pool, player_id, group_id).await,
        "Pre-condition failed: Player should be in group"
    );

    let invite_uuid = create_test_invite(&pool, instructor_id, Some(game_id), Some(group_id)).await;

    let payload = ProcessInviteLinkPayload {
        player_id,
        uuid: invite_uuid,
    };
    let response = server
        .post("/teacher/process_invite_link")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(
        body.data.unwrap_or(false),
        "API response data should be true"
    );

    assert!(
        check_player_in_game(&pool, player_id, game_id).await,
        "Post-condition failed: Player should NOW be in game"
    );
    assert!(
        check_player_in_group(&pool, player_id, group_id).await,
        "Post-condition failed: Player should still be in group"
    );
    assert_eq!(
        count_player_game_registrations(&pool, player_id).await,
        1,
        "Player should only have 1 game registration"
    );
    assert_eq!(
        count_player_group_memberships(&pool, player_id).await,
        1,
        "Player should only have 1 group membership"
    );
}

#[tokio::test]
async fn test_process_invite_link_success_game_only() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 27003;
    let player_id = 27103;
    let course_id = create_test_course(&pool, "Course GameOnly").await;
    let game_id = create_test_game(&pool, course_id, "GameOnly Game", 1).await;
    create_test_instructor(&pool, instructor_id, "gameonly@test.com", "GameOnly Inst").await;
    create_test_player(&pool, player_id, "gameonly_p@test.com", "GameOnly P").await;

    assert!(
        !check_player_in_game(&pool, player_id, game_id).await,
        "Pre-condition failed: Player should NOT be in game"
    );
    assert_eq!(
        count_player_group_memberships(&pool, player_id).await,
        0,
        "Pre-condition failed: Player should be in 0 groups"
    );

    let invite_uuid = create_test_invite(&pool, instructor_id, Some(game_id), None).await;

    let payload = ProcessInviteLinkPayload {
        player_id,
        uuid: invite_uuid,
    };
    let response = server
        .post("/teacher/process_invite_link")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(
        body.data.unwrap_or(false),
        "API response data should be true"
    );

    assert!(
        check_player_in_game(&pool, player_id, game_id).await,
        "Post-condition failed: Player should NOW be in game"
    );
    assert_eq!(
        count_player_game_registrations(&pool, player_id).await,
        1,
        "Player should only have 1 game registration"
    );
    assert_eq!(
        count_player_group_memberships(&pool, player_id).await,
        0,
        "Player should still be in 0 groups"
    );
}

#[tokio::test]
async fn test_process_invite_link_success_group_only() {
    let (server, pool) = setup_test_environment().await;
    let instructor_id = 27004;
    let player_id = 27104;
    let group_id = 132;
    create_test_instructor(&pool, instructor_id, "grouponly@test.com", "GroupOnly Inst").await;
    create_test_group_with_id(&pool, group_id, "GroupOnly Group").await;
    create_test_player(&pool, player_id, "grouponly_p@test.com", "GroupOnly P").await;

    assert!(
        !check_player_in_group(&pool, player_id, group_id).await,
        "Pre-condition failed: Player should NOT be in group"
    );
    assert_eq!(
        count_player_game_registrations(&pool, player_id).await,
        0,
        "Pre-condition failed: Player should be in 0 games"
    );

    let invite_uuid = create_test_invite(&pool, instructor_id, None, Some(group_id)).await;

    let payload = ProcessInviteLinkPayload {
        player_id,
        uuid: invite_uuid,
    };
    let response = server
        .post("/teacher/process_invite_link")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(
        body.data.unwrap_or(false),
        "API response data should be true"
    );

    assert!(
        check_player_in_group(&pool, player_id, group_id).await,
        "Post-condition failed: Player should NOW be in group"
    );
    assert_eq!(
        count_player_group_memberships(&pool, player_id).await,
        1,
        "Player should only have 1 group membership"
    );
    assert_eq!(
        count_player_game_registrations(&pool, player_id).await,
        0,
        "Player should still be in 0 games"
    );
}
