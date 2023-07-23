use crate::types::{GameStatus, SenderDetails};

#[derive(Debug)]
pub struct StartRequest {
  pub game_id: i32,
  pub sender: SenderDetails,
}

#[derive(Debug)]
pub struct StartResponse {
  pub game_id: i32,
  pub status: GameStatus,
}
