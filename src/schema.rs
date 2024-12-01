use diesel::table;

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