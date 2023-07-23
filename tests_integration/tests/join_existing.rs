use game_core::types::{Error, JoinExistingRequest, QueryGameRequest, TeamRole};
use rstest::*;
use tests_integration::{setup_with_players, start_game, TestSetup};

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_join_with_a_duplicate_display_name(
  #[values("Team Copycats", "Team-12", "_", "you")] display_name: &str,
  #[values(TeamRole::Spy, TeamRole::Minelayer, TeamRole::Cloaker)] team_role: TeamRole,
) {
  let TestSetup { mut games, game_id, .. } = setup_with_players(&[(display_name, team_role)]).await.unwrap();

  for role in [TeamRole::Spy, TeamRole::Minelayer, TeamRole::Cloaker] {
    let error = games
      .try_join_an_existing_game(JoinExistingRequest {
        display_name: display_name.to_string(),
        team_role: role,
        game_id,
      })
      .await
      .unwrap_err();

    assert_eq!(error, Error::TeamDisplayNameAlreadyTaken);
  }
}

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_join_game_once_started(
  #[values("Team Copycats", "Team-12", "_", "you")] display_name: &str,
  #[values(TeamRole::Spy, TeamRole::Minelayer, TeamRole::Cloaker)] team_role: TeamRole,
) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(&[(display_name, team_role)]).await.unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  for role in [TeamRole::Spy, TeamRole::Minelayer, TeamRole::Cloaker] {
    let error = games
      .try_join_an_existing_game(JoinExistingRequest {
        display_name: format!("{display_name}_{role:?}"),
        team_role: role,
        game_id,
      })
      .await
      .unwrap_err();

    assert_eq!(error, Error::CannotJoinAfterHostHasStarted);
  }
}

#[rstest]
#[case::two_different_roles(vec![("we-the-best", TeamRole::Spy), ("A", TeamRole::Minelayer)])]
#[case::three_different_roles(vec![("hydrogen-2-oxygen-1", TeamRole::Spy), ("A", TeamRole::Minelayer), ("0192031923", TeamRole::Cloaker)])]
#[case::many_of_different_roles(vec![("team::0", TeamRole::Spy), ("team::1", TeamRole::Minelayer), ("team::2", TeamRole::Cloaker), ("team::3", TeamRole::Spy), ("team::4", TeamRole::Minelayer), ("team::5", TeamRole::Cloaker), ("team::6", TeamRole::Spy), ("team::7", TeamRole::Minelayer), ("team::8", TeamRole::Cloaker), ("team::9", TeamRole::Spy)])]
#[tokio::test]
async fn test_multiple_players_should_all_be_able_to_join_the_same_game(#[case] teams: Vec<(&str, TeamRole)>) {
  let TestSetup { games, game_id, added } = setup_with_players(teams.iter()).await.unwrap();
  let response = games.try_query_game(QueryGameRequest { game_id }).await.unwrap();
  let game = response.game;
  let received_teams = &game.teams;

  assert_eq!(received_teams.len(), teams.len());

  for ((display_name, _), (team_id, team_key)) in teams.iter().zip(added) {
    let team = game.teams.iter().find(|t| t.id == team_id).unwrap();
    assert_eq!(team.display_name.as_str(), *display_name);
    assert_eq!(team.id, team_id);
    assert_eq!(game.id, game_id);
    assert!(!team.role_used);
    assert_eq!(team.time_of_last_command, None);
    assert_eq!(team.key, team_key);
  }
}
