# Lightweight FGPE Server

Simple backend providing connection between database and clients.

## Features

- rock solid and fast (thanks to Rust)
- compile-time-safe SQL queries
- using pool of database connections, enhancing performance by reusing connections across multiple requests
- asynchronous and capable of handling high concurrency

## Installation & usage

Prerequisites:
- [Rust](https://www.rust-lang.org/learn/get-started)
- PostgreSQL (17+) - table creation script is located in `/db` directory

Building:

    1. open project directory in terminal
    2. execute build command `cargo build --release` for optimized version, omit `--release` otherwise
    3. enjoy executable from `/target` directory

Usage:
```
lightweight-fgpe-server.exe [OPTIONS] --connection-str <CONNECTION_STR>

Options:
--connection-str <CONNECTION_STR>
Database connection string (e.g., "postgres://user:password@host:port/database") Can also be set using the DATABASE_URL environment variable [env: DATABASE_URL=]
--db-pool-max-size <DB_POOL_MAX_SIZE>
Database connection pool size Can also be set using the DB_POOL_MAX_SIZE environment variable. Default value: 10 [env: DB_POOL_MAX_SIZE=] [default: 10]
--server-address <SERVER_ADDRESS>
Server listen address and port (e.g., "127.0.0.1:3000") Can also be set using the SERVER_ADDRESS environment variable. Default value: 127.0.0.1:3000 [env: SERVER_ADDRESS=] [default: 127.0.0.1:3000]
--keycloak-server-url <KEYCLOAK_SERVER_URL>
Keycloak server address and port (e.g., "127.0.0.1:8080") Can also be set using the KEYCLOAK_SERVER_URL environment variable. Default value: 127.0.0.1:8080 [env: KEYCLOAK_SERVER_URL=] [default: http://127.0.0.1:8080]
--keycloak-realm <KEYCLOAK_REALM>
Keycloak realm name Can also be set using the KEYCLOAK_REALM environment variable. Default value: fgpe [env: KEYCLOAK_REALM=] [default: fgpe]
--keycloak-audiences <KEYCLOAK_AUDIENCES>
Keycloak allowed audiences (e.g., "account") Can also be set using the KEYCLOAK_AUDIENCES environment variable. Default value: account [env: KEYCLOAK_AUDIENCES=] [default: account]
--log-level <LOG_LEVEL>
Log level (e.g., "info") Can also be set using the RUST_LOG environment variable. Default value: info [env: RUST_LOG=] [default: info]
-h, --help
Print help
-V, --version
Print version
```

Testing:

    1. open project directory in terminal
    2. execute command `cargo test -- --test-threads=1`

## API Specification

### Authentication

All endpoints require authentication via a Keycloak-issued JWT Bearer token provided in the `Authorization` header. The token must be valid, unexpired, and contain the audience specified in the server configuration (`--keycloak-audiences` / `KEYCLOAK_AUDIENCES`).

Instructor that with `id = 0` is treated as an admin.

### Common Response Format

All API responses follow a standard JSON structure:

**Success:**

```json
{
  "status_code": 200, // Or other 2xx/3xx code
  "status_message": "OK", // Or other relevant message
  "data": { ... } // The actual response data (or null), type varies by endpoint
}
```

**Error:**

```json
{
"status_code": 4xx or 5xx, // e.g., 404, 403, 500
"status_message": "Error description", // e.g., "Not Found: Resource not found", "Forbidden: Insufficient permissions"
"data": null
}
```

### Common Error Status Codes:

- **400 - Bad Request**: Invalid request format or parameters.
- **401 - Unauthorized**: Missing or invalid authentication token.
- **403 - Forbidden**: Authenticated user lacks permission for the action/resource.
- **404 - Not Found**: The requested resource (game, player, course, etc.) does not exist.
- **409 - Conflict**: The request conflicts with the current state (e.g., unique constraint violation).
- **422 - Unprocessable Entity**: The request was well-formed but semantically incorrect (e.g., invalid language choice).
- **500 - Internal Server Error**: An unexpected error occurred on the server.

### Notes

- submissions are considered correct when `result > 50`

### Student Endpoints (/student)

*All endpoints require authentication.*

*   **`GET /get_available_games`**
    *   Description: Retrieves a list of public and active game IDs.
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        [101, 105, 210]
        ```
*   **`POST /join_game`**
    *   Description: Registers the authenticated player into a specific game.
    *   Request Body:
        ```json
        {
          "player_id": 123,
          "game_id": 456,
          "language": "en"
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        789
        ```
    *   Errors: 404 (Player/Game not found), 409 (Already registered)
*   **`POST /save_game`**
    *   Description: Saves the player's current game state for a specific registration.
    *   Request Body:
        ```json
        {
          "player_registrations_id": 789,
          "game_state": <Json>
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 404 (Registration not found)
*   **`POST /load_game`**
    *   Description: Loads the player's previously saved game state for a specific registration.
    *   Request Body:
        ```json
        {
          "player_registrations_id": 789
        }
        ```
    *   Success Response Body (`data` field):
        ```json
          <Json>
        ```
    *   Errors: 404 (Registration not found)
*   **`POST /leave_game`**
    *   Description: Marks the player's registration in a game as inactive.
    *   Request Body:
        ```json
        {
          "player_id": 123,
          "game_id": 456
        }
        ```
    *   Success Response Body (`data` field): `null`
    *   Errors: 404 (Active registration not found)
*   **`POST /set_game_lang`**
    *   Description: Sets the preferred language for the player within a specific game registration, if allowed by the course.
    *   Request Body:
        ```json
        {
          "player_id": 123,
          "game_id": 456,
          "language": "fr"
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 404 (Registration not found), 422 (Language not allowed)
*   **`GET /get_player_games`**
    *   Description: Retrieves the player registration IDs for the authenticated player.
    *   Query Params: `player_id` (i64, required), `active` (bool, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        [789, 801, 805]
        ```
    *   Errors: 404 (Player not found)
*   **`GET /get_game_metadata/{registration_id}`**
    *   Description: Retrieves detailed metadata about a specific game registration and the associated game.
    *   Path Params: `registration_id` (i64)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        {
          "registration_id": 789,
          "progress": 5,
          "joined_at": "2024-07-27T10:00:00Z",
          "left_at": null,
          "language": "en",
          "game_id": 456,
          "game_title": "Adventure Quest",
          "game_active": true,
          "game_description": "Explore the world!",
          "game_programming_language": "py",
          "game_total_exercises": 10,
          "game_start_date": "2024-07-01T00:00:00Z",
          "game_end_date": "2024-12-31T23:59:59Z"
        }
        ```
    *   Errors: 404 (Registration not found)
*   **`GET /get_course_data`**
    *   Description: Retrieves course-level data (gamification rules, module IDs) relevant to a specific game and language.
    *   Query Params: `game_id` (i64, required), `language` (string, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        {
          "gamification_rule_conditions": "<some rules>",
          "gamification_complex_rules": "<some rules>",
          "gamification_rule_results": "<some rewards>",
          "module_ids": [11, 12, 15]
        }
        ```
    *   Errors: 404 (Game or associated course not found)
*   **`GET /get_module_data`**
    *   Description: Retrieves module details and relevant exercise IDs based on language filters.
    *   Query Params: `module_id` (i64, required), `language` (string, required), `programming_language` (string, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        {
          "order": 1,
          "title": "Introduction",
          "description": "Getting started basics.",
          "start_date": "2024-07-01T00:00:00Z",
          "end_date": "2024-07-15T23:59:59Z",
          "exercise_ids": [101, 102, 103]
        }
        ```
    *   Errors: 404 (Module not found)
*   **`GET /get_exercise_data`**
    *   Description: Retrieves detailed data for a specific exercise, calculating context-dependent hidden/locked status based on game rules and player progress/unlocks.
    *   Query Params: `exercise_id` (i64, required), `game_id` (i64, required), `player_id` (i64, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        {
          "order": 1,
          "title": "Hello World",
          "description": "Print 'Hello, World!'",
          "init_code": "",
          "pre_code": "",
          "post_code": "",
          "test_code": "assert output == 'Hello, World!'",
          "check_source": "",
          "mode": "code",
          "mode_parameters": {},
          "difficulty": "easy",
          "hidden": false,
          "locked": false
        }
        ```
    *   Errors: 404 (Exercise or Game not found)
*   **`POST /submit_solution`**
    *   Description: Submits a solution attempt for an exercise, updates progress, and potentially grants rewards.
    *   Request Body:
        ```json
        {
          "player_id": 123,
          "exercise_id": 101,
          "game_id": 456,
          "client": "web-ide",
          "submitted_code": "print('Hello, World!')",
          "metrics": <Json>,
          "result": 100.0,
          "result_description": <Json>,
          "feedback": "",
          "entered_at": "2024-07-27T11:00:00Z",
          "earned_rewards": [51, 52]
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
        *(`true` if first correct submission, `false` otherwise)*
    *   Errors: 404 (Registration, Game, Exercise, or Reward ID not found)
*   **`POST /unlock`**
    *   Description: Explicitly unlocks (makes visible/accessible) a specific exercise for the player.
    *   Request Body:
        ```json
        {
          "player_id": 123,
          "exercise_id": 102
        }
        ```
    *   Success Response Body (`data` field): `null`
    *   Errors: 404 (Player or Exercise not found)
*   **`GET /get_last_solution`**
    *   Description: Retrieves the most recent relevant submission for an exercise (prioritizes last correct, falls back to last overall).
    *   Query Params: `player_id` (i64, required), `exercise_id` (i64, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        // If a solution exists
        {
          "submitted_code": "print('Hello, World!')",
          "metrics": <Json>,
          "result": 100.0,
          "result_description": <Json>,
          "feedback": "",
          "submitted_at": "2024-07-27T11:05:00Z"
        }
        // If no solution exists
        null
        ```
    *   Errors: 404 (Player or Exercise not found)

---

### Teacher Endpoints (`/teacher`)

*All endpoints require authentication.*

*   **`GET /get_instructor_games`**
    *   Description: Retrieves game IDs associated with the authenticated instructor.
    *   Query Params: `instructor_id` (i64, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        [456, 457, 459]
        ```
    *   Errors: 404 (Instructor not found)
*   **`GET /get_instructor_game_metadata`**
    *   Description: Retrieves detailed metadata for a specific game if the instructor has access.
    *   Query Params: `instructor_id` (i64, required), `game_id` (i64, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        {
          "title": "Adventure Quest",
          "description": "Explore the world!",
          "active": true,
          "public": false,
          "total_exercises": 10,
          "start_date": "2024-07-01T00:00:00Z",
          "end_date": "2024-12-31T23:59:59Z",
          "is_owner": true,
          "player_count": 25
        }
        ```
    *   Errors: 403 (Permission denied), 404 (Game not found)
*   **`GET /list_students`**
    *   Description: Lists student IDs participating in a specific game, with optional filters.
    *   Query Params: `instructor_id` (i64, required), `game_id` (i64, required), `group_id` (i64, optional), `only_active` (bool, optional, default=false)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        [123, 124, 125, 127]
        ```
    *   Errors: 403 (Permission denied), 404 (Game or filter group not found)
*   **`GET /get_student_progress`**
    *   Description: Retrieves progress metrics (attempts, solved, percentage) for a specific student in a game.
    *   Query Params: `instructor_id` (i64, required), `game_id` (i64, required), `player_id` (i64, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        {
          "attempts": 15,
          "solved_exercises": 8,
          "progress": 80.0
        }
        ```
    *   Errors: 403 (Permission denied), 404 (Game/Player not found, or player not registered)
*   **`GET /get_student_exercises`**
    *   Description: Retrieves lists of attempted and solved exercise IDs for a student in a game.
    *   Query Params: `instructor_id` (i64, required), `game_id` (i64, required), `player_id` (i64, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        {
          "attempted_exercises": [101, 102, 103, 104, 105],
          "solved_exercises": [101, 102, 104]
        }
        ```
    *   Errors: 403 (Permission denied), 404 (Game/Player not found, or player not registered)
*   **`GET /get_student_submissions`**
    *   Description: Retrieves submission IDs for a student in a game, optionally filtering for success.
    *   Query Params: `instructor_id` (i64, required), `game_id` (i64, required), `player_id` (i64, required), `success_only` (bool, optional, default=false)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        [5001, 5005, 5008, 5010]
        ```
    *   Errors: 403 (Permission denied), 404 (Game/Player not found, or player not registered)
*   **`GET /get_submission_data`**
    *   Description: Retrieves the full data for a specific submission.
    *   Query Params: `instructor_id` (i64, required), `submission_id` (i64, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        {
          "id": 5005,
          "exercise_id": 102,
          "game_id": 456,
          "player_id": 123,
          "client": "web-ide",
          "submitted_code": "def func():\n  pass",
          "metrics": <Json>,
          "result": 100.0,
          "result_description": <Json>,
          "first_solution": true,
          "feedback": "",
          "earned_rewards": [],
          "entered_at": "2024-07-27T12:00:00Z",
          "submitted_at": "2024-07-27T12:00:05Z"
        }
        ```
    *   Errors: 403 (Permission denied for associated game), 404 (Submission or associated game not found)
*   **`GET /get_exercise_stats`**
    *   Description: Retrieves statistics (attempts, success rate, difficulty) for an exercise within a game.
    *   Query Params: `instructor_id` (i64, required), `game_id` (i64, required), `exercise_id` (i64, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        {
          "attempts": 50,
          "successful_attempts": 35,
          "difficulty": 30.0,
          "solved_percentage": 70.0
        }
        ```
    *   Errors: 403 (Permission denied), 404 (Game or Exercise not found)
*   **`GET /get_exercise_submissions`**
    *   Description: Retrieves submission IDs for a specific exercise within a game, optionally filtering for success.
    *   Query Params: `instructor_id` (i64, required), `game_id` (i64, required), `exercise_id` (i64, required), `success_only` (bool, optional, default=false)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        [5001, 5003, 5005, 5006, 5008]
        ```
    *   Errors: 403 (Permission denied), 404 (Game or Exercise not found)
*   **`POST /create_game`**
    *   Description: Creates a new game based on a course and assigns ownership to the requesting instructor.
    *   Request Body:
        ```json
        {
          "instructor_id": 201,
          "title": "New Python Course Game",
          "public": false, // optional, default false
          "active": true, // optional, default false
          "description": "A game for the Python course.", // optional, default ""
          "course_id": 33,
          "programming_language": "py",
          "module_lock": 0.5, // optional, default 0.0
          "exercise_lock": true // optional, default false
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        460
        ```
    *   Errors: 404 (Instructor or Course not found), 422 (Programming language not allowed for course)
*   **`POST /modify_game`**
    *   Description: Modifies settings of an existing game. Only include fields to be changed.
    *   Request Body:
        ```json
        {
          "instructor_id": 201,
          "game_id": 460,
          "title": "Updated Python Game Title", // optional
          "active": false, // optional
          "module_lock": 0.8 // optional
          // other fields like public, description, exercise_lock are also optional
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 403 (Permission denied), 404 (Game not found)
*   **`POST /add_game_instructor`**
    *   Description: Adds another instructor to a game, potentially granting ownership. Requires owner permission.
    *   Request Body:
        ```json
        {
          "requesting_instructor_id": 201, // Must be owner or admin
          "game_id": 460,
          "instructor_to_add_id": 205,
          "is_owner": false // optional, default false
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 403 (Permission denied), 404 (Game or instructor_to_add not found)
*   **`POST /remove_game_instructor`**
    *   Description: Removes an instructor's association from a game. Requires owner permission.
    *   Request Body:
        ```json
        {
          "requesting_instructor_id": 201, // Must be owner or admin
          "game_id": 460,
          "instructor_to_remove_id": 205
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 403 (Permission denied), 404 (Game not found, or instructor not associated)
*   **`POST /activate_game`**
    *   Description: Sets a game's status to active.
    *   Request Body:
        ```json
        {
          "instructor_id": 201,
          "game_id": 460
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 403 (Permission denied), 404 (Game not found)
*   **`POST /stop_game`**
    *   Description: Sets a game's status to inactive.
    *   Request Body:
        ```json
        {
          "instructor_id": 201,
          "game_id": 460
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 403 (Permission denied), 404 (Game not found)
*   **`POST /remove_game_student`**
    *   Description: Removes a student's registration from a game.
    *   Request Body:
        ```json
        {
          "instructor_id": 201,
          "game_id": 460,
          "student_id": 123
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 403 (Permission denied), 404 (Game not found, or student not registered)
*   **`GET /translate_email_to_player_id`**
    *   Description: Finds the player ID associated with a given email address.
    *   Query Params: `email` (string, required)
    *   Request Body: None
    *   Success Response Body (`data` field):
        ```json
        123
        ```
    *   Errors: 404 (Email not found)
*   **`POST /create_group`**
    *   Description: Creates a new group, assigns ownership, and optionally adds initial members.
    *   Request Body:
        ```json
        {
          "instructor_id": 201,
          "display_name": "Study Group Alpha",
          "display_avatar": null, // optional URL string
          "member_list": [123, 124] // optional, default empty
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        55
        ```
    *   Errors: 404 (Instructor or member player not found), 409 (Group name conflict)
*   **`POST /dissolve_group`**
    *   Description: Deletes a group and removes all members and ownership. Requires owner permission.
    *   Request Body:
        ```json
        {
          "instructor_id": 201, // Must be owner or admin
          "group_id": 55
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 403 (Permission denied), 404 (Group not found)
*   **`POST /add_group_member`**
    *   Description: Adds a student (player) to a group. Requires owner permission.
    *   Request Body:
        ```json
        {
          "instructor_id": 201, // Must be owner or admin
          "group_id": 55,
          "player_id": 125
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 403 (Permission denied), 404 (Group or Player not found)
*   **`POST /remove_group_member`**
    *   Description: Removes a student (player) from a group. Requires owner permission.
    *   Request Body:
        ```json
        {
          "instructor_id": 201, // Must be owner or admin
          "group_id": 55,
          "player_id": 124
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 403 (Permission denied), 404 (Group not found, or player not a member)
*   **`POST /create_player`**
    *   Description: Creates a new player account, optionally adding them to a game and/or group. Requires admin or relevant game/group permission.
    *   Request Body:
        ```json
        {
          "instructor_id": 201, // Admin (0) or instructor with permission for game/group
          "email": "new.student@example.com",
          "display_name": "New Student",
          "display_avatar": null, // optional URL string
          "game_id": 460, // optional
          "group_id": 55, // optional
          "language": "en" // optional, defaults to "en" if game_id is present
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        130
        ```
    *   Errors: 403 (Permission denied), 404 (Game or Group not found), 409 (Email conflict)
*   **`POST /disable_player`**
    *   Description: Disables a player account. Requires admin permission.
    *   Request Body:
        ```json
        {
          "instructor_id": 0, // Must be admin
          "player_id": 130
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 403 (Permission denied), 404 (Player not found)
*   **`POST /delete_player`**
    *   Description: Permanently deletes a player account and all associated data. Requires admin permission.
    *   Request Body:
        ```json
        {
          "instructor_id": 0, // Must be admin
          "player_id": 130
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 403 (Permission denied), 404 (Player not found)
*   **`POST /generate_invite_link`**
    *   Description: Generates a unique invite link (UUID), optionally associated with a game and/or group. Requires admin or group permission.
    *   Request Body:
        ```json
        {
          "instructor_id": 201, // Admin (0) or instructor with permission for group_id
          "game_id": 460, // optional
          "group_id": 55 // optional
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        {
          "invite_uuid": "f47ac10b-58cc-4372-a567-0e02b2c3d479" // Example UUID
        }
        ```
    *   Errors: 403/404 (Permission denied or Instructor/Game/Group not found)
*   **`POST /process_invite_link`**
    *   Description: Processes an invite link for a player, adding them to the associated game/group if applicable.
    *   Request Body:
        ```json
        {
          "player_id": 135,
          "uuid": "f47ac10b-58cc-4372-a567-0e02b2c3d479" // The UUID from generate_invite_link
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 404 (Invite, Player, Game, or Group not found)

---

### Editor Endpoints (`/editor`)

*All endpoints require authentication.*

*   **`POST /import_course`**
    *   Description: Imports a complete course structure (modules, exercises) from JSON data and assigns ownership to the specified instructor.
    *   Request Body: (See `ImportCoursePayload` structure - complex, example below)
        ```json
        {
          "instructor_id": 301,
          "public": false, // optional, default false
          "course_data": {
            "title": "Imported Advanced Course",
            "description": "Details about the imported course.", // optional
            "languages": "en", // optional
            "programming_languages": "rust", // optional
            "gamification_rule_conditions": "{}", // optional
            "gamification_complex_rules": "{}", // optional
            "gamification_rule_results": "{}", // optional
            "modules": [ // optional
              {
                "order": 1,
                "title": "Module A",
                "description": "First module.", // optional
                "language": "en",
                "start_date": null, // optional ISO 8601 string or null
                "end_date": null, // optional ISO 8601 string or null
                "exercises": [ // optional
                  {
                    "version": 1.0,
                    "order": 1,
                    "title": "Exercise A.1",
                    "description": "First exercise.", // optional
                    "language": "en",
                    "programming_language": "rust",
                    "init_code": "", // optional
                    "pre_code": "", // optional
                    "post_code": "", // optional
                    "test_code": "", // optional
                    "check_source": "", // optional
                    "hidden": false, // optional, default false
                    "locked": false, // optional, default false
                    "mode": "code",
                    "mode_parameters": {}, // optional, default {}
                    "difficulty": "medium"
                  }
                ]
              }
            ]
          }
        }
        ```
    *   Success Response Body (`data` field):
        ```json
        true
        ```
    *   Errors: 404 (Instructor specified in payload not found)
*   **`GET /export_course`**
    *   Description: Exports the full structure of a course (details, modules, exercises) as JSON. Requires course ownership or admin permission.
    *   Query Params: `instructor_id` (i64, required), `course_id` (i64, required)
    *   Request Body: None
    *   Success Response Body (`data` field): (See `ExportCourseResponse` structure - complex, example below)
        ```json
        {
          "title": "Exported Course Title",
          "description": "Description of the course.",
          "languages": "en,fr",
          "programming_languages": "py,rust",
          "gamification_rule_conditions": "",
          "gamification_complex_rules": "",
          "gamification_rule_results": "",
          "modules": [
            {
              "order": 1,
              "title": "Exported Module 1",
              "description": "First module.",
              "language": "en",
              "start_date": "2024-01-01T00:00:00Z",
              "end_date": "2024-06-30T23:59:59Z",
              "exercises": [
                {
                  "order": 1,
                  "title": "Exported Exercise 1.1",
                  "description": "First exercise.",
                  "language": "en",
                  "programming_language": "py",
                  "init_code": "...",
                  "pre_code": "...",
                  "post_code": "...",
                  "test_code": "...",
                  "check_source": "...",
                  "hidden": false,
                  "locked": false,
                  "mode": "code",
                  "mode_parameters": {},
                  "difficulty": "easy"
                }
                // ... more exercises
              ]
            }
            // ... more modules
          ]
        }
        ```
    *   Errors: 403 (Permission denied), 404 (Course not found)

---
```