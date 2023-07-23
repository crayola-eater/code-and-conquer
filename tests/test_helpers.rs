use code_and_conquer::core::games;
use code_and_conquer::types::{
  Command, CommandResponse, CreateAndJoinRequest, GameStatus, Games, JoinExistingRequest, Result, SenderDetails, StartRequest,
  TeamRole,
};

#[derive(Debug)]
pub struct TestSetup {
  pub games: Games,
  pub game_id: i32,
  pub added: Vec<(i32, String)>,
}

pub async fn setup_with_players<'a, 'b, T>(teams: impl IntoIterator<Item = T>) -> Result<'a, TestSetup>
where
  T: core::borrow::Borrow<(&'b str, TeamRole)>,
{
  let hex = games::create_random_hex().await.unwrap();
  let database_name = format!("test_{hex}");
  let pool = {
    let pool = games::create_pool(None).await.unwrap();
    let _ = sqlx::query(&format!("CREATE DATABASE {database_name};"))
      .execute(&pool)
      .await
      .unwrap();
    let _ = sqlx::query(&format!("ALTER DATABASE {database_name} SET log_statement = 'all';"))
      .execute(&pool)
      .await
      .unwrap();
    let pool = games::create_pool(Some(&database_name)).await.unwrap();
    games::setup_database(&pool).await.unwrap();
    pool
  };

  let mut games = games::Games::try_new(pool).await?;
  let mut added = Vec::new();

  let mut game_id = Option::<i32>::None;

  for (i, team) in teams.into_iter().enumerate() {
    let (display_name, raw_role) = team.borrow();
    let command = if i > 0 {
      Command::JoinExisting(JoinExistingRequest {
        game_id: game_id.unwrap(),
        display_name: display_name.to_string(),
        team_role: *raw_role,
      })
    } else {
      Command::CreateAndJoin(CreateAndJoinRequest {
        display_name: display_name.to_string(),
        team_role: *raw_role,
      })
    };

    let (team_id, team_key) = match games.try_process_command(command).await? {
      CommandResponse::CreateAndJoin(response) => {
        game_id.get_or_insert(response.game_id);
        (response.team_id, response.team_key)
      }
      CommandResponse::JoinExisting(response) => (response.team_id, response.team_key),
      unexpected => unreachable!("{unexpected:?}"),
    };

    added.push((team_id, team_key));
  }

  Ok(TestSetup {
    games,
    game_id: game_id.unwrap(),
    added,
  })
}

#[allow(dead_code)]
pub async fn start_game(games: &mut Games, game_id: i32, host_id: i32, host_key: String) {
  let sender = SenderDetails {
    team_id: host_id,
    team_key: host_key,
  };
  let start = StartRequest { game_id, sender };
  let command = Command::Start(start);
  let response = match games.try_process_command(command).await.unwrap() {
    CommandResponse::Start(response) => response,
    unexpected => unreachable!("{unexpected:?}"),
  };
  assert_eq!(response.game_id, game_id);
  assert_eq!(response.status, GameStatus::Started);
}
