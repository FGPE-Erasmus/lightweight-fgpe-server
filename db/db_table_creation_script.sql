DROP TABLE IF EXISTS group_ownership CASCADE;
DROP TABLE IF EXISTS course_ownership CASCADE;
DROP TABLE IF EXISTS game_ownership CASCADE;
DROP TABLE IF EXISTS player_rewards CASCADE;
DROP TABLE IF EXISTS player_unlocks CASCADE;
DROP TABLE IF EXISTS player_registrations CASCADE;
DROP TABLE IF EXISTS player_groups CASCADE;
DROP TABLE IF EXISTS submissions CASCADE;
DROP TABLE IF EXISTS invites CASCADE;
DROP TABLE IF EXISTS instructors CASCADE;
DROP TABLE IF EXISTS rewards CASCADE;
DROP TABLE IF EXISTS groups CASCADE;
DROP TABLE IF EXISTS players CASCADE;
DROP TABLE IF EXISTS exercises CASCADE;
DROP TABLE IF EXISTS modules CASCADE;
DROP TABLE IF EXISTS games CASCADE;
DROP TABLE IF EXISTS courses CASCADE;

CREATE TABLE courses (
    id BIGSERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    languages TEXT NOT NULL,
    programming_languages TEXT NOT NULL,
    gamification_rule_conditions TEXT NOT NULL,
    gamification_complex_rules TEXT NOT NULL,
    gamification_rule_results TEXT NOT NULL,
    public BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE games (
    id BIGSERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    public BOOLEAN NOT NULL DEFAULT FALSE,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    description TEXT NOT NULL,
    course_id BIGINT NOT NULL,
    programming_language VARCHAR(100) NOT NULL,
    module_lock DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    exercise_lock BOOLEAN NOT NULL DEFAULT FALSE,
    total_exercises INTEGER NOT NULL DEFAULT 0,
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_games_course FOREIGN KEY (course_id) REFERENCES courses (id) ON DELETE RESTRICT
);
CREATE TABLE modules (
    id BIGSERIAL PRIMARY KEY,
    course_id BIGINT NOT NULL,
    "order" INTEGER NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    language VARCHAR(10) NOT NULL,
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NOT NULL,
    CONSTRAINT fk_modules_course FOREIGN KEY (course_id) REFERENCES courses (id) ON DELETE CASCADE
);
CREATE TABLE exercises (
    id BIGSERIAL PRIMARY KEY,
    version NUMERIC NOT NULL DEFAULT 1.0,
    module_id BIGINT NOT NULL,
    "order" INTEGER NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    language VARCHAR(10) NOT NULL,
    programming_language VARCHAR(100) NOT NULL,
    init_code TEXT NOT NULL,
    pre_code TEXT NOT NULL,
    post_code TEXT NOT NULL,
    test_code TEXT NOT NULL,
    check_source TEXT NOT NULL,
    hidden BOOLEAN NOT NULL DEFAULT FALSE,
    locked BOOLEAN NOT NULL DEFAULT FALSE,
    mode VARCHAR(50) NOT NULL,
    mode_parameters JSONB NOT NULL,
    difficulty VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_exercises_module FOREIGN KEY (module_id) REFERENCES modules (id) ON DELETE CASCADE
);
CREATE TABLE players (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    display_avatar TEXT,
    points INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_active TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    disabled BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE TABLE groups (
    id BIGSERIAL PRIMARY KEY,
    display_name VARCHAR(100) NOT NULL,
    display_avatar TEXT
);
CREATE TABLE instructors (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    display_avatar TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_active TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE rewards (
    id BIGSERIAL PRIMARY KEY,
    course_id BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    message_when_won TEXT NOT NULL,
    image_url TEXT,
    valid_period INTERVAL,
    CONSTRAINT fk_rewards_course FOREIGN KEY (course_id) REFERENCES courses (id) ON DELETE CASCADE
);
CREATE TABLE invites (
    id BIGSERIAL PRIMARY KEY,
    uuid UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    instructor_id BIGINT NOT NULL,
    game_id BIGINT NULL,
    group_id BIGINT NULL,
    CONSTRAINT fk_invites_instructor FOREIGN KEY (instructor_id) REFERENCES instructors (id) ON DELETE CASCADE,
    CONSTRAINT fk_invites_game FOREIGN KEY (game_id) REFERENCES games (id) ON DELETE SET NULL,
    CONSTRAINT fk_invites_group FOREIGN KEY (group_id) REFERENCES groups (id) ON DELETE SET NULL
);
CREATE TABLE submissions (
    id BIGSERIAL PRIMARY KEY,
    exercise_id BIGINT NOT NULL,
    game_id BIGINT NOT NULL,
    player_id BIGINT NOT NULL,
    client VARCHAR(255) NOT NULL,
    submitted_code TEXT NOT NULL,
    metrics JSONB NOT NULL,
    result NUMERIC NOT NULL,
    result_description JSONB NOT NULL,
    first_solution BOOLEAN NOT NULL DEFAULT FALSE,
    feedback TEXT NOT NULL,
    earned_rewards JSONB NOT NULL,
    entered_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_submissions_exercise FOREIGN KEY (exercise_id) REFERENCES exercises (id) ON DELETE CASCADE,
    CONSTRAINT fk_submissions_game FOREIGN KEY (game_id) REFERENCES games (id) ON DELETE CASCADE,
    CONSTRAINT fk_submissions_player FOREIGN KEY (player_id) REFERENCES players (id) ON DELETE CASCADE
);
CREATE TABLE player_groups (
    player_id BIGINT NOT NULL,
    group_id BIGINT NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    left_at TIMESTAMPTZ NULL,
    PRIMARY KEY (player_id, group_id),
    CONSTRAINT fk_playergroups_player FOREIGN KEY (player_id) REFERENCES players (id) ON DELETE CASCADE,
    CONSTRAINT fk_playergroups_group FOREIGN KEY (group_id) REFERENCES groups (id) ON DELETE CASCADE
);
CREATE TABLE player_registrations (
    id BIGSERIAL PRIMARY KEY,
    player_id BIGINT NOT NULL,
    game_id BIGINT NOT NULL,
    language VARCHAR(10) NOT NULL,
    progress INTEGER NOT NULL DEFAULT 0,
    game_state JSONB NOT NULL,
    saved_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    left_at TIMESTAMPTZ NULL,
    CONSTRAINT fk_playerregistrations_player FOREIGN KEY (player_id) REFERENCES players (id) ON DELETE CASCADE,
    CONSTRAINT fk_playerregistrations_game FOREIGN KEY (game_id) REFERENCES games (id) ON DELETE CASCADE,
    UNIQUE (player_id, game_id)
);
CREATE TABLE player_unlocks (
    player_id BIGINT NOT NULL,
    exercise_id BIGINT NOT NULL,
    unlocked_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (player_id, exercise_id),
    CONSTRAINT fk_playerunlocks_player FOREIGN KEY (player_id) REFERENCES players (id) ON DELETE CASCADE,
    CONSTRAINT fk_playerunlocks_exercise FOREIGN KEY (exercise_id) REFERENCES exercises (id) ON DELETE CASCADE
);
CREATE TABLE player_rewards (
    id BIGSERIAL PRIMARY KEY,
    player_id BIGINT NOT NULL,
    reward_id BIGINT NOT NULL,
    game_id BIGINT NULL,
    count INTEGER NOT NULL DEFAULT 1,
    used_count INTEGER NOT NULL DEFAULT 0,
    obtained_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT fk_playerrewards_player FOREIGN KEY (player_id) REFERENCES players (id) ON DELETE CASCADE,
    CONSTRAINT fk_playerrewards_reward FOREIGN KEY (reward_id) REFERENCES rewards (id) ON DELETE CASCADE,
    CONSTRAINT fk_playerrewards_game FOREIGN KEY (game_id) REFERENCES games (id) ON DELETE SET NULL,
    CONSTRAINT uq_player_reward_game UNIQUE (player_id, reward_id, game_id, obtained_at)
);
CREATE TABLE game_ownership (
    game_id BIGINT NOT NULL,
    instructor_id BIGINT NOT NULL,
    owner BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (game_id, instructor_id),
    CONSTRAINT fk_gameownership_game FOREIGN KEY (game_id) REFERENCES games (id) ON DELETE CASCADE,
    CONSTRAINT fk_gameownership_instructor FOREIGN KEY (instructor_id) REFERENCES instructors (id) ON DELETE CASCADE
);
CREATE TABLE course_ownership (
    course_id BIGINT NOT NULL,
    instructor_id BIGINT NOT NULL,
    owner BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (course_id, instructor_id),
    CONSTRAINT fk_courseownership_course FOREIGN KEY (course_id) REFERENCES courses (id) ON DELETE CASCADE,
    CONSTRAINT fk_courseownership_instructor FOREIGN KEY (instructor_id) REFERENCES instructors (id) ON DELETE CASCADE
);
CREATE TABLE group_ownership (
    group_id BIGINT NOT NULL,
    instructor_id BIGINT NOT NULL,
    owner BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (group_id, instructor_id),
    CONSTRAINT fk_groupownership_group FOREIGN KEY (group_id) REFERENCES groups (id) ON DELETE CASCADE,
    CONSTRAINT fk_groupownership_instructor FOREIGN KEY (instructor_id) REFERENCES instructors (id) ON DELETE CASCADE
);

CREATE INDEX idx_games_course_id ON games (course_id);
CREATE INDEX idx_modules_course_id ON modules (course_id);
CREATE INDEX idx_exercises_module_id ON exercises (module_id);
CREATE INDEX idx_submissions_exercise_id ON submissions (exercise_id);
CREATE INDEX idx_submissions_game_id ON submissions (game_id);
CREATE INDEX idx_submissions_player_id ON submissions (player_id);
CREATE INDEX idx_player_groups_group_id ON player_groups (group_id);
CREATE INDEX idx_player_registrations_player_id ON player_registrations (player_id);
CREATE INDEX idx_player_registrations_game_id ON player_registrations (game_id);
CREATE INDEX idx_player_unlocks_exercise_id ON player_unlocks (exercise_id);
CREATE INDEX idx_player_rewards_reward_id ON player_rewards (reward_id);
CREATE INDEX idx_player_rewards_game_id ON player_rewards (game_id);
CREATE INDEX idx_game_ownership_instructor_id ON game_ownership (instructor_id);
CREATE INDEX idx_course_ownership_instructor_id ON course_ownership (instructor_id);
CREATE INDEX idx_group_ownership_instructor_id ON group_ownership (instructor_id);
CREATE INDEX idx_rewards_course_id ON rewards (course_id);
CREATE INDEX idx_invites_uuid ON invites (uuid);
CREATE INDEX idx_invites_instructor_id ON invites (instructor_id);
CREATE INDEX idx_invites_game_id ON invites (game_id);
CREATE INDEX idx_invites_group_id ON invites (group_id);
CREATE INDEX idx_exercises_mode_parameters_gin ON exercises USING GIN (mode_parameters);
CREATE INDEX idx_submissions_metrics_gin ON submissions USING GIN (metrics);
CREATE INDEX idx_submissions_result_description_gin ON submissions USING GIN (result_description);
CREATE INDEX idx_submissions_earned_rewards_gin ON submissions USING GIN (earned_rewards);
CREATE INDEX idx_player_registrations_game_state_gin ON player_registrations USING GIN (game_state);