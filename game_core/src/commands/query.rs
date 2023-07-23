use crate::types::{DateTimeUtc, Error, Game, GameStatus, GridSquare, Json, PgPool, Result, Team};
use postgres_syntax::sql;

#[derive(Debug)]
pub struct QueryGameRequest {
  pub game_id: i32,
}

#[derive(Debug)]
pub struct QueryGameResponse {
  pub game: Game,
}

#[derive(Debug)]
pub struct QueryGridSquareRequest {
  pub game_id: i32,
  pub row_index: i32,
  pub column_index: i32,
}

#[derive(Debug)]
pub struct QueryGridSquareResponse {
  pub square: GridSquare,
}

#[derive(Debug)]
pub struct QueryGridResponse {}

pub async fn try_query_grid_square(pool: &PgPool, request: QueryGridSquareRequest) -> Result<QueryGridSquareResponse> {
  let query = sql!(
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
    "
  );

  type Row = (i32, i32, Option<i32>, DateTimeUtc, i32, i32, i32, i32);
  let (square_id, game_id, owner_id, created_at, row_index, column_index, bonus, health): Row = sqlx::query_as(query)
    .bind(request.game_id)
    .bind(request.row_index)
    .bind(request.column_index)
    .fetch_optional(pool)
    .await?
    .ok_or(Error::FailedToQueryGridSquare)?;

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

  Ok(QueryGridSquareResponse { square })
}

pub async fn try_query_game(pool: &PgPool, request: QueryGameRequest) -> Result<QueryGameResponse> {
  let query = sql!(
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
      FROM aggregated_game;
    "
  );

  type Row = (i32, DateTimeUtc, Json<GameStatus>, Json<Vec<Team>>, Json<Vec<GridSquare>>);

  let (game_id, created_at, Json(status), Json(teams), Json(grid)): Row =
    sqlx::query_as(query).bind(request.game_id).fetch_one(pool).await?;

  let game = Game {
    id: game_id,
    created_at,
    grid,
    status,
    teams,
  };

  Ok(QueryGameResponse { game })
}
