use crate::commands::{
  try_attack_a_square, try_create_and_join_a_game, try_defend_a_square, try_join_an_existing_game, try_place_a_mine,
  try_query_game, try_query_grid_square, try_start,
};
use crate::types::{
  AttackRequest, AttackResponse, CreateAndJoinRequest, CreateAndJoinResponse, DefendRequest, DefendResponse, Error,
  JoinExistingRequest, JoinExistingResponse, PgPool, PlaceMineRequest, PlaceMineResponse, QueryGameRequest, QueryGameResponse,
  QueryGridResponse, QueryGridSquareRequest, QueryGridSquareResponse, Result, StartRequest, StartResponse,
};

use sqlx::postgres::PgPoolOptions;

pub async fn create_random_hex() -> Result<String> {
  use std::fmt::Write;
  use tokio::fs::File;
  use tokio::io::AsyncReadExt;

  let mut file = File::open("/dev/urandom").await.unwrap();
  let mut buffer = [0_u8; 15];
  let length = file.read_exact(&mut buffer).await.unwrap();

  buffer
    .iter()
    .take(length)
    .try_fold(String::with_capacity(buffer.len() * 2), |mut acc, byte| {
      write!(&mut acc, "{byte:02x?}").map(|_| acc)
    })
    .map_err(|_| Error::Unexpected {
      message: "failed to create random hex",
    })
}

pub async fn create_pool(database_name: Option<&str>) -> Result<PgPool> {
  let database_name = database_name.unwrap_or("postgres");
  // todo: load connection string via env variables
  let url = format!("postgres://postgres:password@localhost/{database_name}");

  PgPoolOptions::new()
    .max_connections(5)
    .connect(url.as_str())
    .await
    .map_err(|e| Error::FailedToConnectToDatabase { cause: e.to_string() })
}

pub async fn setup_database(db_pool: &PgPool) -> Result<()> {
  sqlx::query("DROP TABLE IF EXISTS mine, grid_square, game, team;")
    .execute(db_pool)
    .await?;

  sqlx::query(
    "
    CREATE TABLE game (
      id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
      status TEXT NOT NULL CHECK (status IN ('WaitingForRegistrations', 'Started', 'Ended')),
      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );",
  )
  .execute(db_pool)
  .await?;

  sqlx::query("
    CREATE TABLE team (
      id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
      game_id INTEGER NOT NULL REFERENCES game (id),
      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      display_name VARCHAR(30) NOT NULL,
      key VARCHAR(30) NOT NULL,
      role TEXT NOT NULL CONSTRAINT role_is_valid CHECK (role IN ('Minelayer', 'Spy', 'Cloaker')),
      role_used BOOLEAN NOT NULL DEFAULT FALSE,
      requests_left INTEGER NOT NULL CONSTRAINT requests_left_is_within_valid_range CHECK (requests_left BETWEEN 0 AND 30),
      time_of_last_command TIMESTAMPTZ CONSTRAINT time_of_last_command_either_null_or_gte_created_at CHECK (time_of_last_command IS NULL OR time_of_last_command >= created_at),
      UNIQUE (game_id, display_name),
      UNIQUE (game_id, key)
    );",
  )
  .execute(db_pool)
  .await?;

  sqlx::query(
    "
    CREATE TABLE grid_square (
      id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
      game_id INTEGER NOT NULL REFERENCES game (id),
      owner_id INTEGER NULL REFERENCES team (id),
      row_index INTEGER NOT NULL,
      column_index INTEGER NOT NULL,
      bonus INTEGER NOT NULL CHECK (bonus BETWEEN 0 AND 5),
      health INTEGER NOT NULL CHECK (health BETWEEN 0 AND 120),
      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      UNIQUE (game_id, row_index, column_index)
    );",
  )
  .execute(db_pool)
  .await?;

  sqlx::query(
    "
    CREATE TABLE mine (
      id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
      square_id INTEGER NOT NULL REFERENCES grid_square (id),
      game_id INTEGER NOT NULL REFERENCES game (id),
      owner_id INTEGER NOT NULL REFERENCES team (id),
      triggerer_id INTEGER REFERENCES team (id),
      UNIQUE (game_id, owner_id),
      UNIQUE (game_id, square_id)
    );
  ",
  )
  .execute(db_pool)
  .await?;

  Ok(())
}

#[derive(Debug)]
pub struct Games {
  db_pool: PgPool,
}

impl Games {
  pub async fn try_new(pool: PgPool) -> Result<Self> {
    Ok(Self { db_pool: pool })
  }

  pub async fn try_create_and_join_a_game(&mut self, request: CreateAndJoinRequest) -> Result<CreateAndJoinResponse> {
    try_create_and_join_a_game(&self.db_pool, request).await
  }

  pub async fn try_join_an_existing_game(&mut self, request: JoinExistingRequest) -> Result<JoinExistingResponse> {
    try_join_an_existing_game(&self.db_pool, request).await
  }

  pub async fn try_attack_a_square(&mut self, request: AttackRequest) -> Result<AttackResponse> {
    try_attack_a_square(&self.db_pool, request).await
  }

  pub async fn try_defend_a_square(&mut self, request: DefendRequest) -> Result<DefendResponse> {
    try_defend_a_square(&self.db_pool, request).await
  }

  pub async fn try_query_grid_square(&self, request: QueryGridSquareRequest) -> Result<QueryGridSquareResponse> {
    try_query_grid_square(&self.db_pool, request).await
  }

  pub async fn try_query_grid(&self) -> Result<QueryGridResponse> {
    todo!()
  }

  pub async fn try_query_game(&self, request: QueryGameRequest) -> Result<QueryGameResponse> {
    try_query_game(&self.db_pool, request).await
  }

  pub async fn try_place_a_mine(&mut self, request: PlaceMineRequest) -> Result<PlaceMineResponse> {
    try_place_a_mine(&self.db_pool, request).await
  }

  pub async fn try_start(&mut self, request: StartRequest) -> Result<StartResponse> {
    try_start(&self.db_pool, request).await
  }
}
