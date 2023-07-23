use crate::types::{Game, GridSquare};

#[derive(Debug)]
pub struct QueryGameRequest {
  pub game_id: i32,
}

#[derive(Debug)]
pub struct QueryGameResponse {
  pub game: Game,
}

#[derive(Debug)]
pub struct QueryGridSquareRequest {
  pub game_id: i32,
  pub row_index: i32,
  pub column_index: i32,
}

#[derive(Debug)]
pub struct QueryGridSquareResponse {
  pub square: GridSquare,
}

#[derive(Debug)]
pub struct QueryGridResponse {}
