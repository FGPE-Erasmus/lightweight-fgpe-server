use chrono::NaiveDate;
use diesel::prelude::*;
use crate::schema::*;

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Course {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub languages: String,
    pub programminglanguages: String,
    pub gamificationruleconditions: String,
    pub gamificationcomplexrules: String,
    pub gamificationruleresults: String,
    pub createdat: NaiveDate,
    pub updatedat: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Game {
    pub id: i32,
    pub title: String,
    pub public: bool,
    pub active: bool,
    pub description: String,
    pub course: i32,
    pub programminglanguage: String,
    pub modulelock: f32,
    pub exerciselock: bool,
    pub totalexercises: i32,
    pub startdate: NaiveDate,
    pub enddate: NaiveDate,
    pub createdat: NaiveDate,
    pub updatedat: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Module {
    pub id: i32,
    pub course: i32,
    pub order: i32,
    pub title: String,
    pub description: String,
    pub language: String,
    pub startdate: NaiveDate,
    pub enddate: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Exercise {
    pub id: i32,
    pub version: i32,
    pub module: i32,
    pub order: i32,
    pub title: String,
    pub description: String,
    pub language: String,
    pub programminglanguage: String,
    pub initcode: String,
    pub precode: String,
    pub postcode: String,
    pub testcode: String,
    pub checksource: String,
    pub hidden: bool,
    pub locked: bool,
    pub mode: String,
    pub modeparameters: String,
    pub difficulty: String,
    pub createdat: NaiveDate,
    pub updatedat: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Insertable)]
#[diesel(table_name = submissions)]
pub struct NewSubmission {
    pub exercise: i32,
    pub player: i32,
    pub client: String,
    pub submittedcode: String,
    pub metrics: String,
    pub result: f64,
    pub resultdescription: String,
    pub feedback: String,
    pub earnedrewards: String,
    pub enteredat: NaiveDate,
    pub submittedat: NaiveDate,
}

impl NewSubmission {
    pub fn new(exercise: i32, player: i32, client: String, submittedcode: String, metrics: String,
               result: f64, resultdescription: String, feedback: String, earnedrewards: String,
               enteredat: NaiveDate, submittedat: NaiveDate) -> Self {
        Self { exercise, player, client, submittedcode, metrics, result, resultdescription,
            feedback, earnedrewards, enteredat, submittedat }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Submission {
    pub id: i32,
    pub exercise: i32,
    pub player: i32,
    pub client: String,
    pub submittedcode: String,
    pub metrics: String,
    pub result: f64,
    pub resultdescription: String,
    pub feedback: String,
    pub earnedrewards: String,
    pub enteredat: NaiveDate,
    pub submittedat: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Player {
    pub id: i32,
    pub email: String,
    pub displayname: String,
    pub displayavatar: String,
    pub points: i32,
    pub createdat: NaiveDate,
    pub lastactive: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Group {
    pub id: i32,
    pub displayname: String,
    pub displayavatar: String,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
#[diesel(table_name = playergroups)]
pub struct PlayerGroup {
    pub player: i32,
    pub group: i32,
    pub joinedat: NaiveDate,
    pub leftat: Option<NaiveDate>,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable, Insertable)]
#[diesel(table_name = playerunlocks)]
pub struct PlayerUnlock {
    pub player: i32,
    pub exercise: i32,
    pub unlockedat: NaiveDate,
}

impl PlayerUnlock {
    pub fn new(player: i32, exercise: i32, unlockedat: NaiveDate) -> Self {
        Self { player, exercise, unlockedat }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
pub struct Reward {
    pub id: i32,
    pub course: i32,
    pub name: String,
    pub description: String,
    pub messagewhenwon: String,
    pub imageurl: String,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
#[diesel(table_name = playerrewards)]
pub struct PlayerReward {
    pub player: i32,
    pub reward: i32,
    pub game: Option<i32>,
    pub count: i32,
    pub usedcount: i32,
    pub obtainedat: NaiveDate,
    pub expiresat: NaiveDate,
}

#[derive(serde::Serialize, serde::Deserialize, Selectable, Queryable)]
#[diesel(table_name = playerregistrations)]
pub struct PlayerRegistration {
    pub id: i32,
    pub player: i32,
    pub game: i32,
    pub language: String,
    pub progress: i32,
    pub gamestate: String,
    pub savedat: NaiveDate,
    pub joinedat: NaiveDate,
    pub leftat: Option<NaiveDate>,
}

#[derive(serde::Serialize, serde::Deserialize, Insertable)]
#[diesel(table_name = playerregistrations)]
pub struct NewPlayerRegistration {
    pub player: i32,
    pub game: i32,
    pub language: String,
    pub progress: i32,
    pub gamestate: String,
    pub savedat: NaiveDate,
    pub joinedat: NaiveDate,
    pub leftat: Option<NaiveDate>,
}

impl NewPlayerRegistration {
    pub fn new(player: i32, game: i32, language: String, progress: i32, gamestate: String,
               savedat: NaiveDate, joinedat: NaiveDate, leftat: Option<NaiveDate>) -> Self {
        Self { player, game, language, progress, gamestate, savedat, joinedat, leftat }
    }
}

allow_tables_to_appear_in_same_query!(playerregistrations, games);
allow_tables_to_appear_in_same_query!(modules, exercises);
