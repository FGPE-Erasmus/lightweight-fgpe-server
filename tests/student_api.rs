use axum::http::StatusCode;
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::Utc;
use diesel::ExpressionMethods;
use diesel::{QueryDsl, RunQueryDsl};
use lightweight_fgpe_server::model::student::{
    CourseDataResponse, ExerciseDataResponse, GameMetadata, LastSolutionResponse,
    ModuleDataResponse,
};
use lightweight_fgpe_server::payloads::student::{
    JoinGamePayload, LeaveGamePayload, LoadGamePayload, SaveGamePayload, SetGameLangPayload,
    SubmitSolutionPayload, UnlockPayload,
};
use lightweight_fgpe_server::response::ApiResponse;
use serde_json::{Value, json};

mod helpers;
use helpers::{
    check_player_in_game, check_player_unlock_exists, create_test_course, create_test_exercise,
    create_test_game, create_test_module, create_test_player, create_test_player_registration,
    create_test_player_unlock, create_test_submission, setup_test_environment,
};
use lightweight_fgpe_server::schema;

// get_available_games

#[tokio::test]
async fn test_get_available_games_success() {
    let (server, pool) = setup_test_environment().await;
    let course_id = create_test_course(&pool, "Available Course").await;
    let game1_id = create_test_game(&pool, course_id, "Public Active Game", 1).await;
    let _game2_id = create_test_game(&pool, course_id, "Private Active Game", 1).await;
    let game3_id = create_test_game(&pool, course_id, "Public Active Game 2", 1).await;
    let _game4_id = create_test_game(&pool, course_id, "Public Inactive Game", 1).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::games::table.find(game1_id))
            .set((
                schema::games::public.eq(true),
                schema::games::active.eq(true),
            ))
            .execute(conn)?;
        diesel::update(schema::games::table.find(game3_id))
            .set((
                schema::games::public.eq(true),
                schema::games::active.eq(true),
            ))
            .execute(conn)?;
        diesel::update(schema::games::table.find(_game4_id))
            .set((
                schema::games::public.eq(true),
                schema::games::active.eq(false),
            ))
            .execute(conn)?;
        Ok::<_, diesel::result::Error>(())
    })
    .await
    .unwrap()
    .unwrap();

    let response = server.get("/student/get_available_games").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let mut game_ids = body.data.unwrap();
    game_ids.sort();
    assert_eq!(game_ids, vec![game1_id, game3_id]);
}

#[tokio::test]
async fn test_get_available_games_success_none_available() {
    let (server, pool) = setup_test_environment().await;
    let course_id = create_test_course(&pool, "No Available Course").await;
    let _game1_id = create_test_game(&pool, course_id, "Private Game", 1).await;
    let _game2_id = create_test_game(&pool, course_id, "Inactive Game", 1).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::games::table.find(_game2_id))
            .set(schema::games::active.eq(false))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server.get("/student/get_available_games").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    assert!(body.data.unwrap().is_empty());
}

// join_game

#[tokio::test]
async fn test_join_game_success() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 101;
    let course_id = create_test_course(&pool, "Join Course").await;
    let game_id = create_test_game(&pool, course_id, "Join Game", 1).await;
    create_test_player(&pool, player_id, "join@test.com", "Join Player").await;

    let payload = JoinGamePayload {
        player_id,
        game_id,
        language: "en".to_string(),
    };

    let response = server.post("/student/join_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<i64> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let registration_id = body.data.unwrap();
    assert!(registration_id > 0);

    assert!(
        check_player_in_game(&pool, player_id, game_id).await,
        "Player should be registered in the game"
    );
}

#[tokio::test]
async fn test_join_game_conflict_already_registered() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 102;
    let course_id = create_test_course(&pool, "Join Conflict Course").await;
    let game_id = create_test_game(&pool, course_id, "Join Conflict Game", 1).await;
    create_test_player(&pool, player_id, "join_conflict@test.com", "Join Conflict").await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let payload = JoinGamePayload {
        player_id,
        game_id,
        language: "en".to_string(),
    };

    let response = server.post("/student/join_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::CONFLICT);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 409);
    assert!(body.status_message.contains("already registered in game"));
}

