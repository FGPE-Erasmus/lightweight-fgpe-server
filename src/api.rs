use crate::api::model::{ApiResponseCore, GetAvailableGamesResponse, GetCourseDataPayload, GetCourseDataResponse, GetExerciseDataPayload, GetExerciseDataResponse, GetGameMetadataPayload, GetGameMetadataResponse, GetLastSolutionPayload, GetLastSolutionResponse, GetModuleDataPayload, GetModuleDataResponse, GetPlayerGamesPayload, GetPlayerGamesResponse, JoinGamePayload, JoinGameResponse, LeaveGamePayload, LeaveGameResponse, LoadGamePayload, LoadGameResponse, SaveGamePayload, SaveGameResponse, SetGameLangPayload, SetGameLangResponse, SubmitSolutionPayload, SubmitSolutionResponse, UnlockPayload};
use crate::model::{Course, Exercise, Game, Module, NewPlayerRegistration, NewPlayerReward, NewSubmission, PlayerRegistration, PlayerUnlock, Reward, Submission};
use crate::schema::games::{active, public};
use crate::schema::playerregistrations::{game, gamestate, id, language, leftat, player, savedat};
use crate::schema::{courses, exercises, games, modules, playerregistrations, playerrewards, rewards, submissions};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use deadpool_diesel::postgres::Pool;
use diesel::{ExpressionMethods, JoinOnDsl, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper};
use crate::schema::playerunlocks;

mod model;

// private helper

type ApiResponse<T> = Json<ApiResponseCore<T>>;

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

async fn run_query<T, F>(
    pool: &deadpool_diesel::postgres::Pool,
    query: F,
) -> Result<T, (StatusCode, String)>
where
    F: FnOnce(&mut diesel::PgConnection) -> Result<T, diesel::result::Error> + Send + 'static,
    T: Send + 'static,
{
    let conn = pool.get().await.map_err(internal_error)?;
    conn.interact(query)
        .await
        .map_err(internal_error)
        .and_then(|res| res.map_err(internal_error))
}

// public api

pub async fn get_available_games(
    State(pool): State<deadpool_diesel::postgres::Pool>,
) -> ApiResponse<GetAvailableGamesResponse> {
    let games_res = run_query(&pool, |conn| {
        games::table
            .filter(active.eq(true))
            .filter(public.eq(true))
            .select(Game::as_select())
            .load(conn)
    })
    .await;
    match games_res {
        Ok(games) => Json(ApiResponseCore::ok(GetAvailableGamesResponse::new(games))),
        Err(err) => Json(ApiResponseCore::err(err)),
    }
}

pub async fn join_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<JoinGamePayload>,
) -> ApiResponse<JoinGameResponse> {
    let result = run_query(&pool, move |conn| {
        playerregistrations::table
            .filter(player.eq(payload.player_id))
            .filter(game.eq(payload.game_id))
            .select(PlayerRegistration::as_select())
            .first(conn)
    })
    .await;
    if result.is_ok() {
        return Json(ApiResponseCore::ok(JoinGameResponse::new(None)));
    }
    let date = Utc::now().date_naive();
    let registration = NewPlayerRegistration::new(
        payload.player_id,
        payload.game_id,
        payload.language.unwrap_or_else(|| "english".to_string()),
        0,
        String::new(),
        date,
        date,
        None,
    );
    let result = run_query(&pool, |conn| {
        diesel::insert_into(playerregistrations::table)
            .values(registration)
            .returning(PlayerRegistration::as_returning())
            .get_result(conn)
    })
    .await;
    match result {
        Ok(pr) => Json(ApiResponseCore::ok(JoinGameResponse::new(Some(pr.id)))),
        Err(_) => Json(ApiResponseCore::ok(JoinGameResponse::new(None))),
    }
}

pub async fn save_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<SaveGamePayload>,
) -> ApiResponse<SaveGameResponse> {
    let result = run_query(&pool, move |conn| {
        diesel::update(playerregistrations::table.find(payload.player_registration_id))
            .set((
                gamestate.eq(payload.game_state),
                savedat.eq(Utc::now().date_naive()),
            ))
            .execute(conn)
    })
    .await;
    match result {
        Ok(rows_updated) => Json(ApiResponseCore::ok(SaveGameResponse::new(
            rows_updated == 1,
        ))),
        Err(_) => Json(ApiResponseCore::ok(SaveGameResponse::new(false))),
    }
}

