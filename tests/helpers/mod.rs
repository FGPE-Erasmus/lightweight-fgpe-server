use crate::schema::{player_groups::dsl as pg_dsl, player_registrations::dsl as pr_dsl};
use axum::Router;
pub(crate) use axum_test::TestServer;
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::Utc;
pub(crate) use deadpool_diesel::postgres::{
    Manager as TestManager, Pool as TestPool, Runtime as TestRuntime,
};
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use lightweight_fgpe_server::model::editor::{NewCourse, NewExercise, NewModule};
use lightweight_fgpe_server::model::student::NewPlayerUnlock;
use lightweight_fgpe_server::model::student::{NewPlayerRegistration, NewSubmission};
use lightweight_fgpe_server::model::teacher::{
    NewGame, NewGameOwnership, NewGroupOwnership, NewInvite, NewPlayerGroup,
};
use lightweight_fgpe_server::schema::player_unlocks::dsl as pu_dsl;
use lightweight_fgpe_server::{init_test_router, schema};
use serde_json::json;
use uuid::Uuid;

// test structs

#[derive(Insertable)]
#[diesel(table_name = schema::instructors)]
struct TestNewInstructor<'a> {
    pub id: i64,
    pub email: &'a str,
    pub display_name: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = schema::players)]
struct TestNewPlayer<'a> {
    pub id: i64,
    pub email: &'a str,
    pub display_name: &'a str,
    pub display_avatar: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = schema::groups)]
struct TestNewGroup<'a> {
    pub id: i64,
    pub display_name: &'a str,
    pub display_avatar: Option<String>,
}

// test infra setup

pub fn get_test_db_pool() -> TestPool {
    let db_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:admin@localhost:5432/fgpe-test".to_string());

    let manager = TestManager::new(&db_url, TestRuntime::Tokio1);
    TestPool::builder(manager)
        .max_size(15)
        .build()
        .expect("Failed to create test database pool")
}

pub async fn setup_test_environment() -> (TestServer, TestPool) {
    let test_pool = get_test_db_pool();
    clear_test_database(&test_pool).await;
    let app: Router = init_test_router(test_pool.clone());
    let server = TestServer::new(app).expect("Failed to create TestServer");
    (server, test_pool)
}

async fn clear_test_database(pool: &TestPool) {
    println!("Attempting to clear test database...");
    let conn = pool.get().await.expect("Failed to get conn for cleanup");
    conn.interact(|conn| {
        conn.transaction::<_, DieselError, _>(|tx_conn| {
            diesel::delete(schema::submissions::table).execute(tx_conn)?;
            diesel::delete(schema::player_rewards::table).execute(tx_conn)?;
            diesel::delete(schema::player_unlocks::table).execute(tx_conn)?;
            diesel::delete(schema::player_registrations::table).execute(tx_conn)?;
            diesel::delete(schema::player_groups::table).execute(tx_conn)?;
            diesel::delete(schema::invites::table).execute(tx_conn)?;
            diesel::delete(schema::game_ownership::table).execute(tx_conn)?;
            diesel::delete(schema::course_ownership::table).execute(tx_conn)?;
            diesel::delete(schema::exercises::table).execute(tx_conn)?;
            diesel::delete(schema::rewards::table).execute(tx_conn)?;
            diesel::delete(schema::games::table).execute(tx_conn)?;
            diesel::delete(schema::modules::table).execute(tx_conn)?;
            diesel::delete(schema::courses::table).execute(tx_conn)?;
            diesel::delete(schema::group_ownership::table).execute(tx_conn)?;
            diesel::delete(schema::instructors::table).execute(tx_conn)?;
            diesel::delete(schema::groups::table).execute(tx_conn)?;
            diesel::delete(schema::players::table).execute(tx_conn)?;
            Ok(())
        })
    })
    .await
    .expect("Database interaction failed during cleanup")
    .expect("Diesel cleanup transaction failed");
    println!("Finished clearing test database tables.");
}

// endpoint helpers

