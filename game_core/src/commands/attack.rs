use crate::types::{DateTimeUtc, Error, GameStatus, GridSquare, Json, PgPool, Result, SenderDetails};
use postgres_syntax::sql;

#[derive(Debug)]
pub struct AttackRequest {
  pub game_id: i32,
  pub sender: SenderDetails,
  pub row_index: i32,
  pub column_index: i32,
}

#[derive(Debug)]
pub struct AttackResponse {
  pub square: GridSquare,
  pub conquered: bool,
  pub requests_left: i32,
}

pub async fn try_attack_a_square(pool: &PgPool, request: AttackRequest) -> Result<AttackResponse> {
  let mut tx = pool.begin().await?;

  let query = sql!(
    "
      SELECT
        game.id AS game_id,
        to_json(game.status) AS game_status,
        team.id AS team_id,
        key,
        requests_left
      FROM team
      INNER JOIN game ON team.id = $1 AND team.game_id = game.id
      LIMIT 1;
    "
  );

  let (game_id, Json(game_status), team_id, team_key, requests_left): (i32, Json<GameStatus>, i32, String, i32) =
    sqlx::query_as(query)
      .bind(request.sender.team_id)
      .fetch_optional(&mut *tx)
      .await?
      .ok_or(Error::InvalidTeamId {
        team_id: request.sender.team_id,
      })?;

  debug_assert_eq!(team_id, request.sender.team_id);

  Some(&team_key)
    .filter(|key| key.as_str() == request.sender.team_key)
    .ok_or(Error::InvalidCredentials)?;

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

  let query = sql!(
    "
      UPDATE team
      SET requests_left = requests_left - 1
      WHERE game_id = $1 AND id = $2 AND key = $3
      RETURNING requests_left;
    "
  );

  let (requests_left,): (i32,) = sqlx::query_as(query)
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

  let query = sql!(
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
        health;
    "
  );

  let (square_id, game_id, owner_id, created_at, row_index, column_index, bonus, health): (
    i32,
    i32,
    Option<i32>,
    DateTimeUtc,
    i32,
    i32,
    i32,
    i32,
  ) = sqlx::query_as(query)
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
    created_at,
    bonus,
    health,
    mine: None,
  };

  Ok(AttackResponse {
    conquered: square.health == 120,
    square,
    requests_left,
  })
}