#[tokio::test]
async fn test_join_game_not_found_player() {
    let (server, pool) = setup_test_environment().await;
    let non_existent_player_id = 9901;
    let course_id = create_test_course(&pool, "Join NF Player Course").await;
    let game_id = create_test_game(&pool, course_id, "Join NF Player Game", 1).await;

    let payload = JoinGamePayload {
        player_id: non_existent_player_id,
        game_id,
        language: "en".to_string(),
    };

    let response = server.post("/student/join_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(body.status_message.contains("Player with ID"));
    assert!(body.status_message.contains("not found"));
}

#[tokio::test]
async fn test_join_game_not_found_game() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 103;
    let non_existent_game_id = 9902;
    create_test_player(&pool, player_id, "join_nf_game@test.com", "Join NF Game P").await;

    let payload = JoinGamePayload {
        player_id,
        game_id: non_existent_game_id,
        language: "en".to_string(),
    };

    let response = server.post("/student/join_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(body.status_message.contains("Game with ID"));
    assert!(body.status_message.contains("not found"));
}

// save_game

#[tokio::test]
async fn test_save_game_success() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 201;
    let course_id = create_test_course(&pool, "Save Course").await;
    let game_id = create_test_game(&pool, course_id, "Save Game", 1).await;
    create_test_player(&pool, player_id, "save@test.com", "Save Player").await;
    let registration_id = create_test_player_registration(&pool, player_id, game_id).await;

    let game_state = json!({"level": 5, "score": 1200});
    let payload = SaveGamePayload {
        player_registrations_id: registration_id,
        game_state: game_state.clone(),
    };

    let response = server.post("/student/save_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.unwrap_or(false));

    let conn = pool.get().await.unwrap();
    let (saved_state, saved_at): (Value, chrono::DateTime<Utc>) = conn
        .interact(move |conn| {
            schema::player_registrations::table
                .find(registration_id)
                .select((
                    schema::player_registrations::game_state,
                    schema::player_registrations::saved_at,
                ))
                .first(conn)
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(saved_state, game_state);
    assert!(saved_at > Utc::now() - chrono::Duration::seconds(5));
}

#[tokio::test]
async fn test_save_game_not_found_registration() {
    let (server, _pool) = setup_test_environment().await;
    let non_existent_registration_id = 9911;

    let payload = SaveGamePayload {
        player_registrations_id: non_existent_registration_id,
        game_state: json!({}),
    };

    let response = server.post("/student/save_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(body.status_message.contains("Player registration"));
}

// load_game

#[tokio::test]
async fn test_load_game_success() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 301;
    let course_id = create_test_course(&pool, "Load Course").await;
    let game_id = create_test_game(&pool, course_id, "Load Game", 1).await;
    create_test_player(&pool, player_id, "load@test.com", "Load Player").await;
    let registration_id = create_test_player_registration(&pool, player_id, game_id).await;

    let game_state_to_save = json!({"checkpoint": "level_3", "items": ["key", "potion"]});

    let conn = pool.get().await.unwrap();
    let state_clone = game_state_to_save.clone();
    conn.interact(move |conn| {
        diesel::update(schema::player_registrations::table.find(registration_id))
            .set(schema::player_registrations::game_state.eq(state_clone))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let payload = LoadGamePayload {
        player_registrations_id: registration_id,
    };

    let response = server.post("/student/load_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    assert_eq!(body.data.unwrap(), game_state_to_save);
}

#[tokio::test]
async fn test_load_game_not_found_registration() {
    let (server, _pool) = setup_test_environment().await;
    let non_existent_registration_id = 9921;

    let payload = LoadGamePayload {
        player_registrations_id: non_existent_registration_id,
    };

    let response = server.post("/student/load_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
}

// leave_game

#[tokio::test]
async fn test_leave_game_success() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 401;
    let course_id = create_test_course(&pool, "Leave Course").await;
    let game_id = create_test_game(&pool, course_id, "Leave Game", 1).await;
    create_test_player(&pool, player_id, "leave@test.com", "Leave Player").await;
    let registration_id = create_test_player_registration(&pool, player_id, game_id).await;

    let payload = LeaveGamePayload { player_id, game_id };

    let response = server.post("/student/leave_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<()> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_none());

    let conn = pool.get().await.unwrap();
    let left_at: Option<chrono::DateTime<Utc>> = conn
        .interact(move |conn| {
            schema::player_registrations::table
                .find(registration_id)
                .select(schema::player_registrations::left_at)
                .first(conn)
        })
        .await
        .unwrap()
        .unwrap();

    assert!(left_at.is_some());
    assert!(left_at.unwrap() > Utc::now() - chrono::Duration::seconds(5));
}

#[tokio::test]
async fn test_leave_game_not_found_registration() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 402;
    let course_id = create_test_course(&pool, "Leave NF Course").await;
    let game_id = create_test_game(&pool, course_id, "Leave NF Game", 1).await;
    create_test_player(&pool, player_id, "leave_nf@test.com", "Leave NF Player").await;

    let payload = LeaveGamePayload { player_id, game_id };

    let response = server.post("/student/leave_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(
        body.status_message
            .contains("Active player registration not found")
    );
}

#[tokio::test]
async fn test_leave_game_already_left() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 403;
    let course_id = create_test_course(&pool, "Leave AL Course").await;
    let game_id = create_test_game(&pool, course_id, "Leave AL Game", 1).await;
    create_test_player(&pool, player_id, "leave_al@test.com", "Leave AL Player").await;
    let registration_id = create_test_player_registration(&pool, player_id, game_id).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::player_registrations::table.find(registration_id))
            .set(schema::player_registrations::left_at.eq(Utc::now()))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let payload = LeaveGamePayload { player_id, game_id };
    let response = server.post("/student/leave_game").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// set_game_lang

#[tokio::test]
async fn test_set_game_lang_success() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 501;
    let course_id = create_test_course(&pool, "Lang Course").await;
    let game_id = create_test_game(&pool, course_id, "Lang Game", 1).await;
    create_test_player(&pool, player_id, "lang@test.com", "Lang Player").await;
    let registration_id = create_test_player_registration(&pool, player_id, game_id).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::courses::table.find(course_id))
            .set(schema::courses::languages.eq("en,fr"))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let payload = SetGameLangPayload {
        player_id,
        game_id,
        language: "fr".to_string(),
    };

    let response = server.post("/student/set_game_lang").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.unwrap_or(false));

    let conn = pool.get().await.unwrap();
    let lang: String = conn
        .interact(move |conn| {
            schema::player_registrations::table
                .find(registration_id)
                .select(schema::player_registrations::language)
                .first(conn)
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(lang, "fr");
}

#[tokio::test]
async fn test_set_game_lang_unprocessable_language_not_allowed() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 502;
    let course_id = create_test_course(&pool, "Lang Invalid Course").await;
    let game_id = create_test_game(&pool, course_id, "Lang Invalid Game", 1).await;
    create_test_player(&pool, player_id, "lang_inv@test.com", "Lang Inv Player").await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let payload = SetGameLangPayload {
        player_id,
        game_id,
        language: "de".to_string(),
    };

    let response = server.post("/student/set_game_lang").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 422);
    assert!(body.status_message.contains("Language 'de' is not valid"));
}

#[tokio::test]
async fn test_set_game_lang_not_found_registration() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 503;
    let course_id = create_test_course(&pool, "Lang NF Course").await;
    let game_id = create_test_game(&pool, course_id, "Lang NF Game", 1).await;
    create_test_player(&pool, player_id, "lang_nf@test.com", "Lang NF Player").await;

    let payload = SetGameLangPayload {
        player_id,
        game_id,
        language: "en".to_string(),
    };

    let response = server.post("/student/set_game_lang").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// get_player_games

#[tokio::test]
async fn test_get_player_games_success_active_only() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 601;
    let course_id = create_test_course(&pool, "PlayerGames Course").await;
    let game_active_id = create_test_game(&pool, course_id, "PG Active Game", 1).await;
    let game_inactive_id = create_test_game(&pool, course_id, "PG Inactive Game", 1).await;
    let game_left_id = create_test_game(&pool, course_id, "PG Left Game", 1).await;
    create_test_player(&pool, player_id, "pg@test.com", "Player Games").await;

    let reg_active_id = create_test_player_registration(&pool, player_id, game_active_id).await;
    let _reg_inactive_id =
        create_test_player_registration(&pool, player_id, game_inactive_id).await;
    let reg_left_id = create_test_player_registration(&pool, player_id, game_left_id).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::games::table.find(game_inactive_id))
            .set(schema::games::active.eq(false))
            .execute(conn)?;
        diesel::update(schema::player_registrations::table.find(reg_left_id))
            .set(schema::player_registrations::left_at.eq(Utc::now()))
            .execute(conn)?;
        Ok::<_, diesel::result::Error>(())
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_player_games?player_id={}&active=true",
            player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    assert_eq!(body.data.unwrap(), vec![reg_active_id]);
}