pub async fn load_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<LoadGamePayload>,
) -> ApiResponse<LoadGameResponse> {
    let result = run_query(&pool, move |conn| {
        playerregistrations::table
            .filter(id.eq(payload.player_registration_id))
            .select(PlayerRegistration::as_select())
            .first(conn)
    })
    .await;
    match result {
        Ok(pr) => Json(ApiResponseCore::ok(LoadGameResponse::new(pr.id, pr.gamestate))),
        Err(_) => Json(ApiResponseCore::err(
            (StatusCode::INTERNAL_SERVER_ERROR, "could not get player registration".to_string())
        )),
    }
}

pub async fn leave_game(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<LeaveGamePayload>,
) -> ApiResponse<LeaveGameResponse> {
    let registration = run_query(&pool, move |conn| {
        playerregistrations::table
            .filter(player.eq(payload.player_id))
            .filter(game.eq(payload.game_id))
            .select(PlayerRegistration::as_select())
            .first(conn)
    })
    .await;
    if let Err(err) = registration {
        return Json(ApiResponseCore::err(err));
    }
    let rows_updated = run_query(&pool, move |conn| {
        diesel::update(playerregistrations::table.find(registration.unwrap().id))
            .set(leftat.eq(Utc::now().date_naive()))
            .execute(conn)
    })
    .await;
    match rows_updated {
        Ok(rows) => Json(ApiResponseCore::ok(LeaveGameResponse::new(rows == 1))),
        Err(_) => Json(ApiResponseCore::ok(LeaveGameResponse::new(false))),
    }
}

pub async fn set_game_lang(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<SetGameLangPayload>,
) -> ApiResponse<SetGameLangResponse> {
    let game_result = run_query(&pool, move |conn| {
        games::table
            .filter(games::id.eq(payload.game_id))
            .select(Game::as_select())
            .first(conn)
    })
    .await;
    if let Err(err) = game_result {
        return Json(ApiResponseCore::err(err));
    }
    let course_result = run_query(&pool, move |conn| {
        courses::table
            .filter(courses::id.eq(game_result.unwrap().course))
            .select(Course::as_select())
            .first(conn)
    })
    .await;
    if let Err(err) = course_result {
        return Json(ApiResponseCore::err(err));
    }
    if course_result
        .unwrap()
        .languages
        .replace(" ", "")
        .split(',')
        .any(|lang| lang == payload.language)
    {
        let registration_result = run_query(&pool, move |conn| {
            playerregistrations::table
                .filter(player.eq(payload.player_id))
                .filter(game.eq(payload.game_id))
                .select(PlayerRegistration::as_select())
                .first(conn)
        })
        .await;
        if let Err(err) = registration_result {
            return Json(ApiResponseCore::err(err));
        }
        let rows_updated = run_query(&pool, move |conn| {
            diesel::update(playerregistrations::table.find(registration_result.unwrap().id))
                .set(language.eq(payload.language))
                .execute(conn)
        })
        .await;
        match rows_updated {
            Ok(rows) => Json(ApiResponseCore::ok(SetGameLangResponse::new(rows == 1))),
            Err(err) => Json(ApiResponseCore::err(err)),
        }
    } else {
        Json(ApiResponseCore::ok(SetGameLangResponse::new(false)))
    }
}

pub async fn get_player_games(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetPlayerGamesPayload>,
) -> ApiResponse<GetPlayerGamesResponse> {
    let games_result = run_query(&pool, move |conn| {
        if payload.active {
            playerregistrations::table
                .inner_join(games::table.on(game.eq(games::id)))
                .filter(player.eq(payload.player_id))
                .filter(active.eq(true))
                .filter(leftat.is_null())
                .select(id)
                .load(conn)
        } else {
            playerregistrations::table
                .filter(player.eq(payload.player_id))
                .select(id)
                .load(conn)
        }
    })
    .await;
    match games_result {
        Ok(gr) => Json(ApiResponseCore::ok(GetPlayerGamesResponse::new(gr))),
        Err(err) => Json(ApiResponseCore::err(err)),
    }
}

