
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
- PostgreSQL (17+) - sample local database dump is located in `/db` directory

Building:

    1. open project directory in terminal
    2. execute build command `cargo build --release` for optimized version, omit `--release` otherwise
    3. enjoy executable from `/target` directory

Usage:

```
lightweight-fgpe-server.exe [OPTIONS] --connection-str <CONNECTION_STR>

Options:
-c, --connection-str <CONNECTION_STR>
-s, --server-url <SERVER_URL>          [default: 127.0.0.1:3000]
-h, --help                             Print help
```

## API
### Response format

```
{
    "status_text": string,
    "status_code": int,
    "data": obj
}
```

### Available Endpoints

#### Get Available Games
- URL: `/get_available_games`
- Method: `GET`
- Payload: `None`
- Response:
```
{
  "status_text": "OK",
  "status_code": 200,
  "data": {
    "games": [
      {
        "id": 1,
        "title": "string",
        "public": true,
        "active": true,
        "description": "string",
        "course": 1,
        "programminglanguage": "string",
        "modulelock": 0.0,
        "exerciselock": false,
        "totalexercises": 0,
        "startdate": "YYYY-MM-DD",
        "enddate": "YYYY-MM-DD",
        "createdat": "YYYY-MM-DD",
        "updatedat": "YYYY-MM-DD"
      }
    ]
  }
}
```

#### Join Game
- URL: `/join_game`
- Method: `POST`
- Payload:
```
{
  "player_id": 1,
  "game_id": 1,
  "language": "optional string"
}
```
- Response:
```
{
  "status_text": "OK",
  "status_code": 200,
  "data": {
    "player_registration_id": 1
  }
}
```

#### Save Game
- URL: `/save_game`
- Method: `POST`
- Payload:
```
{
  "player_registration_id": 1,
  "game_state": "string"
}
```
- Response:
```
{
  "status_text": "OK",
  "status_code": 200,
  "data": {
    "saved": true
  }
}
```

#### Load Game
- URL: `/load_game`
- Method: `POST`
- Payload:
```
{
  "player_registration_id": 1
}
```
- Response:
```
{
  "status_text": "OK",
  "status_code": 200,
  "data": {
    "game_state": "string"
  }
}
```

#### Leave Game
- URL: `/leave_game`
- Method: `POST`
- Payload:
```
{
  "player_id": 1,
  "game_id": 1
}
```
- Response:
```
{
  "status_text": "OK",
  "status_code": 200,
  "data": {
    "left": true
  }
}
```

#### Set Game Language
- URL: `/set_game_lang`
- Method: `POST`
- Payload:
```
{
  "player_id": 1,
  "game_id": 1,
  "language": "string"
}
```
- Response:
```
{
  "status_text": "OK",
  "status_code": 200,
  "data": {
    "set": true
  }
}
```

#### Get Player Games
- URL: `/get_player_games`
- Method: `POST`
- Payload:
```
{
  "player_id": 1,
  "active": true
}
```
- Response:
```
{
  "status_text": "OK",
  "status_code": 200,
  "data": {
    "games": [1, 2, 3]
  }
}
```

#### Get Game Metadata
- URL: `/get_game_metadata`
- Method: `POST`
- Payload:
```
{
  "player_registrations_id": 1
}
```
- Response:
```
{
  "status_text": "OK",
  "status_code": 200,
  "data": {
    "data": {
      "player_registration": {
        // Player registration details
      },
      "game": {
        // Game details
      }
    }
  }
}
```

#### Get Course Data
- URL: `/get_course_data`
- Method: `POST`
- Payload:
```
{
  "game_id": 1,
  "language": "string"
}
```
- Response:
```
{
  "status_text": "OK",
  "status_code": 200,
  "data": {
    "course_gamification_rule_conditions": "string",
    "gamification_complex_rules": "string",
    "gamification_rule_results": "string",
    "modules": [1, 2, 3]
  }
}
```

#### Get Module Data
- URL: `/get_module_data`
- Method: `POST`
- Payload:
```
{
  "module_id": 1,
  "language": "string",
  "programming_language": "string"
}
```
- Response:
```
{
  "status_text": "OK",
  "status_code": 200,
  "data": {
    "module_order": 1,
    "module_title": "string",
    "module_description": "string",
    "module_start_date": "YYYY-MM-DD",
    "module_end_date": "YYYY-MM-DD",
    "exercises": [1, 2, 3]
  }
}
```

#### Get Exercise Data
- URL: `/get_exercise_data`
- Method: `POST`
- Payload:
```
{
    "exercise_id": 1,
    "player_id": 1,
    "game_id": 1
}
```
- Response:
```
{
    "status_text": "OK",
    "status_code": 200,
    "data": {
        "exercise": {
            "id": 1,
            "version": 1,
            "module": 1,
            "order": 1,
            "title": "Exercise 1",
            "description": "Description of Exercise 1",
            "language": "English",
            "programminglanguage": "Python",
            "initcode": "print(\"Hello\")",
            "precode": "",
            "postcode": "",
            "testcode": "",
            "checksource": "",
            "hidden": false,
            "locked": false,
            "mode": "Standard",
            "modeparameters": "{}",
            "difficulty": "Easy",
            "createdat": "2024-12-06",
            "updatedat": "2024-12-06"
        }
    }
}
```

#### Submit Solution
- URL: `/submit_solution`
- Method: `POST`
- Payload:
```
{
    "exercise_id": 1,
    "player_id": 1,
    "submission_client": "submission_client1",
    "submission_submitted_code": "submission_submitted_code1",
    "submission_metrics": "submission_metrics1",
    "submission_result": 100.0,
    "submission_result_description": "perfect",
    "submission_feedback": "OK",
    "submission_entered_at": "2024-12-06",
    "submission_earned_rewards": "badge1,badge2"
}
```
- Response:
```
{
    "status_text": "OK",
    "status_code": 200,
    "data": {
        "first_submission": false
    }
}
```

#### Unlock
- URL: `/unlock`
- Method: `POST`
- Payload:
```
{
    "exercise_id": 1,
    "player_id": 1
}
```
- Response:
```
{
    "status_text": "OK",
    "status_code": 200,
    "data": null
}
```

#### Get Last Solution
- URL: `/get_last_solution`
- Method: `POST`
- Payload:
```
{
    "exercise_id": 1,
    "player_id": 1
}
```
- Response:
```
{
    "status_text": "OK",
    "status_code": 200,
    "data": {
        "submission": {
            "id": 9,
            "exercise": 1,
            "player": 1,
            "client": "submission_client1",
            "submittedcode": "submission_submitted_code1",
            "metrics": "submission_metrics1",
            "result": 100.0,
            "resultdescription": "perfect",
            "feedback": "OK",
            "earnedrewards": "badge1,badge2",
            "enteredat": "2024-12-06",
            "submittedat": "2025-03-02"
        }
    }
}
```