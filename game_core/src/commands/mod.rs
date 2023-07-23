mod attack;
mod create_and_join;
mod defend;
mod join_existing;
mod place_mine;
mod query;
mod start;

pub use attack::{AttackRequest, AttackResponse};
pub use create_and_join::{try_create_and_join_a_game, CreateAndJoinRequest, CreateAndJoinResponse};
pub use defend::{DefendRequest, DefendResponse};
pub use join_existing::{try_join_an_existing_game, JoinExistingRequest, JoinExistingResponse};
pub use place_mine::{PlaceMineRequest, PlaceMineResponse};
pub use query::{QueryGameRequest, QueryGameResponse, QueryGridResponse, QueryGridSquareRequest, QueryGridSquareResponse};
pub use start::{StartRequest, StartResponse};

pub const REQUESTS_COUNT: i32 = 30;
pub const GRID_SQUARE_DEFAULT_HEALTH: i32 = 60;
