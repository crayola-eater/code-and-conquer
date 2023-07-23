use game_core::types::{
  AttackRequest, Error, GameStatus, JoinExistingRequest, QueryGameRequest, QueryGameResponse, QueryGridSquareRequest,
  SenderDetails, StartRequest, TeamRole,
};
use rstest::*;
use tests_integration::{setup_with_players, TestSetup};

#[rstest]
#[case::one_of_each_role(&[("bravo-rapido-0x050fbcA", TeamRole::Spy), ("THE BOSSES 📋", TeamRole::Minelayer), ("KR_1", TeamRole::Cloaker)])]
#[tokio::test]
async fn test_three_player_game(#[case] teams: &[(&str, TeamRole)]) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(teams).await.unwrap();

  let (team_1_id, team_1_key) = added[0].to_owned();
  let (team_2_id, team_2_key) = added[1].to_owned();
  let (team_3_id, team_3_key) = added[2].to_owned();

  let (team_1_display_name, team_1_role) = teams[0];
  let (team_2_display_name, team_2_role) = teams[1];
  let (team_3_display_name, team_3_role) = teams[2];

  // act: query game
  // assert success
  // assert initial game state
  // assert that game contains team
  let created_at = {
    let QueryGameResponse { game } = games.try_query_game(QueryGameRequest { game_id }).await.unwrap();
    assert_eq!(game.status, GameStatus::WaitingForRegistrations);

    for (team_id, team_display_name, team_role) in [
      (team_1_id, team_1_display_name, team_1_role),
      (team_2_id, team_2_display_name, team_2_role),
      (team_3_id, team_3_display_name, team_3_role),
    ] {
      let team = game.teams.iter().find(|t| t.id == team_id).unwrap();
      assert_eq!(team.display_name, team_display_name);
      assert_eq!(team.role, team_role);
      assert_eq!(team.id, team_id);
      assert_eq!(game.id, game_id);
      assert!(!team.role_used);
      assert_eq!(team.time_of_last_command, None);
    }

    game.created_at
  };

  // act: team tries to rejoin existing game with { same name, any role }
  // assert failure (display name already exists)
  for display_name in [team_1_display_name, team_2_display_name, team_3_display_name] {
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

  // {t2, t3} try to start game
  // assert failure
  for (team_id, team_key) in [(team_2_id, team_2_key), (team_3_id, team_3_key)] {
    let sender = SenderDetails { team_id, team_key };
    let error = games.try_start(StartRequest { game_id, sender }).await.unwrap_err();
    assert_eq!(error, Error::OnlyHostCanStartGame { team_id });
  }

  // t1 try to start game
  // assert success
  {
    let sender = SenderDetails {
      team_id: team_1_id,
      team_key: team_1_key.clone(),
    };

    let _ = games.try_start(StartRequest { game_id, sender }).await.unwrap();
  }

  // assert game state has changed
  {
    let game = games.try_query_game(QueryGameRequest { game_id }).await.unwrap().game;
    assert_eq!(game.status, GameStatus::Started);
    assert_eq!(game.created_at, created_at);
  }

  // existing and new teams try to join after game has been started
  // assert failure
  for (display_name, raw_team_role) in [
    ("too_late", TeamRole::Spy),
    (team_1_display_name, team_1_role),
    (team_2_display_name, team_2_role),
    (team_3_display_name, team_3_role),
  ] {
    let error = games
      .try_join_an_existing_game(JoinExistingRequest {
        game_id,
        display_name: display_name.to_string(),
        team_role: raw_team_role,
      })
      .await
      .unwrap_err();
    assert_eq!(error, Error::CannotJoinAfterHostHasStarted);
  }

  // t1 tries to attack a square 50 times
  // assert success for first 30 times
  for i in 1..=50 {
    let sender = SenderDetails {
      team_id: team_1_id,
      team_key: team_1_key.clone(),
    };
    let row = 0;
    let column = 0;

    let attack = AttackRequest {
      game_id,
      sender,
      row_index: row,
      column_index: column,
    };

    if i <= 30 {
      let response = games.try_attack_a_square(attack).await.unwrap();
      assert!(!response.conquered);
      assert_eq!(response.square.row, row);
      assert_eq!(response.square.column, column);
      assert_eq!(response.square.health, 60 - i);
      assert_eq!(response.requests_left, 30 - i);
    } else {
      let error = games.try_attack_a_square(attack).await.unwrap_err();
      assert_eq!(error, Error::NoMoreRequestsLeft);

      let response = games
        .try_query_grid_square(QueryGridSquareRequest {
          game_id,
          row_index: row,
          column_index: column,
        })
        .await
        .unwrap();
      assert_eq!(response.square.health, 30);
    }
  }

  // // p1 joins as spy
  // // p2 joins as minelayer
  // // test commands:
  // //    attack, defend, query_grid_square, query_grid, place_mine (p1 can't, p2 can)
}
