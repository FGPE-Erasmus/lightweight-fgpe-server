
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
- PostgreSQL (17+)

Building:

    1. open project directory in terminal
    2. execute build command `cargo build --release` for optimized version, omit `--release` otherwise
    3. enjoy executable from `/target` directory

Notes:

- Temporarily assumes "postgresql://postgres:admin@localhost:5432/gam2" DB connection
- Serves at localhost:3000

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
```json
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
```json
{
  "player_id": 1,
  "game_id": 1,
  "language": "optional string"
}
```
- Response:
```json
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
```json
{
  "player_registration_id": 1,
  "game_state": "string"
}
```
- Response:
```json
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
```json
{
  "player_registration_id": 1
}
```
- Response:
```json
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
```json
{
  "player_id": 1,
  "game_id": 1
}
```
- Response:
```json
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
```json
{
  "player_id": 1,
  "game_id": 1,
  "language": "string"
}
```
- Response:
```json
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
```json
{
  "player_id": 1,
  "active": true
}
```
- Response:
```json
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
```json
{
  "player_registrations_id": 1
}
```
- Response:
```json
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
```json
{
  "game_id": 1,
  "language": "string"
}
```
- Response:
```json
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
```json
{
  "module_id": 1,
  "language": "string",
  "programming_language": "string"
}
```
- Response:
```json
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

