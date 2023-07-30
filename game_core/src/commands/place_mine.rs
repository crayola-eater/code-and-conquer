use crate::types::{
  DatabaseErrorKind, DateTimeUtc, Error, GameStatus, GridSquare, Json, Mine, PgPool, Result, SenderDetails, TeamRole,
};
use postgres_syntax::sql;

#[derive(Debug)]
pub struct PlaceMineRequest {
  pub game_id: i32,
  pub sender: SenderDetails,
  pub row_index: i32,
  pub column_index: i32,
}

#[derive(Debug)]
pub struct PlaceMineResponse {
  pub square: GridSquare,
  pub requests_left: i32,
}

pub async fn try_place_a_mine(pool: &PgPool, request: PlaceMineRequest) -> Result<PlaceMineResponse> {
  // if team found and creds ok and game found and game status is started:
  //    and team_role is minelayer
  //    and role not used
  //    insert mine into table
  //    return square, triggered mine

  let query = sql!(
    "
      WITH
        found_game AS (
          SELECT *
          FROM game
          WHERE id = $1
          LIMIT 1
          FOR UPDATE
        ),
        found_team AS (
          SELECT *
          FROM team
          WHERE id = $2
          LIMIT 1
          FOR UPDATE
        ),
        found_square AS (
          SELECT *
          FROM grid_square
          WHERE
            game_id = $1 AND row_index = $3 AND column_index = $4
          LIMIT 1
          FOR UPDATE
        ),
        err AS (
          SELECT
            to_json(
              CASE
                WHEN found_game.id IS NULL THEN $5
                WHEN found_team.id IS NULL OR found_team.id <> $2 OR found_team.key <> $6 THEN $7
                WHEN 0 = found_team.requests_left THEN $8
                WHEN found_square IS NULL THEN $9
                WHEN found_game.status <> $10 THEN $11
                WHEN found_team.role <> $12 THEN $13
                WHEN found_team.role_used THEN $14
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
            role_used = TRUE,
            time_of_last_command = NOW()
          FROM found_team
          WHERE team.id = found_team.id AND (SELECT error_kind IS NULL FROM err)
          RETURNING team.requests_left, team.role
        ),
        updated_mine AS (
          INSERT INTO mine (square_id, game_id, owner_id)
          SELECT found_square.id, $1, $2
          FROM found_square
          WHERE (SELECT error_kind IS NULL FROM err)
          ON CONFLICT (game_id, square_id) DO UPDATE
          SET owner_id = $2
        ),
        collated AS (
          SELECT
            to_json(err.error_kind) AS error_kind,
            to_json(found_game.status) AS status,
            to_json(updated_team.role) AS team_role,
            updated_team.requests_left,
            found_square.id AS square_id,
            found_square.bonus,
            found_square.created_at,
            found_square.health
          FROM err
          FULL JOIN updated_team ON TRUE
          FULL JOIN found_game ON TRUE
          FULL JOIN found_square ON TRUE
        )
      SELECT *
      FROM collated
    "
  );

  type Row = (
    Option<Json<DatabaseErrorKind>>,
    Option<Json<GameStatus>>,
    Option<Json<TeamRole>>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    Option<DateTimeUtc>,
    Option<i32>,
  );

  let (error_kind, game_status, team_role, requests_left, square_id, bonus, created_at, health): Row = sqlx::query_as(query)
    .bind(request.game_id)
    .bind(request.sender.team_id)
    .bind(request.row_index)
    .bind(request.column_index)
    .bind::<&'static str>(DatabaseErrorKind::InvalidGameId.into())
    .bind(&request.sender.team_key)
    .bind::<&'static str>(DatabaseErrorKind::InvalidCredentials.into())
    .bind::<&'static str>(DatabaseErrorKind::NoMoreRequestsLeft.into())
    .bind::<&'static str>(DatabaseErrorKind::InvalidCoordinates.into())
    .bind::<&'static str>(GameStatus::Started.into())
    .bind::<&'static str>(DatabaseErrorKind::InvalidGameStatus.into())
    .bind::<&'static str>(TeamRole::Minelayer.into())
    .bind::<&'static str>(DatabaseErrorKind::InvalidTeamRole.into())
    .bind::<&'static str>(DatabaseErrorKind::RoleAlreadyUsed.into())
    .fetch_one(pool)
    .await?;

  error_kind
    .map(|Json(error_kind)| match error_kind {
      DatabaseErrorKind::InvalidGameId => Error::InvalidGameId {
        game_id: request.game_id,
      },
      DatabaseErrorKind::InvalidCoordinates => Error::InvalidCoordinates {
        row: request.row_index,
        column: request.column_index,
      },
      DatabaseErrorKind::InvalidCredentials => Error::InvalidCredentials,
      DatabaseErrorKind::InvalidGameStatus => Error::InvalidGameStatus {
        current: game_status
          .map(|Json(game_status)| game_status)
          .unwrap_or(GameStatus::WaitingForRegistrations),
        required: GameStatus::Started,
        action: "place mine",
      },
      DatabaseErrorKind::NoMoreRequestsLeft => Error::NoMoreRequestsLeft,
      DatabaseErrorKind::InvalidTeamRole => Error::OnlyMinelayersCanPlaceMines {
        team_role: team_role.map_or(TeamRole::Spy, |Json(team_role)| team_role),
      },
      DatabaseErrorKind::RoleAlreadyUsed => Error::RoleAlreadyUsed,
    })
    .map_or(Ok(()), Err)?;

  let (requests_left, square_id, bonus, created_at, health) = requests_left
    .and_then(|requests_left| Some((requests_left, square_id?, bonus?, created_at?, health?)))
    .ok_or(Error::Unexpected {
      message: "failed to place mine",
    })?;

  let mine = Mine {
    placed_by: request.sender.team_id,
    triggered_by: None,
  };

  let square = GridSquare {
    id: square_id,
    bonus,
    created_at,
    game_id: request.game_id,
    health,
    mine: Some(mine),
    owner_id: None,
    column: request.column_index,
    row: request.row_index,
  };

  Ok(PlaceMineResponse { square, requests_left })
}