#[tokio::test]
async fn test_get_player_games_success_all() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 602;
    let course_id = create_test_course(&pool, "PlayerGames All Course").await;
    let game_active_id = create_test_game(&pool, course_id, "PGA Active Game", 1).await;
    let game_inactive_id = create_test_game(&pool, course_id, "PGA Inactive Game", 1).await;
    let game_left_id = create_test_game(&pool, course_id, "PGA Left Game", 1).await;
    create_test_player(&pool, player_id, "pga@test.com", "Player Games All").await;

    let reg_active_id = create_test_player_registration(&pool, player_id, game_active_id).await;
    let reg_inactive_id = create_test_player_registration(&pool, player_id, game_inactive_id).await;
    let reg_left_id = create_test_player_registration(&pool, player_id, game_left_id).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::games::table.find(game_inactive_id))
            .set(schema::games::active.eq(false))
            .execute(conn)?;
        diesel::update(schema::player_registrations::table.find(reg_left_id))
            .set(schema::player_registrations::left_at.eq(Utc::now()))
            .execute(conn)?;
        Ok::<_, diesel::result::Error>(())
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_player_games?player_id={}&active=false",
            player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let mut reg_ids = body.data.unwrap();
    reg_ids.sort();
    let mut expected_ids = vec![reg_active_id, reg_inactive_id, reg_left_id];
    expected_ids.sort();
    assert_eq!(reg_ids, expected_ids);
}

