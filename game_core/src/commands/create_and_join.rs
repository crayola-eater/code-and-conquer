use crate::commands::{GRID_SQUARE_DEFAULT_HEALTH, REQUESTS_COUNT};
use crate::games::create_random_hex;
use crate::types::{GameStatus, PgPool, Result, TeamRole};
use postgres_syntax::sql;

#[derive(Debug)]
pub struct CreateAndJoinRequest {
  pub display_name: String,
  pub team_role: TeamRole,
}

#[derive(Debug)]
pub struct CreateAndJoinResponse {
  pub game_id: i32,
  pub team_id: i32,
  pub team_key: String,
}

pub async fn try_create_and_join_a_game(pool: &PgPool, request: CreateAndJoinRequest) -> Result<CreateAndJoinResponse> {
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

  let query = sql!(
    "
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
      FROM created_team;
    "
  );

  let (game_id, team_id, team_key): (i32, i32, String) = sqlx::query_as(query)
    .bind(&request.display_name)
    .bind(team_key.as_str())
    .bind(role)
    .bind(REQUESTS_COUNT)
    .bind(status)
    .bind(squares)
    .fetch_one(pool)
    .await?;

  Ok(CreateAndJoinResponse {
    game_id,
    team_id,
    team_key,
  })
}
