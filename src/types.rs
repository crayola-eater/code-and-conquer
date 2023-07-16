use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::IntoStaticStr;

pub use crate::core::command::attack::{AttackRequest, AttackResponse};
pub use crate::core::command::create_and_join::{CreateAndJoinRequest, CreateAndJoinResponse};
pub use crate::core::command::defend::{DefendRequest, DefendResponse};
pub use crate::core::command::join_existing::{JoinExistingRequest, JoinExistingResponse};
pub use crate::core::command::place_mine::{PlaceMineRequest, PlaceMineResponse};
pub use crate::core::command::query::{
  QueryGameRequest, QueryGameResponse, QueryGridResponse, QueryGridSquareRequest, QueryGridSquareResponse,
};
pub use crate::core::command::start::{StartRequest, StartResponse};
pub use crate::core::command::{Command, CommandResponse};
pub use crate::core::error::{Error, Result};
pub use crate::core::games::Games;

#[derive(Debug, PartialEq, Copy, Clone, IntoStaticStr, Serialize, Deserialize)]
pub enum TeamRole {
  Minelayer,
  Cloaker,
  Spy,
}

#[derive(Debug, PartialEq, PartialOrd, Serialize, Deserialize, Clone, Copy, IntoStaticStr)]
pub enum GameStatus {
  WaitingForRegistrations,
  Started,
  Ended,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mine {
  pub placed_by: i32,
  pub triggered_by: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GridSquare {
  pub id: i32,
  pub game_id: i32,
  pub owner_id: Option<i32>,
  #[serde(rename = "row_index")]
  pub row: i32,
  #[serde(rename = "column_index")]
  pub column: i32,
  pub created_at: DateTime<Utc>,
  pub bonus: i32,
  pub health: i32,
  pub mine: Option<Mine>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Team {
  pub id: i32,
  pub display_name: String,
  pub key: String,
  pub role: TeamRole,
  pub role_used: bool,
  pub requests_left: i32,
  pub created_at: DateTime<Utc>,
  pub time_of_last_command: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct Game {
  pub id: i32,
  pub status: GameStatus,
  pub created_at: DateTime<Utc>,
  pub grid: Vec<GridSquare>,
  pub teams: Vec<Team>,
}

#[derive(Debug)]
pub struct SenderDetails {
  pub team_id: i32,
  pub team_key: String,
}