#[tokio::test]
async fn test_get_player_games_success_no_registrations() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 603;
    create_test_player(&pool, player_id, "pg_none@test.com", "Player Games None").await;

    let response = server
        .get(&format!(
            "/student/get_player_games?player_id={}&active=false",
            player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Vec<i64>> = response.json();
    assert!(body.data.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_player_games_not_found_player() {
    let (server, _pool) = setup_test_environment().await;
    let non_existent_player_id = 9931;

    let response = server
        .get(&format!(
            "/student/get_player_games?player_id={}&active=true",
            non_existent_player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
    assert!(body.status_message.contains("Player with ID"));
}

// get_game_metadata

#[tokio::test]
async fn test_get_game_metadata_success() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 701;
    let course_id = create_test_course(&pool, "Metadata Course").await;
    let game_id = create_test_game(&pool, course_id, "Metadata Game", 5).await;
    create_test_player(&pool, player_id, "meta@test.com", "Metadata Player").await;
    let registration_id = create_test_player_registration(&pool, player_id, game_id).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::player_registrations::table.find(registration_id))
            .set(schema::player_registrations::progress.eq(2))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!("/student/get_game_metadata/{}", registration_id))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<GameMetadata> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let metadata = body.data.unwrap();

    assert_eq!(metadata.registration_id, registration_id);
    assert_eq!(metadata.game_id, game_id);
    assert_eq!(metadata.game_title, "Metadata Game");
    assert_eq!(metadata.progress, 2);
    assert_eq!(metadata.language, "en");
    assert!(metadata.game_active);
    assert_eq!(metadata.game_total_exercises, 5);
    assert!(metadata.left_at.is_none());
}

#[tokio::test]
async fn test_get_game_metadata_not_found_registration() {
    let (server, _pool) = setup_test_environment().await;
    let non_existent_registration_id = 9941;

    let response = server
        .get(&format!(
            "/student/get_game_metadata/{}",
            non_existent_registration_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let body: ApiResponse<Value> = response.json();
    assert_eq!(body.status_code, 404);
}

// get_course_data

#[tokio::test]
async fn test_get_course_data_success() {
    let (server, pool) = setup_test_environment().await;
    let course_id = create_test_course(&pool, "CourseData Course").await;
    let game_id = create_test_game(&pool, course_id, "CourseData Game", 3).await;
    let module1_id = create_test_module(&pool, course_id, 1, "CD Mod EN 1").await;
    let module2_id = create_test_module(&pool, course_id, 2, "CD Mod EN 2").await;
    let _module3_id = create_test_module(&pool, course_id, 1, "CD Mod FR 1").await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::courses::table.find(course_id))
            .set((
                schema::courses::gamification_rule_conditions.eq("cond1"),
                schema::courses::gamification_complex_rules.eq("rule1"),
                schema::courses::gamification_rule_results.eq("res1"),
            ))
            .execute(conn)?;
        diesel::update(schema::modules::table.find(module1_id))
            .set(schema::modules::language.eq("en"))
            .execute(conn)?;
        diesel::update(schema::modules::table.find(module2_id))
            .set(schema::modules::language.eq("en"))
            .execute(conn)?;
        diesel::update(schema::modules::table.find(_module3_id))
            .set(schema::modules::language.eq("fr"))
            .execute(conn)?;
        Ok::<_, diesel::result::Error>(())
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_course_data?game_id={}&language=en",
            game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<CourseDataResponse> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let data = body.data.unwrap();

    assert_eq!(data.gamification_rule_conditions, "cond1");
    assert_eq!(data.gamification_complex_rules, "rule1");
    assert_eq!(data.gamification_rule_results, "res1");
    let mut module_ids = data.module_ids;
    module_ids.sort();
    assert_eq!(module_ids, vec![module1_id, module2_id]);
}

#[tokio::test]
async fn test_get_course_data_success_no_matching_modules() {
    let (server, pool) = setup_test_environment().await;
    let course_id = create_test_course(&pool, "CourseData NoMod Course").await;
    let game_id = create_test_game(&pool, course_id, "CourseData NoMod Game", 0).await;
    let _module1_id = create_test_module(&pool, course_id, 1, "CD NoMod EN 1").await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::modules::table.find(_module1_id))
            .set(schema::modules::language.eq("en"))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_course_data?game_id={}&language=fr",
            game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<CourseDataResponse> = response.json();
    assert!(body.data.is_some());
    assert!(body.data.unwrap().module_ids.is_empty());
}

#[tokio::test]
async fn test_get_course_data_not_found_game() {
    let (server, _pool) = setup_test_environment().await;
    let non_existent_game_id = 9951;

    let response = server
        .get(&format!(
            "/student/get_course_data?game_id={}&language=en",
            non_existent_game_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// get_module_data

#[tokio::test]
async fn test_get_module_data_success() {
    let (server, pool) = setup_test_environment().await;
    let course_id = create_test_course(&pool, "ModuleData Course").await;
    let module_id = create_test_module(&pool, course_id, 1, "ModuleData Module").await;
    let ex1_id = create_test_exercise(&pool, module_id, 1, "MD Ex PY 1").await;
    let ex2_id = create_test_exercise(&pool, module_id, 2, "MD Ex PY 2").await;
    let _ex3_id = create_test_exercise(&pool, module_id, 1, "MD Ex RS 1").await;
    let _ex4_id = create_test_exercise(&pool, module_id, 1, "MD Ex PY FR 1").await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::exercises::table.find(ex1_id))
            .set((
                schema::exercises::language.eq("en"),
                schema::exercises::programming_language.eq("py"),
            ))
            .execute(conn)?;
        diesel::update(schema::exercises::table.find(ex2_id))
            .set((
                schema::exercises::language.eq("en"),
                schema::exercises::programming_language.eq("py"),
            ))
            .execute(conn)?;
        diesel::update(schema::exercises::table.find(_ex3_id))
            .set((
                schema::exercises::language.eq("en"),
                schema::exercises::programming_language.eq("rs"),
            ))
            .execute(conn)?;
        diesel::update(schema::exercises::table.find(_ex4_id))
            .set((
                schema::exercises::language.eq("fr"),
                schema::exercises::programming_language.eq("py"),
            ))
            .execute(conn)?;
        Ok::<_, diesel::result::Error>(())
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_module_data?module_id={}&language=en&programming_language=py",
            module_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ModuleDataResponse> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let data = body.data.unwrap();

    assert_eq!(data.title, "ModuleData Module");
    assert_eq!(data.order, 1);
    let mut exercise_ids = data.exercise_ids;
    exercise_ids.sort();
    assert_eq!(exercise_ids, vec![ex1_id, ex2_id]);
}

#[tokio::test]
async fn test_get_module_data_success_no_matching_exercises() {
    let (server, pool) = setup_test_environment().await;
    let course_id = create_test_course(&pool, "ModuleData NoEx Course").await;
    let module_id = create_test_module(&pool, course_id, 1, "ModuleData NoEx Module").await;
    let _ex1_id = create_test_exercise(&pool, module_id, 1, "MD NoEx PY 1").await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::exercises::table.find(_ex1_id))
            .set((
                schema::exercises::language.eq("en"),
                schema::exercises::programming_language.eq("py"),
            ))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_module_data?module_id={}&language=en&programming_language=rs",
            module_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ModuleDataResponse> = response.json();
    assert!(body.data.is_some());
    assert!(body.data.unwrap().exercise_ids.is_empty());
}

#[tokio::test]
async fn test_get_module_data_not_found_module() {
    let (server, _pool) = setup_test_environment().await;
    let non_existent_module_id = 9961;

    let response = server
        .get(&format!(
            "/student/get_module_data?module_id={}&language=en&programming_language=py",
            non_existent_module_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// get_exercise_data

#[tokio::test]
async fn test_get_exercise_data_success_basic() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 801;
    let course_id = create_test_course(&pool, "ExData Course").await;
    let game_id = create_test_game(&pool, course_id, "ExData Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExData Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "ExData Ex 1").await;
    create_test_player(&pool, player_id, "exdata@test.com", "ExData Player").await;

    let response = server
        .get(&format!(
            "/student/get_exercise_data?exercise_id={}&game_id={}&player_id={}",
            exercise_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExerciseDataResponse> = response.json();
    assert!(body.data.is_some());
    let data = body.data.unwrap();
    assert_eq!(data.title, "ExData Ex 1");
    assert!(!data.hidden);
    assert!(!data.locked);
}

#[tokio::test]
async fn test_get_exercise_data_hidden_no_unlock() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 802;
    let course_id = create_test_course(&pool, "ExData Hidden Course").await;
    let game_id = create_test_game(&pool, course_id, "ExData Hidden Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExData Hidden Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "ExData Hidden Ex 1").await;
    create_test_player(&pool, player_id, "exdata_h@test.com", "ExData Hidden P").await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::exercises::table.find(exercise_id))
            .set(schema::exercises::hidden.eq(true))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_exercise_data?exercise_id={}&game_id={}&player_id={}",
            exercise_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExerciseDataResponse> = response.json();
    let data = body.data.unwrap();
    assert!(data.hidden);
    assert!(!data.locked);
}

#[tokio::test]
async fn test_get_exercise_data_hidden_with_unlock() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 803;
    let course_id = create_test_course(&pool, "ExData HiddenU Course").await;
    let game_id = create_test_game(&pool, course_id, "ExData HiddenU Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExData HiddenU Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "ExData HiddenU Ex 1").await;
    create_test_player(&pool, player_id, "exdata_hu@test.com", "ExData HiddenU P").await;
    create_test_player_unlock(&pool, player_id, exercise_id).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::exercises::table.find(exercise_id))
            .set(schema::exercises::hidden.eq(true))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_exercise_data?exercise_id={}&game_id={}&player_id={}",
            exercise_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExerciseDataResponse> = response.json();
    let data = body.data.unwrap();
    assert!(!data.hidden);
    assert!(!data.locked);
}

#[tokio::test]
async fn test_get_exercise_data_locked_explicit_no_unlock() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 804;
    let course_id = create_test_course(&pool, "ExData Locked Course").await;
    let game_id = create_test_game(&pool, course_id, "ExData Locked Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExData Locked Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "ExData Locked Ex 1").await;
    create_test_player(&pool, player_id, "exdata_l@test.com", "ExData Locked P").await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::exercises::table.find(exercise_id))
            .set(schema::exercises::locked.eq(true))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_exercise_data?exercise_id={}&game_id={}&player_id={}",
            exercise_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExerciseDataResponse> = response.json();
    let data = body.data.unwrap();
    assert!(!data.hidden);
    assert!(data.locked);
}

#[tokio::test]
async fn test_get_exercise_data_locked_explicit_with_unlock() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 805;
    let course_id = create_test_course(&pool, "ExData LockedU Course").await;
    let game_id = create_test_game(&pool, course_id, "ExData LockedU Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExData LockedU Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "ExData LockedU Ex 1").await;
    create_test_player(&pool, player_id, "exdata_lu@test.com", "ExData LockedU P").await;
    create_test_player_unlock(&pool, player_id, exercise_id).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::exercises::table.find(exercise_id))
            .set(schema::exercises::locked.eq(true))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_exercise_data?exercise_id={}&game_id={}&player_id={}",
            exercise_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExerciseDataResponse> = response.json();
    let data = body.data.unwrap();
    assert!(!data.hidden);
    assert!(!data.locked);
}

#[tokio::test]
async fn test_get_exercise_data_locked_game_module_lock_not_met() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 806;
    let course_id = create_test_course(&pool, "ExData ModLock Course").await;
    let game_id = create_test_game(&pool, course_id, "ExData ModLock Game", 2).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExData ModLock Module").await;
    let _ex1_id = create_test_exercise(&pool, module_id, 1, "ExData ModLock Ex 1").await;
    let ex2_id = create_test_exercise(&pool, module_id, 2, "ExData ModLock Ex 2").await;
    create_test_player(&pool, player_id, "exdata_ml@test.com", "ExData ModLock P").await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::games::table.find(game_id))
            .set(schema::games::module_lock.eq(0.6))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_exercise_data?exercise_id={}&game_id={}&player_id={}",
            ex2_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExerciseDataResponse> = response.json();
    let data = body.data.unwrap();
    assert!(!data.hidden);
    assert!(data.locked);
}

#[tokio::test]
async fn test_get_exercise_data_locked_game_module_lock_met() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 807;
    let course_id = create_test_course(&pool, "ExData ModLockM Course").await;
    let game_id = create_test_game(&pool, course_id, "ExData ModLockM Game", 2).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExData ModLockM Module").await;
    let ex1_id = create_test_exercise(&pool, module_id, 1, "ExData ModLockM Ex 1").await;
    let ex2_id = create_test_exercise(&pool, module_id, 2, "ExData ModLockM Ex 2").await;
    create_test_player(&pool, player_id, "exdata_mlm@test.com", "ExData ModLockM P").await;
    create_test_player_registration(&pool, player_id, game_id).await;
    create_test_submission(&pool, player_id, game_id, ex1_id, true, 1.0).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::games::table.find(game_id))
            .set(schema::games::module_lock.eq(0.4))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_exercise_data?exercise_id={}&game_id={}&player_id={}",
            ex2_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExerciseDataResponse> = response.json();
    let data = body.data.unwrap();
    assert!(!data.hidden);
    assert!(!data.locked);
}

