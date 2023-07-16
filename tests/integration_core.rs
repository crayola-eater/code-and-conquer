use code_and_conquer::types::{
  AttackRequest, Command, CommandResponse, Error, GameStatus, JoinExistingRequest, QueryGameRequest, QueryGridSquareRequest,
  SenderDetails, StartRequest, TeamRole,
};
use rstest::*;
use speculoos::prelude::*;
mod test_helpers;

use test_helpers::{setup_with_players, TestSetup};

#[rstest]
#[tokio::test]
async fn test_cannot_join_with_duplicate_display_names(
  #[values("Team Copycats", "Team-12", "_", "you")] display_name: &str,
  #[values(TeamRole::Spy, TeamRole::Minelayer, TeamRole::Cloaker)] team_role: TeamRole,
) {
  let TestSetup { mut games, game_id, .. } = setup_with_players(&[(display_name, team_role)]).await.unwrap();

  for role in [TeamRole::Spy, TeamRole::Minelayer, TeamRole::Cloaker] {
    let command = Command::JoinExisting(JoinExistingRequest {
      display_name: display_name.to_string(),
      team_role: role,
      game_id,
    });

    let error = games.try_process_command(command).await.unwrap_err();
    assert_that!(error).is_equal_to(Error::TeamDisplayNameAlreadyTaken)
  }
}

#[rstest]
#[case::two_different_roles(vec![("we-the-best", TeamRole::Spy), ("A", TeamRole::Minelayer)])]
#[case::three_different_roles(vec![("hydrogen-2-oxygen-1", TeamRole::Spy), ("A", TeamRole::Minelayer), ("0192031923", TeamRole::Cloaker)])]
#[case::many_of_different_roles(vec![("team::0", TeamRole::Spy), ("team::1", TeamRole::Minelayer), ("team::2", TeamRole::Cloaker), ("team::3", TeamRole::Spy), ("team::4", TeamRole::Minelayer), ("team::5", TeamRole::Cloaker), ("team::6", TeamRole::Spy), ("team::7", TeamRole::Minelayer), ("team::8", TeamRole::Cloaker), ("team::9", TeamRole::Spy)])]
#[tokio::test]

async fn test_multiple_players_can_all_join_the_same_game(#[case] teams: Vec<(&str, TeamRole)>) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = test_helpers::setup_with_players(teams.iter()).await.unwrap();

  let command = Command::QueryGame(QueryGameRequest { game_id });

  let game = match games.try_process_command(command).await.unwrap() {
    CommandResponse::QueryGame(response) => response.game,
    unexpected => unreachable!("{unexpected:?}"),
  };

  let received_teams = &game.teams;

  assert_that!(received_teams.len()).is_equal_to(teams.len());

  for ((display_name, _), (team_id, team_key)) in teams.iter().zip(added) {
    let team = game.teams.iter().find(|t| t.id == team_id).unwrap();
    assert_that!(team.display_name.as_str()).is_equal_to(display_name);
    assert_that!(team.id).is_equal_to(team_id);
    assert_that!(game.id).is_equal_to(game_id);
    assert_that!(team.role_used).is_equal_to(false);
    assert_that!(team.time_of_last_command).is_none();
    assert_that!(team.key).is_equal_to(team_key);
  }
}