pub async fn get_game_metadata(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetGameMetadataPayload>,
) -> ApiResponse<GetGameMetadataResponse> {
    let metadata_result = run_query(&pool, move |conn| {
        playerregistrations::table
            .inner_join(games::table.on(game.eq(games::id)))
            .filter(id.eq(payload.player_registrations_id))
            .select((PlayerRegistration::as_select(), Game::as_select()))
            .first(conn)
    })
    .await;
    match metadata_result {
        Ok(mr) => Json(ApiResponseCore::ok(GetGameMetadataResponse::new(mr))),
        Err(err) => Json(ApiResponseCore::err(err)),
    }
}

pub async fn get_course_data(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetCourseDataPayload>,
) -> ApiResponse<GetCourseDataResponse> {
    let game_result = run_query(&pool, move |conn| {
        games::table
            .filter(games::id.eq(payload.game_id))
            .select(Game::as_select())
            .first(conn)
    })
    .await;
    if let Err(err) = game_result {
        return Json(ApiResponseCore::err(err));
    }
    let course_result = run_query(&pool, move |conn| {
        courses::table
            .filter(courses::id.eq(game_result.unwrap().course))
            .select(Course::as_select())
            .first(conn)
    })
    .await;
    if let Err(err) = course_result {
        return Json(ApiResponseCore::err(err));
    }
    let course_result = course_result.unwrap();
    let modules_result = run_query(&pool, move |conn| {
        modules::table
            .filter(modules::course.eq(course_result.id))
            .filter(modules::language.eq(payload.language))
            .select(modules::id)
            .load(conn)
    })
    .await;
    match modules_result {
        Ok(mr) => Json(ApiResponseCore::ok(GetCourseDataResponse::new(
            course_result.gamificationruleconditions,
            course_result.gamificationcomplexrules,
            course_result.gamificationruleresults,
            mr,
        ))),
        Err(err) => Json(ApiResponseCore::err(err)),
    }
}

pub async fn get_module_data(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetModuleDataPayload>,
) -> ApiResponse<GetModuleDataResponse> {
    let module = run_query(&pool, move |conn| {
        modules::table
            .find(payload.module_id)
            .select(Module::as_select())
            .first(conn)
    })
    .await;
    if let Err(err) = module {
        return Json(ApiResponseCore::err(err));
    }
    let module = module.unwrap();
    let exercises = run_query(&pool, move |conn| {
        exercises::table
            .filter(exercises::module.eq(payload.module_id))
            .filter(exercises::programminglanguage.eq(payload.programming_language))
            .filter(exercises::language.eq(payload.language))
            .select(exercises::id)
            .load(conn)
    })
    .await;
    match exercises {
        Ok(ex) => Json(ApiResponseCore::ok(GetModuleDataResponse::new(
            module.order,
            module.title,
            module.description,
            module.startdate,
            module.enddate,
            ex,
        ))),
        Err(err) => Json(ApiResponseCore::err(err)),
    }
}

