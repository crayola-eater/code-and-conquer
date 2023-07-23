use crate::types::{Error, GameStatus, Json, PgPool, Result, SenderDetails};
use postgres_syntax::sql;

#[derive(Debug)]
pub struct StartRequest {
  pub game_id: i32,
  pub sender: SenderDetails,
}

#[derive(Debug)]
pub struct StartResponse {
  pub game_id: i32,
  pub status: GameStatus,
}

pub async fn try_start(pool: &PgPool, request: StartRequest) -> Result<StartResponse> {
  let expected_status: &'static str = GameStatus::WaitingForRegistrations.into();
  let next_status: &'static str = GameStatus::Started.into();

  // only update if game_id exists and requester's team_id == game's host's team_id
  // AND game status is waiting_for_registrations

  let query = sql!(
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
      FROM collated;
    "
  );

  type Row = (
    Option<i32>,
    Option<Json<GameStatus>>,
    Option<Json<GameStatus>>,
    Option<i32>,
    Option<String>,
  );

  let row: Row = sqlx::query_as(query)
    .bind(request.game_id)
    .bind(next_status)
    .bind(expected_status)
    .bind(request.sender.team_id)
    .bind(&request.sender.team_key)
    .fetch_one(pool)
    .await?;

  match row {
    (Some(game_id), Some(Json(status)), _, _, _) => Ok(StartResponse { game_id, status }),
    // old_status has a simple WHERE clause
    // so if it's missing and sql was bug free, then game_id must have been invalid
    (None, None, None, None, None) | (_, _, None, _, _) => Err(Error::InvalidGameId {
      game_id: request.game_id,
    }),
    (_, _, Some(Json(old_status)), _, _) if old_status != GameStatus::WaitingForRegistrations => Err(Error::InvalidGameStatus {
      current: old_status,
      required: GameStatus::WaitingForRegistrations,
      action: "start game",
    }),
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
