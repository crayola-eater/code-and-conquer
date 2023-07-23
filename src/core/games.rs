use crate::types::{
  AttackRequest, AttackResponse, Command, CommandResponse, CreateAndJoinRequest, CreateAndJoinResponse, DatabaseErrorKind,
  DefendRequest, DefendResponse, Error, Game, GameStatus, GridSquare, JoinExistingRequest, JoinExistingResponse,
  PlaceMineRequest, PlaceMineResponse, QueryGameRequest, QueryGameResponse, QueryGridResponse, QueryGridSquareRequest,
  QueryGridSquareResponse, Result, StartRequest, StartResponse, Team,
};
use chrono::{DateTime, Utc};
use postgres_syntax::sql;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Json;
use sqlx::PgPool;

const REQUESTS_COUNT: i32 = 30;
const GRID_SQUARE_DEFAULT_HEALTH: i32 = 60;

pub async fn create_random_hex() -> Result<'static, String> {
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

pub async fn create_pool<'a>(database_name: Option<&str>) -> Result<'a, PgPool> {
  let database_name = database_name.unwrap_or("postgres");
  // todo: load connection string via env variables
  let url = format!("postgres://postgres:password@localhost/{database_name}");

  PgPoolOptions::new()
    .max_connections(5)
    .connect(url.as_str())
    .await
    .map_err(|e| Error::FailedToConnectToDatabase { cause: e.to_string() })
}

