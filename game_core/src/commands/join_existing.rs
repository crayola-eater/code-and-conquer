use crate::commands::REQUESTS_COUNT;
use crate::games::create_random_hex;
use crate::types::{Error, GameStatus, PgPool, Result, TeamRole};
use sqlx::types::Json;

#[derive(Debug)]
pub struct JoinExistingRequest {
  pub game_id: i32,
  pub display_name: String,
  pub team_role: TeamRole,
}

#[derive(Debug)]
pub struct JoinExistingResponse {
  pub team_id: i32,
  pub team_key: String,
}

pub async fn try_join_an_existing_game(pool: &PgPool, request: JoinExistingRequest) -> Result<JoinExistingResponse> {
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
  .fetch_one(pool)
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
