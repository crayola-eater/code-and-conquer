use crate::types::{GridSquare, SenderDetails};

#[derive(Debug)]
pub struct PlaceMineRequest {
  pub game_id: u32,
  pub sender: SenderDetails,
  pub row_index: usize,
  pub column_index: usize,
}

#[derive(Debug)]
pub struct PlaceMineResponse {
  pub square: GridSquare,
  pub triggered_mine: bool,
}
