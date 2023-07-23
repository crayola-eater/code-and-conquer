use crate::types::GameStatus;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum Error {
  #[error("Invalid coordinates row = {row}, column = {column}")]
  InvalidCoordinates { row: i32, column: i32 },

  #[error("Failed to {action}. Game status is currently {current:?}, but needs to be {required:?} to perform this action.")]
  InvalidGameStatus {
    current: GameStatus,
    required: GameStatus,
    action: &'static str,
  },

  #[error("Invalid credentials. Please recheck team_key and team_id")]
  InvalidCredentials,

  #[error("Invalid team role")]
  InvalidTeamRole,

  #[error("Failed to add team, ran out of team ids.")]
  RanOutOfTeamIds,

  #[error("No team found with id {team_id}.")]
  InvalidTeamId { team_id: i32 },

  #[error("Invalid game id {game_id}.")]
  InvalidGameId { game_id: i32 },

  #[error("Failed to attack square, please recheck request.")]
  FailedToAttackSquare,

  #[error("Failed to defend square, please recheck request.")]
  FailedToDefendSquare,

  #[error("Failed to query grid square, please recheck game_id and coordinates.")]
  FailedToQueryGridSquare,

  #[error("Display name is already taken, please choose another.")]
  TeamDisplayNameAlreadyTaken,

  #[error("This display name is not valid, please choose another.")]
  InvalidDisplayName,

  #[error("Game already exists. Cannot create and join an existing game.")]
  GameAlreadyCreated,

  #[error("Failed to start game, your team ({team_id}) is not the host of this game.")]
  OnlyHostCanStartGame { team_id: i32 },

  #[error("No more requests left, please wait before retrying.")]
  NoMoreRequestsLeft,

  #[error("Failed to find host for game (game id = {game_id})")]
  FailedToFindHost { game_id: i32 },

  #[error("Cannot join after host has started the game")]
  CannotJoinAfterHostHasStarted,

  #[error("Failed to connect to database {cause}")]
  FailedToConnectToDatabase { cause: String },

  #[error("Database error {cause}")]
  DatabaseError { cause: String },

  #[error("An unexpected error occurred: {message}")]
  Unexpected { message: &'static str },
}

impl std::convert::From<sqlx::Error> for Error {
  fn from(value: sqlx::Error) -> Self {
    if let sqlx::Error::Database(error) = &value {
      if let Some(pg_error) = error.try_downcast_ref::<sqlx::postgres::PgDatabaseError>() {
        if Some("role_is_valid") == pg_error.constraint() {
          return Self::InvalidTeamRole;
        }

        if error.is_unique_violation() {
          if let Some("team") = pg_error.table() {
            return Self::TeamDisplayNameAlreadyTaken;
          }
        }
      }
    }
    Self::DatabaseError {
      cause: value.to_string(),
    }
  }
}

pub type Result<T> = core::result::Result<T, Error>;
