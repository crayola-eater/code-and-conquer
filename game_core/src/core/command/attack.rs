use crate::types::{GridSquare, SenderDetails};

#[derive(Debug)]
pub struct AttackRequest {
  pub game_id: i32,
  pub sender: SenderDetails,
  pub row_index: i32,
  pub column_index: i32,
}

#[derive(Debug)]
pub struct AttackResponse {
  pub square: GridSquare,
  pub conquered: bool,
  pub requests_left: i32,
}