pub async fn get_exercise_data(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetExerciseDataPayload>,
) -> ApiResponse<GetExerciseDataResponse> {
    let ex: Result<Exercise, _> = run_query(
        &pool, move |conn| {
            exercises::table
                .filter(exercises::id.eq(payload.exercise_id))
                .inner_join(modules::table.on(exercises::module.eq(modules::id)))
                .inner_join(games::table.on(modules::course.eq(games::id)))
                .inner_join(playerregistrations::table.on(games::id.eq(playerregistrations::game)))
                .filter(playerregistrations::player.eq(payload.player_id))
                .filter(games::id.eq(payload.game_id))
                .select(Exercise::as_select())
                .distinct()
                .first::<Exercise>(conn)
        }
    ).await;
    match ex {
        Err(err) => Json(ApiResponseCore::err(err)),
        Ok(exercise) => {
            let res = run_query(&pool, move |conn| {
                playerunlocks::table
                    .filter(playerunlocks::player.eq(payload.player_id))
                    .filter(playerunlocks::exercise.eq(payload.exercise_id))
                    .first::<PlayerUnlock>(conn)
                    .optional()
            }).await;
            match res {
                Err(err) => Json(ApiResponseCore::err(err)),
                Ok(optional_pu) => {
                    let game_res = run_query(&pool, move |conn| {
                        games::table
                            .filter(games::id.eq(payload.game_id))
                            .first::<Game>(conn)
                    }).await;
                    if let Err(err) = game_res {
                        return Json(ApiResponseCore::err(err));
                    }
                    let final_game = game_res.unwrap();
                    let module_lock = module_lock_check(&pool, exercise.module, payload.player_id, final_game.modulelock).await;
                    if module_lock.status_code != 200 {
                        return Json(ApiResponseCore::err((StatusCode::INTERNAL_SERVER_ERROR, "could not query module_lock".to_string())));
                    }
                    let mut final_ex = exercise;
                    final_ex.hidden = optional_pu.is_none() && final_ex.hidden;
                    final_ex.locked = (final_ex.locked || final_game.exerciselock || module_lock.data
                        .expect("api contract")) && optional_pu.is_none();
                    Json(ApiResponseCore::ok(GetExerciseDataResponse::new(final_ex)))
                }
            }
        }
    }
}

async fn module_lock_check(pool: &Pool, module_id: i32, player_id: i32, module_lock: f32) -> ApiResponse<bool> {
    let module_ex_count = run_query(&pool, move |conn| {
        exercises::table
            .filter(exercises::module.eq(module_id))
            .count()
            .get_result::<i64>(conn)
    }).await;
    if let Err(err) = module_ex_count {
        return Json(ApiResponseCore::err(err));
    }
    let module_finished_ex_count = run_query(pool, move |conn| {
        exercises::table
            .inner_join(submissions::table.on(submissions::exercise.eq(exercises::id)))
            .filter(exercises::module.eq(module_id))
            .filter(submissions::player.eq(player_id))
            .filter(submissions::feedback.eq("OK"))
            .select(exercises::id)
            .distinct()
            .count()
            .get_result::<i64>(conn)
    }).await;
    match module_finished_ex_count {
        Ok(finished_count) => Json(ApiResponseCore::ok(finished_count / module_ex_count.unwrap() >= module_lock as i64)),
        Err(err) => Json(ApiResponseCore::err(err))
    }
}

