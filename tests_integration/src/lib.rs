use game_core::games;
use game_core::types::{
  CreateAndJoinRequest, GameStatus, Games, JoinExistingRequest, Result, SenderDetails, StartRequest, TeamRole,
};

#[derive(Debug)]
pub struct TestSetup {
  pub games: Games,
  pub game_id: i32,
  pub added: Vec<(i32, String)>,
}

pub async fn setup_with_players<'b, T>(teams: impl IntoIterator<Item = T>) -> Result<TestSetup>
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
  let mut teams_iter = teams.into_iter().map(|team| team.borrow().clone());

  let game_id = {
    let (display_name, role) = teams_iter.next().unwrap();
    let request = CreateAndJoinRequest {
      display_name: display_name.to_string(),
      team_role: role,
    };
    let response = games.try_create_and_join_a_game(request).await.unwrap();
    added.push((response.team_id, response.team_key));
    response.game_id
  };

  for (display_name, role) in teams_iter {
    games
      .try_join_an_existing_game(JoinExistingRequest {
        game_id,
        display_name: display_name.to_string(),
        team_role: role,
      })
      .await
      .map(|response| added.push((response.team_id, response.team_key)))?;
  }

  Ok(TestSetup {
    games,
    game_id: game_id,
    added,
  })
}

pub async fn start_game(games: &mut Games, game_id: i32, host_id: i32, host_key: String) {
  let sender = SenderDetails {
    team_id: host_id,
    team_key: host_key,
  };
  let start = StartRequest { game_id, sender };
  let response = games.try_start(start).await.unwrap();
  assert_eq!(response.game_id, game_id);
  assert_eq!(response.status, GameStatus::Started);
}
