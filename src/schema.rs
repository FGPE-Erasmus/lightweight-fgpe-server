use diesel::table;

table! {
    courses (id) {
        id -> Integer,
        title -> Text,
        description -> Text,
        languages -> Text,
        programming_languages -> Text,
        gamification_rule_conditions -> Text,
        gamification_complex_rules -> Text,
        gamification_rule_results -> Text,
        created_at -> Date,
        updated_at -> Date,
    }
}

table! {
    games (id) {
        id -> Integer,
        title -> Text,
        public -> Bool,
        active -> Bool,
        description -> Text,
        course -> Integer,
        programming_language -> Text,
        module_lock -> Float,
        exercise_lock -> Bool,
        total_exercises -> Integer,
        start_date -> Date,
        end_date -> Date,
        created_at -> Date,
        updated_at -> Date,
    }
}

table! {
    modules (id) {
        id -> Integer,
        course -> Integer,
        order -> Integer,
        title -> Text,
        description -> Text,
        language -> Text,
        start_date -> Date,
        end_date -> Date,
    }
}

table! {
    exercises (id) {
        id -> Integer,
        version -> Integer,
        module -> Integer,
        order -> Integer,
        title -> Text,
        description -> Text,
        language -> Text,
        programming_language -> Text,
        init_code -> Text,
        pre_code -> Text,
        post_code -> Text,
        test_code -> Text,
        check_source -> Text,
        hidden -> Bool,
        locked -> Bool,
        mode -> Text,
        mode_parameters -> Text,
        difficulty -> Text,
        created_at -> Date,
        updated_at -> Date,
    }
}

table! {
    submissions (id) {
        id -> Integer,
        exercise -> Integer,
        player -> Integer,
        client -> Text,
        submitted_code -> Text,
        metrics -> Text,
        result -> Double,
        result_description -> Text,
        feedback -> Text,
        earned_rewards -> Text,
        entered_at -> Date,
        submitted_at -> Date,
    }
}

table! {
    players (id) {
        id -> Integer,
        email -> Text,
        display_name -> Text,
        display_avatar -> Text,
        points -> Integer,
        created_at -> Date,
        last_active -> Date,
    }
}

table! {
    groups (id) {
        id -> Integer,
        display_name -> Text,
        display_avatar -> Text,
    }
}

table! {
    player_groups (player, group) {
        player -> Integer,
        group -> Integer,
        joined_at -> Date,
        left_at -> Date,
    }
}

table! {
    player_registrations (id) {
        id -> Integer,
        player -> Integer,
        game -> Integer,
        language -> Text,
        progress -> Integer,
        game_state -> Text,
        saved_at -> Date,
        joined_at -> Date,
        left_at -> Nullable<Date>,
    }
}

table! {
    player_unlocks (player, exercise) {
        player -> Integer,
        exercise -> Integer,
        unlocked_at -> Date,
    }
}

table! {
    rewards (id) {
        id -> Integer,
        course -> Integer,
        name -> Text,
        description -> Text,
        message_when_won -> Text,
        image_url -> Text,
    }
}

table! {
    player_rewards (player, reward, game) {
        player -> Integer,
        reward -> Integer,
        game -> Integer,
        count -> Integer,
        used_count -> Integer,
        obtained_at -> Date,
        expires_at -> Date,
    }
}
