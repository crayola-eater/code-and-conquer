use crate::commands::GRID_SQUARE_DEFAULT_HEALTH;
use crate::types::{DatabaseErrorKind, DateTimeUtc, Error, GameStatus, GridSquare, Json, PgPool, Result, SenderDetails};
use postgres_syntax::sql;

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

pub async fn try_defend_a_square(pool: &PgPool, request: DefendRequest) -> Result<DefendResponse> {
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
    Option<DateTimeUtc>,
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
    .fetch_one(pool)
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