pub async fn setup_database<'a>(db_pool: &PgPool) -> Result<'a, ()> {
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
  pub async fn try_new<'a>(pool: PgPool) -> Result<'a, Self> {
    Ok(Self { db_pool: pool })
  }

  async fn try_create_and_join_a_game<'a>(&mut self, request: CreateAndJoinRequest) -> Result<'a, CreateAndJoinResponse> {
    let role: &'static str = request.team_role.into();
    let team_key = create_random_hex().await?;
    let status: &'static str = GameStatus::WaitingForRegistrations.into();

    let squares = (0..5)
      .flat_map(|row_index| {
        (0..5).map(move |column_index| {
          serde_json::json!({
            "row_index": row_index,
            "column_index": column_index,
            "bonus": 0,
            "health": GRID_SQUARE_DEFAULT_HEALTH,
          })
        })
      })
      .collect::<Vec<_>>();

    let squares = serde_json::Value::Array(squares);

    let (game_id, team_id, team_key): (i32, i32, String) = sqlx::query_as(
      r#"
      WITH
        created_game AS (
          INSERT INTO game (status)
          VALUES ($5)
          RETURNING id
        ),
        parsed AS (
          SELECT *
          FROM jsonb_to_recordset($6) AS X(row_index INTEGER, column_index INTEGER, bonus INTEGER, health INTEGER)
        ),
        created_grid_squares AS (
          INSERT INTO grid_square (game_id, row_index, column_index, bonus, health)
          SELECT created_game.id, parsed.row_index, parsed.column_index, parsed.bonus, parsed.health
          FROM parsed, created_game
        ),
        created_team AS (
          INSERT INTO team (game_id, display_name, key, role, requests_left)
          SELECT created_game.id, $1, $2, $3, $4
          FROM created_game
          RETURNING game_id, id, key
        )
      SELECT
        game_id,
        id AS team_id,
        key AS team_key
      FROM created_team;"#,
    )
    .bind(&request.display_name)
    .bind(team_key.as_str())
    .bind(role)
    .bind(REQUESTS_COUNT)
    .bind(status)
    .bind(squares)
    .fetch_one(&self.db_pool)
    .await?;

    Ok(CreateAndJoinResponse {
      game_id,
      team_id,
      team_key,
    })
  }

  async fn try_join_an_existing_game<'a>(&mut self, request: JoinExistingRequest) -> Result<'a, JoinExistingResponse> {
    // try find game (otherwise err_invalid_game_id)
    // proposed = create new team
    // if game.status != waiting_for_reg:
    //    err_cannot_join_after_started
    // if game.teams contains proposed.display_name:
    //    err_display_name_already_taken

    let expected_status: &'static str = GameStatus::WaitingForRegistrations.into();
    let role: &'static str = request.team_role.into();
    let team_key = create_random_hex().await?;

    let row: (Option<i32>, Option<String>, Option<Json<GameStatus>>) = sqlx::query_as(
      "
      WITH
        found AS (
          SELECT status
          FROM game
          WHERE game.id = $1
          FOR UPDATE
        ),
        to_insert AS (
          SELECT $1, $2, $3, $4, $5
          FROM found
          WHERE found.status = $6
        ),
        inserted AS (
          INSERT INTO team (game_id, display_name, key, role, requests_left)
          SELECT *
          FROM to_insert
          RETURNING id, key
        ),
        collated AS (
          SELECT inserted.id, inserted.key, to_json(found.status)
          FROM found
          LEFT JOIN inserted
          ON TRUE
        )
      SELECT * FROM collated;",
    )
    .bind(request.game_id)
    .bind(request.display_name)
    .bind(team_key.as_str())
    .bind(role)
    .bind(REQUESTS_COUNT)
    .bind(expected_status)
    .fetch_one(&self.db_pool)
    .await?;

    match row {
      (Some(team_id), Some(team_key), _) => Ok(JoinExistingResponse { team_id, team_key }),
      (_, _, Some(Json(old_status))) if old_status != GameStatus::WaitingForRegistrations => {
        Err(Error::CannotJoinAfterHostHasStarted)
      }
      _ => Err(Error::Unexpected {
        message: "Failed to join game. Please recheck your request and try again.",
      }),
    }
  }

  async fn try_attack_a_square<'a>(&mut self, request: AttackRequest) -> Result<'a, AttackResponse> {
    let mut tx = self.db_pool.begin().await?;

    let (game_id, Json(game_status), team_id, team_key, requests_left): (i32, Json<GameStatus>, i32, String, i32) =
      sqlx::query_as(
        "
        SELECT
          game.id AS game_id,
          to_json(game.status) AS game_status,
          team.id AS team_id,
          key,
          requests_left
        FROM team
        INNER JOIN game ON team.id = $1 AND team.game_id = game.id
        LIMIT 1;",
      )
      .bind(request.sender.team_id)
      .fetch_optional(&mut *tx)
      .await?
      .ok_or_else(|| Error::InvalidTeamId {
        team_id: request.sender.team_id,
      })?;

    debug_assert_eq!(team_id, request.sender.team_id);

    Some(&team_key)
      .filter(|key| key.as_str() == &request.sender.team_key)
      .ok_or_else(|| Error::InvalidCredentials)?;

    Some(game_id)
      .filter(|game_id| *game_id == request.game_id)
      .ok_or(Error::InvalidGameId {
        game_id: request.game_id,
      })?;

    Some(requests_left)
      .filter(|count| count > &0)
      .ok_or(Error::NoMoreRequestsLeft)?;

    Some(game_status)
      .filter(|status| *status == GameStatus::Started)
      .ok_or(Error::InvalidGameStatus {
        current: game_status,
        required: GameStatus::Started,
        action: "attack square",
      })?;

    let (requests_left,): (i32,) = sqlx::query_as(
      "
      UPDATE team
      SET requests_left = requests_left - 1
      WHERE game_id = $1 AND id = $2 AND key = $3
      RETURNING requests_left;",
    )
    .bind(request.game_id)
    .bind(request.sender.team_id)
    .bind(&request.sender.team_key)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
      eprintln!("{e:?}");
      Error::FailedToAttackSquare
    })?;

    debug_assert!(requests_left >= 0);

    let (square_id, game_id, owner_id, created_at, row_index, column_index, bonus, health): (
      i32,
      i32,
      Option<i32>,
      DateTime<Utc>,
      i32,
      i32,
      i32,
      i32,
    ) = sqlx::query_as(
      "
      UPDATE grid_square
      SET
        owner_id = (CASE WHEN health > 1 THEN owner_id ELSE $1 END),
        health = (CASE WHEN health > 1 THEN health - 1 ELSE 120 END)
      WHERE game_id = $2 AND row_index = $3 AND column_index = $4
      RETURNING
        id,
        game_id,
        owner_id,
        created_at,
        row_index,
        column_index,
        bonus,
        health;",
    )
    .bind(request.sender.team_id)
    .bind(request.game_id)
    .bind(request.row_index)
    .bind(request.column_index)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
      eprintln!("{e:?}");
      Error::FailedToAttackSquare
    })?;

    tx.commit().await?;

    let square = GridSquare {
      id: square_id,
      game_id,
      owner_id,
      row: row_index,
      column: column_index,
      created_at: created_at,
      bonus: bonus,
      health: health,
      mine: None,
    };

    Ok(AttackResponse {
      conquered: square.health == 120,
      square,
      requests_left,
    })
  }

  async fn try_defend_a_square<'a>(&mut self, request: DefendRequest) -> Result<'a, DefendResponse> {
    // if creds are ok AND game id is ok AND game status is "started" AND row is ok AND column is ok AND requests_left is greater than 0:
    //    update grid_square
    //    set health =  MAX(
    //      health + 1,
    //      60 if owner_id is null otherwise 120
    //    ),
    //    requests_left--
    //    where game_id, row, column all match

    type Row = (
      Option<Json<DatabaseErrorKind>>,
      Option<Json<GameStatus>>,
      Option<i32>,
      Option<i32>,
      Option<i32>,
      Option<i32>,
      Option<i32>,
      Option<i32>,
      Option<DateTime<Utc>>,
      Option<i32>,
      Option<i32>,
    );

    let query = sql!(
      "
      WITH
        found_game AS (
          SELECT id, status
          FROM game
          WHERE game.id = $1
          LIMIT 1
        ),
        found_team AS (
          SELECT id, key, requests_left
          FROM team
          WHERE team.id = $2
          LIMIT 1
        ),
        found_square AS (
          SELECT id, row_index, column_index, bonus, health
          FROM grid_square
          WHERE
            game_id = $1
            AND row_index = $4
            AND column_index = $5
          LIMIT 1
        ),
        err AS (
          SELECT
            to_json(
              CASE
                WHEN found_game.id IS NULL THEN $6
                WHEN found_team.id IS NULL OR found_team.id <> $7 OR found_team.key <> $8 THEN $9
                WHEN 0 = found_team.requests_left THEN $10
                WHEN found_square IS NULL THEN $11
                WHEN found_game.status <> $12 THEN $13
                ELSE NULL
              END
            ) AS error_kind
          FROM found_game
          FULL JOIN found_team ON TRUE
          FULL JOIN found_square ON TRUE
        ),
        updated_team AS (
          UPDATE team
          SET
            requests_left = team.requests_left - 1,
            time_of_last_command = NOW()
          FROM found_team
          WHERE team.id = found_team.id AND (SELECT error_kind IS NULL FROM err)
          RETURNING team.requests_left
        ),
        updated_square AS (
          UPDATE grid_square
          SET
            health = LEAST(
              grid_square.health + 1,
              CASE WHEN grid_square.owner_id IS NULL THEN $3 ELSE 120 END
            )
          FROM found_square
          WHERE grid_square.id = found_square.id AND (SELECT error_kind IS NULL FROM err)
          RETURNING
            grid_square.*
        ),
        updated AS (
          SELECT
            to_json(err.error_kind) AS error_kind,
            to_json(found_game.status) AS status,
            updated_team.requests_left,
            updated_square.id,
            updated_square.game_id,
            updated_square.owner_id,
            updated_square.row_index,
            updated_square.column_index,
            updated_square.created_at,
            updated_square.bonus,
            updated_square.health
          FROM err
          FULL JOIN found_game ON TRUE
          FULL JOIN found_team ON TRUE
          FULL JOIN found_square ON TRUE
          FULL JOIN updated_team ON TRUE
          FULL JOIN updated_square ON TRUE
        )
      SELECT *
      FROM updated;"
    );

    let (
      error_kind,
      game_status,
      requests_left,
      square_id,
      game_id,
      owner_id,
      row_index,
      column_index,
      created_at,
      bonus,
      health,
    ): Row = sqlx::query_as(query)
      .bind(request.game_id)
      .bind(request.sender.team_id)
      .bind(GRID_SQUARE_DEFAULT_HEALTH)
      .bind(request.row_index)
      .bind(request.column_index)
      .bind::<&'static str>(DatabaseErrorKind::InvalidGameId.into())
      .bind(request.sender.team_id)
      .bind(&request.sender.team_key)
      .bind::<&'static str>(DatabaseErrorKind::InvalidCredentials.into())
      .bind::<&'static str>(DatabaseErrorKind::NoMoreRequestsLeft.into())
      .bind::<&'static str>(DatabaseErrorKind::InvalidCoordinates.into())
      .bind::<&'static str>(GameStatus::Started.into())
      .bind::<&'static str>(DatabaseErrorKind::InvalidGameStatus.into())
      .fetch_one(&self.db_pool)
      .await?;

    error_kind
      .map(|Json(error_kind)| match error_kind {
        DatabaseErrorKind::InvalidCoordinates => Error::InvalidCoordinates {
          row: request.row_index,
          column: request.column_index,
        },
        DatabaseErrorKind::InvalidCredentials => Error::InvalidCredentials,
        DatabaseErrorKind::InvalidGameId => Error::InvalidGameId {
          game_id: request.game_id,
        },
        DatabaseErrorKind::NoMoreRequestsLeft => Error::NoMoreRequestsLeft,
        DatabaseErrorKind::InvalidGameStatus => Error::InvalidGameStatus {
          current: game_status
            .map(|Json(game_status)| game_status)
            .unwrap_or(GameStatus::WaitingForRegistrations),
          required: GameStatus::Started,
          action: "defend square",
        },
      })
      .map_or(Ok(()), Err)?;

    let (square_id, _, requests_left, game_id, owner_id, row_index, column_index, created_at, bonus, health) = square_id
      .and_then(|square_id| {
        Some((
          square_id,
          game_status?,
          requests_left?,
          game_id?,
          owner_id,
          row_index?,
          column_index?,
          created_at?,
          bonus?,
          health?,
        ))
      })
      .ok_or(Error::FailedToDefendSquare)?;

    let square = GridSquare {
      bonus,
      column: column_index,
      row: row_index,
      created_at,
      game_id,
      health,
      id: square_id,
      mine: None,
      owner_id,
    };

    Ok(DefendResponse { square, requests_left })
  }

  async fn try_query_grid_square<'a>(&self, request: QueryGridSquareRequest) -> Result<'a, QueryGridSquareResponse> {
    type Row = (i32, i32, Option<i32>, DateTime<Utc>, i32, i32, i32, i32);
    let (square_id, game_id, owner_id, created_at, row_index, column_index, bonus, health): Row = sqlx::query_as(
      "
      SELECT
        id,
        game_id,
        owner_id,
        created_at,
        row_index,
        column_index,
        bonus,
        health
      FROM grid_square
      WHERE game_id = $1 AND row_index = $2 AND column_index = $3
      LIMIT 1;
    ",
    )
    .bind(request.game_id)
    .bind(request.row_index)
    .bind(request.column_index)
    .fetch_optional(&self.db_pool)
    .await?
    .ok_or(Error::FailedToQueryGridSquare)?;

    let square = GridSquare {
      id: square_id,
      game_id,
      owner_id,
      row: row_index,
      column: column_index,
      created_at: created_at,
      bonus: bonus,
      health: health,
      mine: None,
    };

    Ok(QueryGridSquareResponse { square })
  }

  async fn try_query_grid<'a>(&self) -> Result<'a, QueryGridResponse> {
    todo!()
  }

  async fn try_query_game<'a>(&self, request: QueryGameRequest) -> Result<'a, QueryGameResponse> {
    type Row = (i32, DateTime<Utc>, Json<GameStatus>, Json<Vec<Team>>, Json<Vec<GridSquare>>);

    let (game_id, created_at, Json(status), Json(teams), Json(grid)): Row = sqlx::query_as(
      "
      WITH
        current_teams AS (
          SELECT json_agg(team.*) AS teams
          FROM team
          WHERE team.game_id = $1
        ),
        grid AS (
          SELECT json_agg(grid_square.*) AS grid_squares
          FROM grid_square
          WHERE grid_square.game_id = $1
        ),
        aggregated_game AS (
          SELECT
            game.id,
            game.created_at,
            to_json(game.status) AS status,
            current_teams.teams AS teams, 
            grid.grid_squares AS grid
          FROM game, current_teams, grid
          WHERE game.id = $1
        )
      SELECT *
      FROM aggregated_game
    ",
    )
    .bind(request.game_id)
    .fetch_one(&self.db_pool)
    .await?;

    let game = Game {
      id: game_id,
      created_at,
      grid,
      status,
      teams,
    };

    Ok(QueryGameResponse { game })
  }

  async fn try_place_a_mine<'a>(&mut self, request: PlaceMineRequest) -> Result<'a, PlaceMineResponse> {
    todo!("{request:?}")
  }

  async fn try_start<'a>(&mut self, request: StartRequest) -> Result<'a, StartResponse> {
    let expected_status: &'static str = GameStatus::WaitingForRegistrations.into();
    let next_status: &'static str = GameStatus::Started.into();

    // only update if game_id exists and requester's team_id == game's host's team_id
    // AND game status is waiting_for_registrations

    type Row = (
      Option<i32>,
      Option<Json<GameStatus>>,
      Option<Json<GameStatus>>,
      Option<i32>,
      Option<String>,
    );

    let row: Row = sqlx::query_as(
      "
        WITH
          previous_status AS (
            SELECT status
            FROM game
            WHERE game.id = $1
          ),
          host_team AS (
            SELECT id, key
            FROM team
            WHERE team.game_id = $1
            ORDER BY id
            LIMIT 1
          ),
          updated AS (
            UPDATE game SET status = $2
            WHERE 
              game.id = $1
              AND game.status = $3
              AND (
                SELECT host_team.id = $4 AND host_team.key = $5
                FROM host_team
              )
            RETURNING id, status
          ),
          collated AS (
            SELECT
              updated.id AS game_id,
              to_json(updated.status) AS status,
              to_json(previous_status.status) AS previous_status,
              host_team.id AS host_team_id,
              host_team.key AS host_team_key
            FROM previous_status, host_team
            LEFT JOIN updated
            ON TRUE
          )
        SELECT *
        FROM collated;",
    )
    .bind(request.game_id)
    .bind(next_status)
    .bind(expected_status)
    .bind(request.sender.team_id)
    .bind(&request.sender.team_key)
    .fetch_one(&self.db_pool)
    .await?;

    match row {
      (Some(game_id), Some(Json(status)), _, _, _) => Ok(StartResponse { game_id, status }),
      // old_status has a simple WHERE clause
      // so if it's missing and sql was bug free, then game_id must have been invalid
      (None, None, None, None, None) | (_, _, None, _, _) => Err(Error::InvalidGameId {
        game_id: request.game_id,
      }),
      (_, _, Some(Json(old_status)), _, _) if old_status != GameStatus::WaitingForRegistrations => {
        Err(Error::InvalidGameStatus {
          current: old_status,
          required: GameStatus::WaitingForRegistrations,
          action: "start game",
        })
      }
      (_, _, _, Some(host_team_id), _) if host_team_id != request.sender.team_id => Err(Error::OnlyHostCanStartGame {
        team_id: request.sender.team_id,
      }),
      (_, _, _, _, Some(host_team_key)) if host_team_key != request.sender.team_key => Err(Error::InvalidCredentials),
      (_, _, _, None, _) => Err(Error::FailedToFindHost {
        game_id: request.game_id,
      }),
      _ => Err(Error::Unexpected {
        message: "Failed to start game. Please recheck request and try again.",
      }),
    }
  }

  pub async fn try_process_command<'a>(&mut self, command: Command) -> Result<'a, CommandResponse> {
    match command {
      Command::CreateAndJoin(options) => Ok(CommandResponse::CreateAndJoin(
        self.try_create_and_join_a_game(options).await?,
      )),
      Command::JoinExisting(options) => Ok(CommandResponse::JoinExisting(self.try_join_an_existing_game(options).await?)),
      Command::Attack(options) => Ok(CommandResponse::Attack(self.try_attack_a_square(options).await?)),
      Command::Defend(options) => Ok(CommandResponse::Defend(self.try_defend_a_square(options).await?)),
      Command::QueryGridSquare(options) => Ok(CommandResponse::QueryGridSquare(self.try_query_grid_square(options).await?)),
      Command::QueryGrid => Ok(CommandResponse::QueryGrid(self.try_query_grid().await?)),
      Command::QueryGame(options) => Ok(CommandResponse::QueryGame(self.try_query_game(options).await?)),
      Command::PlaceMine(options) => Ok(CommandResponse::PlaceMine(self.try_place_a_mine(options).await?)),
      Command::Start(options) => Ok(CommandResponse::Start(self.try_start(options).await?)),
    }
  }
}
