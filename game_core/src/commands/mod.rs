mod attack;
mod create_and_join;
mod defend;
mod join_existing;
mod place_mine;
mod query;
mod start;

pub use attack::{try_attack_a_square, AttackRequest, AttackResponse};
pub use create_and_join::{try_create_and_join_a_game, CreateAndJoinRequest, CreateAndJoinResponse};
pub use defend::{try_defend_a_square, DefendRequest, DefendResponse};
pub use join_existing::{try_join_an_existing_game, JoinExistingRequest, JoinExistingResponse};
pub use place_mine::{PlaceMineRequest, PlaceMineResponse};
pub use query::{
  try_query_game, try_query_grid_square, QueryGameRequest, QueryGameResponse, QueryGridResponse, QueryGridSquareRequest,
  QueryGridSquareResponse,
};
pub use start::{try_start, StartRequest, StartResponse};

pub const REQUESTS_COUNT: i32 = 30;
pub const GRID_SQUARE_DEFAULT_HEALTH: i32 = 60;