#[rstest]
#[case::one_of_each_role(&[("bravo-rapido-0x050fbcA", TeamRole::Spy), ("THE BOSSES 📋", TeamRole::Minelayer), ("KR_1", TeamRole::Cloaker)])]
#[tokio::test]
async fn test_three_player_game(#[case] teams: &[(&str, TeamRole)]) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = test_helpers::setup_with_players(teams).await.unwrap();

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
    let command = Command::QueryGame(QueryGameRequest { game_id });

    let game = match games.try_process_command(command).await.unwrap() {
      CommandResponse::QueryGame(response) => response.game,
      unexpected => unreachable!("{unexpected:?}"),
    };

    assert_that!(game.status).is_equal_to(GameStatus::WaitingForRegistrations);

    for (team_id, team_display_name, team_role) in [
      (team_1_id, team_1_display_name, team_1_role),
      (team_2_id, team_2_display_name, team_2_role),
      (team_3_id, team_3_display_name, team_3_role),
    ] {
      let team = game.teams.iter().find(|t| t.id == team_id).unwrap();
      assert_that!(team.display_name.as_str()).is_equal_to(team_display_name);
      assert_that!(team.id).is_equal_to(team_id);
      assert_that!(team.role).is_equal_to(team_role);
      assert_that!(game.id).is_equal_to(game_id);
      assert_that!(team.role_used).is_equal_to(false);
      assert_that!(team.time_of_last_command).is_none();
    }

    game.created_at
  };

  // act: team tries to rejoin existing game with { same name, any role }
  // assert failure (display name already exists)
  for display_name in [team_1_display_name, team_2_display_name, team_3_display_name] {
    for role in [TeamRole::Spy, TeamRole::Minelayer, TeamRole::Cloaker] {
      let command = Command::JoinExisting(JoinExistingRequest {
        display_name: display_name.to_string(),
        team_role: role,
        game_id,
      });

      let error = games.try_process_command(command).await.unwrap_err();
      assert_that!(error).is_equal_to(Error::TeamDisplayNameAlreadyTaken)
    }
  }

  // {t2, t3} try to start game
  // assert failure
  for (team_id, team_key) in [(team_2_id, team_2_key), (team_3_id, team_3_key)] {
    let sender = SenderDetails { team_id, team_key };
    let command = Command::Start(StartRequest { game_id, sender });
    let error = games.try_process_command(command).await.unwrap_err();
    assert_that!(error).is_equal_to(Error::OnlyHostCanStartGame { team_id });
  }

  // t1 try to start game
  // assert success
  {
    let sender = SenderDetails {
      team_id: team_1_id,
      team_key: team_1_key.clone(),
    };
    let command = Command::Start(StartRequest { game_id, sender });

    let _ = match games.try_process_command(command).await.unwrap() {
      CommandResponse::Start(response) => response,
      unexpected => unreachable!("{unexpected:?}"),
    };
  }

  // assert game state has changed
  {
    let command = Command::QueryGame(QueryGameRequest { game_id });
    let game = match games.try_process_command(command).await.unwrap() {
      CommandResponse::QueryGame(response) => response.game,
      unexpected => unreachable!("{unexpected:?}"),
    };

    assert_that!(game.status).is_equal_to(GameStatus::Started);
    assert_that!(game.created_at).is_equal_to(created_at);
  }

  // existing and new teams try to join after game has been started
  // assert failure
  for (display_name, raw_team_role) in [
    ("too_late", TeamRole::Spy),
    (team_1_display_name, team_1_role),
    (team_2_display_name, team_2_role),
    (team_3_display_name, team_3_role),
  ] {
    let command = Command::JoinExisting(JoinExistingRequest {
      game_id,
      display_name: display_name.to_string(),
      team_role: raw_team_role,
    });
    let error = games.try_process_command(command).await.unwrap_err();
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
    let command = Command::Attack(AttackRequest {
      game_id,
      sender,
      row_index: row,
      column_index: column,
    });

    if i <= 30 {
      let response = match games.try_process_command(command).await.unwrap() {
        CommandResponse::Attack(response) => response,
        unexpected => unreachable!("{unexpected:?}"),
      };

      assert_that!(response.conquered).is_false();
      assert_that!(response.square.row).is_equal_to(row);
      assert_that!(response.square.column).is_equal_to(column);
      assert_that!(response.square.health).is_equal_to(60 - i);
      assert_that!(response.requests_left).is_equal_to(30 - i);
    } else {
      let error = games.try_process_command(command).await.unwrap_err();
      assert_that!(error).is_equal_to(Error::NoMoreRequestsLeft);

      let command = Command::QueryGridSquare(QueryGridSquareRequest {
        game_id,
        row_index: row,
        column_index: column,
      });
      let response = match games.try_process_command(command).await.unwrap() {
        CommandResponse::QueryGridSquare(response) => response,
        unexpected => unreachable!("{unexpected:?}"),
      };
      assert_that!(response.square.health).is_equal_to(30);
    }
  }

  // // p1 joins as spy
  // // p2 joins as minelayer
  // // test commands:
  // //    attack, defend, query_grid_square, query_grid, place_mine (p1 can't, p2 can)
}