pub async fn submit_solution(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<SubmitSolutionPayload>,
) -> ApiResponse<SubmitSolutionResponse> {
    let sub: Result<Option<Submission>, _> = run_query(
        &pool, move |conn| {
            submissions::table
                .filter(submissions::player.eq(payload.player_id))
                .filter(submissions::exercise.eq(payload.exercise_id))
                .first::<Submission>(conn)
                .optional()
        }
    ).await;
    let first = match sub {
        Err(err) => return Json(ApiResponseCore::err(err)),
        Ok(opt_sub) => opt_sub.is_none()
    };
    let earned_rewards = payload.submission_earned_rewards.clone();
    let new_submission = NewSubmission::new(
        payload.exercise_id,
        payload.player_id,
        payload.submission_client,
        payload.submission_submitted_code,
        payload.submission_metrics,
        payload.submission_result,
        payload.submission_result_description,
        payload.submission_feedback,
        payload.submission_earned_rewards,
        payload.submission_entered_at,
        Utc::now().date_naive()
    );
    let submission_res = run_query(&pool, move |conn| {
        diesel::insert_into(submissions::table)
            .values(new_submission)
            .execute(conn)
    }).await;
    if let Err(err) = submission_res {
        return Json(ApiResponseCore::err(err));
    }

    let resp = if first {
        let pr = run_query(&pool, move |conn| {
            playerregistrations::table
                .filter(playerregistrations::player.eq(payload.player_id))
                .first::<PlayerRegistration>(conn)
        }).await;
        if let Err(err) = pr {
            return Json(ApiResponseCore::err(err));
        }
        let pr = pr.unwrap();
        let res = run_query(&pool, move |conn| {
            diesel::update(playerregistrations::table.find(pr.id))
                .set(playerregistrations::progress.eq(pr.progress + 1))
                .execute(conn)
        }).await;
        if let Err(err) = res {
            return Json(ApiResponseCore::err(err));
        }
        for reward_str in earned_rewards.split(",").map(|v| v.to_string()) {
            let reward: Result<Reward, _> = run_query(
                &pool, move |conn| {
                    rewards::table
                        .filter(rewards::name.eq(reward_str))
                        .first::<Reward>(conn)
                }).await;
            if let Err(err) = reward {
                return Json(ApiResponseCore::err(err));
            }
            let reward = reward.unwrap();
            let d = Utc::now().date_naive();
            let player_reward = NewPlayerReward::new(
                pr.player, reward.id, Some(pr.game), 1, 0,
                d, d + reward.validperiod
            );
            let player_reward_res = run_query(&pool, move |conn| {
                diesel::insert_into(playerrewards::table)
                    .values(player_reward)
                    .execute(conn)
            }).await;
            if let Err(err) = player_reward_res {
                return Json(ApiResponseCore::err(err));
            }
        }

        let g: Result<Game, _> = run_query(
            &pool, move |conn| {
                games::table
                    .filter(games::id.eq(pr.game))
                    .first::<Game>(conn)
            }).await;
        if let Err(err) = g {
            return Json(ApiResponseCore::err(err));
        }
        let final_game = g.unwrap();

        let ex: Result<Exercise, _> = run_query(
            &pool, move |conn| {
                exercises::table
                    .filter(exercises::id.eq(payload.exercise_id)).first::<Exercise>(conn)
            }
        ).await;
        if let Err(err) = ex {
            return Json(ApiResponseCore::err(err));
        }

        let module_lock = module_lock_check(&pool, ex.unwrap().module, payload.player_id, final_game.modulelock).await;
        if final_game.exerciselock || module_lock.data.expect("api contract") {
            let unlock_res = unlock_internal(
                &pool, payload.player_id, payload.exercise_id
            ).await;
            if unlock_res.status_code != 200 {
                return Json(ApiResponseCore::err(
                    (StatusCode::INTERNAL_SERVER_ERROR, "could not unlock exercise".to_string())
                ));
            }
        }
        true
    } else {
        false
    };
    Json(ApiResponseCore::ok(SubmitSolutionResponse::new(resp)))
}

pub async fn unlock(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<UnlockPayload>,
) -> ApiResponse<()> {
    unlock_internal(&pool, payload.player_id, payload.exercise_id).await
}

async fn unlock_internal(pool: &Pool, player_id: i32, exercise_id: i32) -> ApiResponse<()> {
    let pu = PlayerUnlock::new(
        player_id, exercise_id, Utc::now().date_naive()
    );
    let res = run_query(&pool, move |conn| {
        diesel::insert_into(playerunlocks::table)
            .values(pu)
            .execute(conn)
    }).await;
    match res {
        Err(err) => Json(ApiResponseCore::err(err)),
        Ok(_) => Json(ApiResponseCore::ok(()))
    }
}

pub async fn get_last_solution(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<GetLastSolutionPayload>,
) -> ApiResponse<GetLastSolutionResponse> {
    let ok_submission: Result<Option<Submission>, _> = run_query(
        &pool, move |conn| {
            submissions::table
                .filter(submissions::player.eq(payload.player_id))
                .filter(submissions::exercise.eq(payload.exercise_id))
                .filter(submissions::feedback.eq("OK"))
                .first::<Submission>(conn)
                .optional()
        },
    ).await;
    match ok_submission {
        Ok(Some(sub)) => Json(ApiResponseCore::ok(GetLastSolutionResponse::new(Some(sub)))),
        Ok(None) => {
            let recent_submission: Result<Option<Submission>, _> = run_query(
                &pool, move |conn| {
                    submissions::table
                        .filter(submissions::player.eq(payload.player_id))
                        .filter(submissions::exercise.eq(payload.exercise_id))
                        .order_by(submissions::submittedat.desc())
                        .first::<Submission>(conn)
                        .optional()
                },
            ).await;
            match recent_submission {
                Ok(sub) => Json(ApiResponseCore::ok(GetLastSolutionResponse::new(sub))),
                Err(err) => Json(ApiResponseCore::err(err)),
            }
        }
        Err(err) => Json(ApiResponseCore::err(err)),
    }
}