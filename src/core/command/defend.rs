use crate::types::{GridSquare, SenderDetails};

#[derive(Debug)]
pub struct DefendRequest {
  pub game_id: i32,
  pub sender: SenderDetails,
  pub row_index: i32,
  pub column_index: i32,
}

#[derive(Debug)]
pub struct DefendResponse {
  pub square: GridSquare,
  pub requests_left: i32,
}