pub async fn create_test_instructor(
    pool: &TestPool,
    id: i64,
    email: &'static str,
    name: &'static str,
) -> i64 {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for instructor insert");
    conn.interact(move |conn| {
        let new_instructor = TestNewInstructor {
            id,
            email,
            display_name: name,
        };
        diesel::insert_into(schema::instructors::table)
            .values(&new_instructor)
            .on_conflict(schema::instructors::id)
            .do_update()
            .set((
                schema::instructors::email.eq(new_instructor.email),
                schema::instructors::display_name.eq(new_instructor.display_name),
            ))
            .returning(schema::instructors::id)
            .get_result(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test instructor")
}

pub async fn create_test_course(pool: &TestPool, title: &str) -> i64 {
    let title_string = title.to_string();
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for course insert");
    conn.interact(move |conn| {
        let new_course = NewCourse {
            title: title_string,
            description: "Test Desc".to_string(),
            languages: "en".to_string(),
            programming_languages: "py,rust".to_string(),
            gamification_rule_conditions: "{}".to_string(),
            gamification_complex_rules: "{}".to_string(),
            gamification_rule_results: "{}".to_string(),
            public: false,
        };
        diesel::insert_into(schema::courses::table)
            .values(&new_course)
            .returning(schema::courses::id)
            .get_result(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test course")
}

pub async fn create_test_game(
    pool: &TestPool,
    course_id: i64,
    title: &str,
    total_exercises: i32,
) -> i64 {
    let title_string = title.to_string();
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for game insert");
    conn.interact(move |conn| {
        let new_game = NewGame {
            title: title_string,
            public: false,
            active: true,
            description: "Test Game Desc".to_string(),
            course_id,
            programming_language: "py".to_string(),
            module_lock: 0.0,
            exercise_lock: false,
            total_exercises,
            start_date: Utc::now(),
            end_date: Utc::now() + chrono::Duration::days(30),
        };
        diesel::insert_into(schema::games::table)
            .values(&new_game)
            .returning(schema::games::id)
            .get_result(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test game")
}

pub async fn create_test_game_ownership(
    pool: &TestPool,
    instructor_id: i64,
    game_id: i64,
    owner: bool,
) {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for ownership insert");
    conn.interact(move |conn| {
        let new_ownership = NewGameOwnership {
            game_id,
            instructor_id,
            owner,
        };
        diesel::insert_into(schema::game_ownership::table)
            .values(&new_ownership)
            .on_conflict((
                schema::game_ownership::game_id,
                schema::game_ownership::instructor_id,
            ))
            .do_nothing()
            .execute(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test game ownership");
}

pub async fn create_test_player(
    pool: &TestPool,
    id: i64,
    email: &'static str,
    name: &'static str,
) -> i64 {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for player insert");
    conn.interact(move |conn| {
        let new_player = TestNewPlayer {
            id,
            email,
            display_name: name,
            display_avatar: None,
        };
        diesel::insert_into(schema::players::table)
            .values(&new_player)
            .on_conflict(schema::players::id)
            .do_update()
            .set((
                schema::players::email.eq(new_player.email),
                schema::players::display_name.eq(new_player.display_name),
            ))
            .returning(schema::players::id)
            .get_result(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test player")
}

pub async fn create_test_player_registration(pool: &TestPool, player_id: i64, game_id: i64) -> i64 {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for registration insert");
    conn.interact(move |conn| {
        let new_registration = NewPlayerRegistration {
            player_id,
            game_id,
            language: "en".to_string(),
            progress: 0,
            game_state: json!({}),
        };
        diesel::insert_into(schema::player_registrations::table)
            .values(&new_registration)
            .on_conflict((
                schema::player_registrations::player_id,
                schema::player_registrations::game_id,
            ))
            .do_nothing()
            .returning(schema::player_registrations::id)
            .get_result::<i64>(conn)
            .optional()
            .map(|opt| opt.unwrap_or(-1))
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert or handle conflict for test player registration")
}

pub async fn create_test_group_with_id(pool: &TestPool, id: i64, name: &'static str) -> i64 {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for group insert");
    conn.interact(move |conn| {
        let new_group = TestNewGroup {
            id,
            display_name: name,
            display_avatar: None,
        };
        diesel::insert_into(schema::groups::table)
            .values(&new_group)
            .on_conflict(schema::groups::id)
            .do_update()
            .set(schema::groups::display_name.eq(new_group.display_name))
            .returning(schema::groups::id)
            .get_result(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test group with ID")
}

pub async fn create_test_group_ownership(
    pool: &TestPool,
    instructor_id: i64,
    group_id: i64,
    owner: bool,
) {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for group ownership insert");
    conn.interact(move |conn| {
        let new_ownership = NewGroupOwnership {
            group_id,
            instructor_id,
            owner,
        };
        diesel::insert_into(schema::group_ownership::table)
            .values(&new_ownership)
            .on_conflict((
                schema::group_ownership::group_id,
                schema::group_ownership::instructor_id,
            ))
            .do_update()
            .set(schema::group_ownership::owner.eq(owner))
            .execute(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test group ownership");
}

pub async fn add_player_to_group(pool: &TestPool, player_id: i64, group_id: i64) {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for player_group insert");
    conn.interact(move |conn| {
        let new_player_group = NewPlayerGroup {
            player_id,
            group_id,
        };
        diesel::insert_into(schema::player_groups::table)
            .values(&new_player_group)
            .on_conflict((
                schema::player_groups::player_id,
                schema::player_groups::group_id,
            ))
            .do_nothing()
            .execute(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test player_group link");
}

pub async fn update_player_status(pool: &TestPool, player_id: i64, disabled: bool) {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for player update");
    conn.interact(move |conn| {
        diesel::update(schema::players::table.find(player_id))
            .set(schema::players::disabled.eq(disabled))
            .execute(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to update player status");
}

pub async fn create_test_module(pool: &TestPool, course_id: i64, order: i32, title: &str) -> i64 {
    let title_string = title.to_string();
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for module insert");
    conn.interact(move |conn| {
        let new_module = NewModule {
            course_id,
            order,
            title: title_string,
            description: "Test Module Desc".to_string(),
            language: "en".to_string(),
            start_date: Utc::now(),
            end_date: Utc::now() + chrono::Duration::days(30),
        };
        diesel::insert_into(schema::modules::table)
            .values(&new_module)
            .returning(schema::modules::id)
            .get_result(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test module")
}

pub async fn create_test_exercise(pool: &TestPool, module_id: i64, order: i32, title: &str) -> i64 {
    let title_string = title.to_string();
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for exercise insert");
    conn.interact(move |conn| {
        let new_exercise = NewExercise {
            version: BigDecimal::from(1),
            module_id,
            order,
            title: title_string,
            description: "Test Exercise Desc".to_string(),
            language: "en".to_string(),
            programming_language: "py".to_string(),
            init_code: "".to_string(),
            pre_code: "".to_string(),
            post_code: "".to_string(),
            test_code: "".to_string(),
            check_source: "".to_string(),
            hidden: false,
            locked: false,
            mode: "code".to_string(),
            mode_parameters: json!({}),
            difficulty: "easy".to_string(),
        };
        diesel::insert_into(schema::exercises::table)
            .values(&new_exercise)
            .returning(schema::exercises::id)
            .get_result(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test exercise")
}

pub async fn create_test_submission(
    pool: &TestPool,
    player_id: i64,
    game_id: i64,
    exercise_id: i64,
    first_solution: bool,
    result: f64,
) -> i64 {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for submission insert");
    let result_bd = BigDecimal::try_from(result * 100.0).unwrap_or_else(|_| BigDecimal::from(0));

    conn.interact(move |conn| {
        let new_submission = NewSubmission {
            exercise_id,
            game_id,
            player_id,
            client: "test_client".to_string(),
            submitted_code: "print('test')".to_string(),
            metrics: json!({}),
            result: result_bd,
            result_description: json!({"status": if result >= 0.5 {"pass"} else {"fail"}}),
            first_solution,
            feedback: "".to_string(),
            earned_rewards: json!([]),
            entered_at: Utc::now(),
        };
        diesel::insert_into(schema::submissions::table)
            .values(&new_submission)
            .returning(schema::submissions::id)
            .get_result(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test submission")
}

pub async fn create_test_invite(
    pool: &TestPool,
    instructor_id: i64,
    game_id: Option<i64>,
    group_id: Option<i64>,
) -> Uuid {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for invite insert");
    let new_uuid = Uuid::new_v4();
    conn.interact(move |conn| {
        let new_invite = NewInvite {
            uuid: new_uuid,
            instructor_id,
            game_id,
            group_id,
        };
        diesel::insert_into(schema::invites::table)
            .values(&new_invite)
            .returning(schema::invites::uuid)
            .get_result(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test invite")
}

pub async fn check_player_in_game(pool: &TestPool, player_id: i64, game_id: i64) -> bool {
    let conn = pool.get().await.expect("Failed to get conn for game check");
    conn.interact(move |conn| {
        pr_dsl::player_registrations
            .filter(pr_dsl::player_id.eq(player_id))
            .filter(pr_dsl::game_id.eq(game_id))
            .filter(pr_dsl::left_at.is_null())
            .select(count_star())
            .get_result::<i64>(conn)
            .map(|count| count > 0)
    })
    .await
    .expect("Interact failed for game check")
    .expect("DB query failed for game check")
}

pub async fn check_player_in_group(pool: &TestPool, player_id: i64, group_id: i64) -> bool {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for group check");
    conn.interact(move |conn| {
        pg_dsl::player_groups
            .filter(pg_dsl::player_id.eq(player_id))
            .filter(pg_dsl::group_id.eq(group_id))
            .filter(pg_dsl::left_at.is_null())
            .select(count_star())
            .get_result::<i64>(conn)
            .map(|count| count > 0)
    })
    .await
    .expect("Interact failed for group check")
    .expect("DB query failed for group check")
}

pub async fn count_player_game_registrations(pool: &TestPool, player_id: i64) -> i64 {
    let conn = pool.get().await.expect("Failed to get conn for game count");
    conn.interact(move |conn| {
        pr_dsl::player_registrations
            .filter(pr_dsl::player_id.eq(player_id))
            .filter(pr_dsl::left_at.is_null())
            .select(count_star())
            .get_result::<i64>(conn)
    })
    .await
    .expect("Interact failed for game count")
    .expect("DB query failed for game count")
}

pub async fn count_player_group_memberships(pool: &TestPool, player_id: i64) -> i64 {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for group count");
    conn.interact(move |conn| {
        pg_dsl::player_groups
            .filter(pg_dsl::player_id.eq(player_id))
            .filter(pg_dsl::left_at.is_null())
            .select(count_star())
            .get_result::<i64>(conn)
    })
    .await
    .expect("Interact failed for group count")
    .expect("DB query failed for group count")
}

pub async fn create_test_player_unlock(pool: &TestPool, player_id: i64, exercise_id: i64) {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for unlock insert");
    conn.interact(move |conn| {
        let new_unlock = NewPlayerUnlock {
            player_id,
            exercise_id,
        };
        diesel::insert_into(schema::player_unlocks::table)
            .values(&new_unlock)
            .on_conflict((
                schema::player_unlocks::player_id,
                schema::player_unlocks::exercise_id,
            ))
            .do_nothing()
            .execute(conn)
    })
    .await
    .expect("Interact failed")
    .expect("Failed to insert test player unlock");
}

pub async fn check_player_unlock_exists(pool: &TestPool, player_id: i64, exercise_id: i64) -> bool {
    let conn = pool
        .get()
        .await
        .expect("Failed to get conn for unlock check");
    conn.interact(move |conn| {
        pu_dsl::player_unlocks
            .filter(pu_dsl::player_id.eq(player_id))
            .filter(pu_dsl::exercise_id.eq(exercise_id))
            .select(count_star())
            .get_result::<i64>(conn)
            .map(|count| count > 0)
    })
    .await
    .expect("Interact failed for unlock check")
    .expect("DB query failed for unlock check")
}