#[tokio::test]
async fn test_get_exercise_data_locked_game_exercise_lock_prev_not_solved() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 808;
    let course_id = create_test_course(&pool, "ExData ExLock Course").await;
    let game_id = create_test_game(&pool, course_id, "ExData ExLock Game", 2).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExData ExLock Module").await;
    let _ex1_id = create_test_exercise(&pool, module_id, 1, "ExData ExLock Ex 1").await;
    let ex2_id = create_test_exercise(&pool, module_id, 2, "ExData ExLock Ex 2").await;
    create_test_player(&pool, player_id, "exdata_el@test.com", "ExData ExLock P").await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::games::table.find(game_id))
            .set(schema::games::exercise_lock.eq(true))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_exercise_data?exercise_id={}&game_id={}&player_id={}",
            ex2_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExerciseDataResponse> = response.json();
    let data = body.data.unwrap();
    assert!(!data.hidden);
    assert!(data.locked);
}

#[tokio::test]
async fn test_get_exercise_data_locked_game_exercise_lock_prev_solved() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 809;
    let course_id = create_test_course(&pool, "ExData ExLockS Course").await;
    let game_id = create_test_game(&pool, course_id, "ExData ExLockS Game", 2).await;
    let module_id = create_test_module(&pool, course_id, 1, "ExData ExLockS Module").await;
    let ex1_id = create_test_exercise(&pool, module_id, 1, "ExData ExLockS Ex 1").await;
    let ex2_id = create_test_exercise(&pool, module_id, 2, "ExData ExLockS Ex 2").await;
    create_test_player(&pool, player_id, "exdata_els@test.com", "ExData ExLockS P").await;
    create_test_player_registration(&pool, player_id, game_id).await;
    create_test_submission(&pool, player_id, game_id, ex1_id, true, 1.0).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::games::table.find(game_id))
            .set(schema::games::exercise_lock.eq(true))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let response = server
        .get(&format!(
            "/student/get_exercise_data?exercise_id={}&game_id={}&player_id={}",
            ex2_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<ExerciseDataResponse> = response.json();
    let data = body.data.unwrap();
    assert!(!data.hidden);
    assert!(!data.locked);
}

#[tokio::test]
async fn test_get_exercise_data_not_found_exercise() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 810;
    let course_id = create_test_course(&pool, "ExData NFEx Course").await;
    let game_id = create_test_game(&pool, course_id, "ExData NFEx Game", 0).await;
    create_test_player(&pool, player_id, "exdata_nfe@test.com", "ExData NFE P").await;
    let non_existent_exercise_id = 9971;

    let response = server
        .get(&format!(
            "/student/get_exercise_data?exercise_id={}&game_id={}&player_id={}",
            non_existent_exercise_id, game_id, player_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// submit_solution

#[tokio::test]
async fn test_submit_solution_success_first_correct() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 901;
    let course_id = create_test_course(&pool, "Submit Course").await;
    let game_id = create_test_game(&pool, course_id, "Submit Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "Submit Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "Submit Ex 1").await;
    create_test_player(&pool, player_id, "submit@test.com", "Submit Player").await;
    let registration_id = create_test_player_registration(&pool, player_id, game_id).await;

    let payload = SubmitSolutionPayload {
        player_id,
        exercise_id,
        game_id,
        client: "test".to_string(),
        submitted_code: "correct".to_string(),
        metrics: json!({}),
        result: BigDecimal::from_f64(1.0).unwrap(),
        result_description: json!({"status": "pass"}),
        feedback: "".to_string(),
        entered_at: Utc::now(),
        earned_rewards: json!([]),
    };

    let response = server.post("/student/submit_solution").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.unwrap_or(false));

    let conn = pool.get().await.unwrap();
    let (sub_count, progress): (i64, i32) = conn
        .interact(move |conn| {
            let count = schema::submissions::table
                .filter(schema::submissions::player_id.eq(player_id))
                .filter(schema::submissions::exercise_id.eq(exercise_id))
                .filter(schema::submissions::game_id.eq(game_id))
                .count()
                .get_result(conn)?;
            let prog = schema::player_registrations::table
                .find(registration_id)
                .select(schema::player_registrations::progress)
                .first(conn)?;
            Ok::<_, diesel::result::Error>((count, prog))
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(sub_count, 1);
    assert_eq!(progress, 1);
}

#[tokio::test]
async fn test_submit_solution_success_subsequent_correct() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 902;
    let course_id = create_test_course(&pool, "Submit Sub Course").await;
    let game_id = create_test_game(&pool, course_id, "Submit Sub Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "Submit Sub Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "Submit Sub Ex 1").await;
    create_test_player(&pool, player_id, "submit_sub@test.com", "Submit Sub P").await;
    let registration_id = create_test_player_registration(&pool, player_id, game_id).await;
    create_test_submission(&pool, player_id, game_id, exercise_id, true, 1.0).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::player_registrations::table.find(registration_id))
            .set(schema::player_registrations::progress.eq(1))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let payload = SubmitSolutionPayload {
        player_id,
        exercise_id,
        game_id,
        client: "test".to_string(),
        submitted_code: "correct again".to_string(),
        metrics: json!({}),
        result: BigDecimal::from_f64(1.0).unwrap(),
        result_description: json!({"status": "pass"}),
        feedback: "".to_string(),
        entered_at: Utc::now(),
        earned_rewards: json!([]),
    };

    let response = server.post("/student/submit_solution").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(!body.data.unwrap_or(true));

    let conn = pool.get().await.unwrap();
    let (sub_count, progress): (i64, i32) = conn
        .interact(move |conn| {
            let count = schema::submissions::table
                .filter(schema::submissions::player_id.eq(player_id))
                .filter(schema::submissions::exercise_id.eq(exercise_id))
                .filter(schema::submissions::game_id.eq(game_id))
                .count()
                .get_result(conn)?;
            let prog = schema::player_registrations::table
                .find(registration_id)
                .select(schema::player_registrations::progress)
                .first(conn)?;
            Ok::<_, diesel::result::Error>((count, prog))
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(sub_count, 2);
    assert_eq!(progress, 1);
}

#[tokio::test]
async fn test_submit_solution_success_incorrect() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 903;
    let course_id = create_test_course(&pool, "Submit Inc Course").await;
    let game_id = create_test_game(&pool, course_id, "Submit Inc Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "Submit Inc Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "Submit Inc Ex 1").await;
    create_test_player(&pool, player_id, "submit_inc@test.com", "Submit Inc P").await;
    let registration_id = create_test_player_registration(&pool, player_id, game_id).await;

    let payload = SubmitSolutionPayload {
        player_id,
        exercise_id,
        game_id,
        client: "test".to_string(),
        submitted_code: "incorrect".to_string(),
        metrics: json!({}),
        result: BigDecimal::from_f64(0.0).unwrap(),
        result_description: json!({"status": "fail"}),
        feedback: "Try again".to_string(),
        entered_at: Utc::now(),
        earned_rewards: json!([]),
    };

    let response = server.post("/student/submit_solution").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<bool> = response.json();
    assert!(!body.data.unwrap_or(true));

    let conn = pool.get().await.unwrap();
    let (sub_count, progress): (i64, i32) = conn
        .interact(move |conn| {
            let count = schema::submissions::table
                .filter(schema::submissions::player_id.eq(player_id))
                .filter(schema::submissions::exercise_id.eq(exercise_id))
                .filter(schema::submissions::game_id.eq(game_id))
                .count()
                .get_result(conn)?;
            let prog = schema::player_registrations::table
                .find(registration_id)
                .select(schema::player_registrations::progress)
                .first(conn)?;
            Ok::<_, diesel::result::Error>((count, prog))
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(sub_count, 1);
    assert_eq!(progress, 0);
}

#[tokio::test]
async fn test_submit_solution_triggers_unlock() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 904;
    let course_id = create_test_course(&pool, "Submit Unlock Course").await;
    let game_id = create_test_game(&pool, course_id, "Submit Unlock Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "Submit Unlock Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "Submit Unlock Ex 1").await;
    create_test_player(&pool, player_id, "submit_ul@test.com", "Submit Unlock P").await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let conn = pool.get().await.unwrap();
    conn.interact(move |conn| {
        diesel::update(schema::games::table.find(game_id))
            .set(schema::games::module_lock.eq(0.1))
            .execute(conn)
    })
    .await
    .unwrap()
    .unwrap();

    let payload = SubmitSolutionPayload {
        player_id,
        exercise_id,
        game_id,
        client: "test".to_string(),
        submitted_code: "correct".to_string(),
        metrics: json!({}),
        result: BigDecimal::from_f64(1.0).unwrap(),
        result_description: json!({"status": "pass"}),
        feedback: "".to_string(),
        entered_at: Utc::now(),
        earned_rewards: json!([]),
    };

    let response = server.post("/student/submit_solution").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);

    assert!(
        check_player_unlock_exists(&pool, player_id, exercise_id).await,
        "Unlock record should exist after correct submission with game lock"
    );
}

#[tokio::test]
async fn test_submit_solution_not_found_registration() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 905;
    let course_id = create_test_course(&pool, "Submit NFReg Course").await;
    let game_id = create_test_game(&pool, course_id, "Submit NFReg Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "Submit NFReg Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "Submit NFReg Ex 1").await;
    create_test_player(&pool, player_id, "submit_nfr@test.com", "Submit NFReg P").await;

    let payload = SubmitSolutionPayload {
        player_id,
        exercise_id,
        game_id,
        client: "test".to_string(),
        submitted_code: "code".to_string(),
        metrics: json!({}),
        result: BigDecimal::from(1),
        result_description: json!({}),
        feedback: "".to_string(),
        entered_at: Utc::now(),
        earned_rewards: json!([]),
    };

    let response = server.post("/student/submit_solution").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    assert!(response.text().contains("Player registration not found"));
}

// unlock

#[tokio::test]
async fn test_unlock_success_new_unlock() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 1001;
    let course_id = create_test_course(&pool, "Unlock Course").await;
    let module_id = create_test_module(&pool, course_id, 1, "Unlock Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "Unlock Ex 1").await;
    create_test_player(&pool, player_id, "unlock@test.com", "Unlock Player").await;

    let payload = UnlockPayload {
        player_id,
        exercise_id,
    };

    let response = server.post("/student/unlock").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<()> = response.json();
    assert_eq!(body.status_code, 200);

    assert!(
        check_player_unlock_exists(&pool, player_id, exercise_id).await,
        "Unlock record should exist"
    );
}

#[tokio::test]
async fn test_unlock_success_already_unlocked() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 1002;
    let course_id = create_test_course(&pool, "Unlock AL Course").await;
    let module_id = create_test_module(&pool, course_id, 1, "Unlock AL Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "Unlock AL Ex 1").await;
    create_test_player(&pool, player_id, "unlock_al@test.com", "Unlock AL Player").await;
    create_test_player_unlock(&pool, player_id, exercise_id).await;

    let payload = UnlockPayload {
        player_id,
        exercise_id,
    };

    let response = server.post("/student/unlock").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<()> = response.json();
    assert_eq!(body.status_code, 200);

    assert!(
        check_player_unlock_exists(&pool, player_id, exercise_id).await,
        "Unlock record should still exist"
    );
}

#[tokio::test]
async fn test_unlock_not_found_player() {
    let (server, pool) = setup_test_environment().await;
    let non_existent_player_id = 9981;
    let course_id = create_test_course(&pool, "Unlock NF P Course").await;
    let module_id = create_test_module(&pool, course_id, 1, "Unlock NF P Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "Unlock NF P Ex 1").await;

    let payload = UnlockPayload {
        player_id: non_existent_player_id,
        exercise_id,
    };

    let response = server.post("/student/unlock").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    assert!(response.text().contains("Player with ID"));
}

#[tokio::test]
async fn test_unlock_not_found_exercise() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 1003;
    let non_existent_exercise_id = 9982;
    create_test_player(&pool, player_id, "unlock_nfe@test.com", "Unlock NFE Player").await;

    let payload = UnlockPayload {
        player_id,
        exercise_id: non_existent_exercise_id,
    };

    let response = server.post("/student/unlock").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    assert!(response.text().contains("Exercise with ID"));
}

// get_last_solution

#[tokio::test]
async fn test_get_last_solution_success_correct_exists() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 1101;
    let course_id = create_test_course(&pool, "LastSol Course").await;
    let game_id = create_test_game(&pool, course_id, "LastSol Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "LastSol Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "LastSol Ex 1").await;
    create_test_player(&pool, player_id, "lastsol@test.com", "LastSol Player").await;
    create_test_player_registration(&pool, player_id, game_id).await;

    create_test_submission(&pool, player_id, game_id, exercise_id, false, 0.2).await;
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    create_test_submission(&pool, player_id, game_id, exercise_id, true, 1.0).await;
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    create_test_submission(&pool, player_id, game_id, exercise_id, false, 0.5).await;

    let response = server
        .get(&format!(
            "/student/get_last_solution?player_id={}&exercise_id={}",
            player_id, exercise_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Option<LastSolutionResponse>> = response.json();
    assert_eq!(body.status_code, 200);
    assert!(body.data.is_some());
    let solution = body.data.unwrap();
    assert!(solution.is_some());
    assert_eq!(solution.unwrap().result, BigDecimal::from(100));
}

#[tokio::test]
async fn test_get_last_solution_success_only_incorrect_exists() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 1102;
    let course_id = create_test_course(&pool, "LastSol Inc Course").await;
    let game_id = create_test_game(&pool, course_id, "LastSol Inc Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "LastSol Inc Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "LastSol Inc Ex 1").await;
    create_test_player(&pool, player_id, "lastsol_inc@test.com", "LastSol Inc P").await;
    create_test_player_registration(&pool, player_id, game_id).await;

    create_test_submission(&pool, player_id, game_id, exercise_id, false, 0.1).await;
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    create_test_submission(&pool, player_id, game_id, exercise_id, false, 0.3).await;

    let response = server
        .get(&format!(
            "/student/get_last_solution?player_id={}&exercise_id={}",
            player_id, exercise_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Option<LastSolutionResponse>> = response.json();
    assert!(body.data.is_some());
    let solution = body.data.unwrap();
    assert!(solution.is_some());
    assert_eq!(solution.unwrap().result, BigDecimal::from(30));
}

#[tokio::test]
async fn test_get_last_solution_success_no_submissions() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 1103;
    let course_id = create_test_course(&pool, "LastSol None Course").await;
    let game_id = create_test_game(&pool, course_id, "LastSol None Game", 1).await;
    let module_id = create_test_module(&pool, course_id, 1, "LastSol None Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "LastSol None Ex 1").await;
    create_test_player(&pool, player_id, "lastsol_none@test.com", "LastSol None P").await;
    create_test_player_registration(&pool, player_id, game_id).await;

    let response = server
        .get(&format!(
            "/student/get_last_solution?player_id={}&exercise_id={}",
            player_id, exercise_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ApiResponse<Option<LastSolutionResponse>> = response.json();
    assert!(body.data.is_none());
}

#[tokio::test]
async fn test_get_last_solution_not_found_player() {
    let (server, pool) = setup_test_environment().await;
    let non_existent_player_id = 9991;
    let course_id = create_test_course(&pool, "LastSol NF P Course").await;
    let module_id = create_test_module(&pool, course_id, 1, "LastSol NF P Module").await;
    let exercise_id = create_test_exercise(&pool, module_id, 1, "LastSol NF P Ex 1").await;

    let response = server
        .get(&format!(
            "/student/get_last_solution?player_id={}&exercise_id={}",
            non_existent_player_id, exercise_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    assert!(response.text().contains("Player with ID"));
}

#[tokio::test]
async fn test_get_last_solution_not_found_exercise() {
    let (server, pool) = setup_test_environment().await;
    let player_id = 1104;
    let non_existent_exercise_id = 9992;
    create_test_player(&pool, player_id, "lastsol_nfe@test.com", "LastSol NFE P").await;

    let response = server
        .get(&format!(
            "/student/get_last_solution?player_id={}&exercise_id={}",
            player_id, non_existent_exercise_id
        ))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    assert!(response.text().contains("Exercise with ID"));
}
