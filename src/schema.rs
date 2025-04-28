// @generated automatically by Diesel CLI.

diesel::table! {
    course_ownership (course_id, instructor_id) {
        course_id -> Int8,
        instructor_id -> Int8,
        owner -> Bool,
    }
}

diesel::table! {
    courses (id) {
        id -> Int8,
        #[max_length = 255]
        title -> Varchar,
        description -> Text,
        languages -> Text,
        programming_languages -> Text,
        gamification_rule_conditions -> Text,
        gamification_complex_rules -> Text,
        gamification_rule_results -> Text,
        public -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    exercises (id) {
        id -> Int8,
        version -> Numeric,
        module_id -> Int8,
        order -> Int4,
        #[max_length = 255]
        title -> Varchar,
        description -> Text,
        #[max_length = 10]
        language -> Varchar,
        #[max_length = 100]
        programming_language -> Varchar,
        init_code -> Text,
        pre_code -> Text,
        post_code -> Text,
        test_code -> Text,
        check_source -> Text,
        hidden -> Bool,
        locked -> Bool,
        #[max_length = 50]
        mode -> Varchar,
        mode_parameters -> Jsonb,
        #[max_length = 50]
        difficulty -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    game_ownership (game_id, instructor_id) {
        game_id -> Int8,
        instructor_id -> Int8,
        owner -> Bool,
    }
}

diesel::table! {
    games (id) {
        id -> Int8,
        #[max_length = 255]
        title -> Varchar,
        public -> Bool,
        active -> Bool,
        description -> Text,
        course_id -> Int8,
        #[max_length = 100]
        programming_language -> Varchar,
        module_lock -> Float8,
        exercise_lock -> Bool,
        total_exercises -> Int4,
        start_date -> Timestamptz,
        end_date -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    group_ownership (group_id, instructor_id) {
        group_id -> Int8,
        instructor_id -> Int8,
        owner -> Bool,
    }
}

diesel::table! {
    groups (id) {
        id -> Int8,
        #[max_length = 100]
        display_name -> Varchar,
        display_avatar -> Nullable<Text>,
    }
}

diesel::table! {
    instructors (id) {
        id -> Int8,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 100]
        display_name -> Varchar,
        display_avatar -> Nullable<Text>,
        created_at -> Timestamptz,
        last_active -> Timestamptz,
    }
}

diesel::table! {
    invites (id) {
        id -> Int8,
        uuid -> Uuid,
        instructor_id -> Int8,
        game_id -> Nullable<Int8>,
        group_id -> Nullable<Int8>,
    }
}

diesel::table! {
    modules (id) {
        id -> Int8,
        course_id -> Int8,
        order -> Int4,
        #[max_length = 255]
        title -> Varchar,
        description -> Text,
        #[max_length = 10]
        language -> Varchar,
        start_date -> Timestamptz,
        end_date -> Timestamptz,
    }
}

diesel::table! {
    player_groups (player_id, group_id) {
        player_id -> Int8,
        group_id -> Int8,
        joined_at -> Timestamptz,
        left_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    player_registrations (id) {
        id -> Int8,
        player_id -> Int8,
        game_id -> Int8,
        #[max_length = 10]
        language -> Varchar,
        progress -> Int4,
        game_state -> Jsonb,
        saved_at -> Timestamptz,
        joined_at -> Timestamptz,
        left_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    player_rewards (id) {
        id -> Int8,
        player_id -> Int8,
        reward_id -> Int8,
        game_id -> Nullable<Int8>,
        count -> Int4,
        used_count -> Int4,
        obtained_at -> Timestamptz,
        expires_at -> Timestamptz,
    }
}

diesel::table! {
    player_unlocks (player_id, exercise_id) {
        player_id -> Int8,
        exercise_id -> Int8,
        unlocked_at -> Timestamptz,
    }
}

diesel::table! {
    players (id) {
        id -> Int8,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 100]
        display_name -> Varchar,
        display_avatar -> Nullable<Text>,
        points -> Int4,
        created_at -> Timestamptz,
        last_active -> Timestamptz,
        disabled -> Bool,
    }
}

diesel::table! {
    rewards (id) {
        id -> Int8,
        course_id -> Int8,
        #[max_length = 255]
        name -> Varchar,
        description -> Text,
        message_when_won -> Text,
        image_url -> Nullable<Text>,
        valid_period -> Nullable<Interval>,
    }
}

diesel::table! {
    submissions (id) {
        id -> Int8,
        exercise_id -> Int8,
        game_id -> Int8,
        player_id -> Int8,
        #[max_length = 255]
        client -> Varchar,
        submitted_code -> Text,
        metrics -> Jsonb,
        result -> Numeric,
        result_description -> Jsonb,
        first_solution -> Bool,
        feedback -> Text,
        earned_rewards -> Jsonb,
        entered_at -> Timestamptz,
        submitted_at -> Timestamptz,
    }
}

diesel::joinable!(course_ownership -> courses (course_id));
diesel::joinable!(course_ownership -> instructors (instructor_id));
diesel::joinable!(exercises -> modules (module_id));
diesel::joinable!(game_ownership -> games (game_id));
diesel::joinable!(game_ownership -> instructors (instructor_id));
diesel::joinable!(games -> courses (course_id));
diesel::joinable!(group_ownership -> groups (group_id));
diesel::joinable!(group_ownership -> instructors (instructor_id));
diesel::joinable!(invites -> games (game_id));
diesel::joinable!(invites -> groups (group_id));
diesel::joinable!(invites -> instructors (instructor_id));
diesel::joinable!(modules -> courses (course_id));
diesel::joinable!(player_groups -> groups (group_id));
diesel::joinable!(player_groups -> players (player_id));
diesel::joinable!(player_registrations -> games (game_id));
diesel::joinable!(player_registrations -> players (player_id));
diesel::joinable!(player_rewards -> games (game_id));
diesel::joinable!(player_rewards -> players (player_id));
diesel::joinable!(player_rewards -> rewards (reward_id));
diesel::joinable!(player_unlocks -> exercises (exercise_id));
diesel::joinable!(player_unlocks -> players (player_id));
diesel::joinable!(rewards -> courses (course_id));
diesel::joinable!(submissions -> exercises (exercise_id));
diesel::joinable!(submissions -> games (game_id));
diesel::joinable!(submissions -> players (player_id));

diesel::allow_tables_to_appear_in_same_query!(
    course_ownership,
    courses,
    exercises,
    game_ownership,
    games,
    group_ownership,
    groups,
    instructors,
    invites,
    modules,
    player_groups,
    player_registrations,
    player_rewards,
    player_unlocks,
    players,
    rewards,
    submissions,
);
