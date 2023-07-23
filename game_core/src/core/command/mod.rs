pub mod attack;
pub mod create_and_join;
pub mod defend;
pub mod join_existing;
pub mod place_mine;
pub mod query;
pub mod start;

#[derive(Debug)]
pub enum Command {
  Attack(attack::AttackRequest),
  CreateAndJoin(create_and_join::CreateAndJoinRequest),
  Defend(defend::DefendRequest),
  JoinExisting(join_existing::JoinExistingRequest),
  PlaceMine(place_mine::PlaceMineRequest),
  QueryGridSquare(query::QueryGridSquareRequest),
  QueryGrid,
  QueryGame(query::QueryGameRequest),
  Start(start::StartRequest),
}

#[derive(Debug)]
pub enum CommandResponse {
  Attack(attack::AttackResponse),
  CreateAndJoin(create_and_join::CreateAndJoinResponse),
  Defend(defend::DefendResponse),
  JoinExisting(join_existing::JoinExistingResponse),
  PlaceMine(place_mine::PlaceMineResponse),
  QueryGridSquare(query::QueryGridSquareResponse),
  QueryGrid(query::QueryGridResponse),
  QueryGame(query::QueryGameResponse),
  Start(start::StartResponse),
}
