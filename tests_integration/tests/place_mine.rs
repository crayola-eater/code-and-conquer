use game_core::types::{DefendRequest, Error, GameStatus, PlaceMineRequest, PlaceMineResponse, SenderDetails, TeamRole};
use rstest::*;
use tests_integration::{setup_with_players, start_game, TestSetup};

#[rstest]
#[tokio::test]
async fn test_should_be_able_to_place_a_mine_when_role_is_minelayer() {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(&[("TheTeam1", TeamRole::Minelayer), ("the-other-team2", TeamRole::Cloaker)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let PlaceMineResponse { requests_left, square } = games
    .try_place_a_mine(PlaceMineRequest {
      game_id,
      sender,
      row_index: 2,
      column_index: 1,
    })
    .await
    .unwrap();

  assert_eq!(requests_left, 29);
  assert_eq!(square.game_id, game_id);
  assert_eq!(square.row, 2);
  assert_eq!(square.column, 1);
  assert_eq!(square.health, 60);
  assert_eq!(square.owner_id, None);

  let mine = square.mine.unwrap();

  assert_eq!(mine.placed_by, added[0].0);
  assert_eq!(mine.triggered_by, None)
}

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_place_a_mine_when_game_has_not_started() {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(&[("TheTeam1", TeamRole::Minelayer), ("the-other-team2", TeamRole::Cloaker)])
    .await
    .unwrap();

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let error = games
    .try_place_a_mine(PlaceMineRequest {
      game_id,
      sender,
      row_index: 0,
      column_index: 0,
    })
    .await
    .unwrap_err();

  assert_eq!(
    error,
    Error::InvalidGameStatus {
      current: GameStatus::WaitingForRegistrations,
      required: GameStatus::Started,
      action: "place mine"
    }
  )
}

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_place_a_mine_when_game_id_is_invalid() {
  let TestSetup {
    mut games,
    game_id,
    added,
    ..
  } = setup_with_players(&[("TheTeam1", TeamRole::Minelayer), ("the-other-team2", TeamRole::Cloaker)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let error = games
    .try_place_a_mine(PlaceMineRequest {
      game_id: -1,
      sender,
      row_index: 0,
      column_index: 0,
    })
    .await
    .unwrap_err();

  assert_eq!(error, Error::InvalidGameId { game_id: -1 })
}

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_place_a_mine_when_team_id_is_invalid() {
  let TestSetup {
    mut games,
    game_id,
    added,
    ..
  } = setup_with_players(&[("TheTeam1", TeamRole::Minelayer), ("the-other-team2", TeamRole::Cloaker)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let sender = SenderDetails {
    team_id: 555,
    team_key: added[0].1.clone(),
  };

  let error = games
    .try_place_a_mine(PlaceMineRequest {
      game_id,
      sender,
      row_index: 0,
      column_index: 0,
    })
    .await
    .unwrap_err();

  assert_eq!(error, Error::InvalidCredentials)
}

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_place_a_mine_when_team_key_is_invalid() {
  let TestSetup {
    mut games,
    game_id,
    added,
    ..
  } = setup_with_players(&[("TheTeam1", TeamRole::Minelayer), ("the-other-team2", TeamRole::Cloaker)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: "invalid-team-key-234234023940".to_string(),
  };

  let error = games
    .try_place_a_mine(PlaceMineRequest {
      game_id,
      sender,
      row_index: 0,
      column_index: 0,
    })
    .await
    .unwrap_err();

  assert_eq!(error, Error::InvalidCredentials)
}

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_place_a_mine_when_team_role_is_not_minelayer() {
  let TestSetup {
    mut games,
    game_id,
    added,
    ..
  } = setup_with_players(&[("TheTeam1", TeamRole::Spy), ("the-other-team2", TeamRole::Minelayer)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let error = games
    .try_place_a_mine(PlaceMineRequest {
      game_id,
      sender,
      row_index: 0,
      column_index: 0,
    })
    .await
    .unwrap_err();

  assert_eq!(
    error,
    Error::OnlyMinelayersCanPlaceMines {
      team_role: TeamRole::Spy
    }
  )
}

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_place_a_mine_when_no_requests_left() {
  let TestSetup {
    mut games,
    game_id,
    added,
    ..
  } = setup_with_players(&[("TheTeam1", TeamRole::Minelayer), ("the-other-team2", TeamRole::Cloaker)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  for _ in 0..30 {
    let sender = SenderDetails {
      team_id: added[0].0,
      team_key: added[0].1.clone(),
    };

    games
      .try_defend_a_square(DefendRequest {
        game_id,
        row_index: 0,
        column_index: 1,
        sender,
      })
      .await
      .unwrap();
  }

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let error = games
    .try_place_a_mine(PlaceMineRequest {
      game_id,
      sender,
      row_index: 0,
      column_index: 0,
    })
    .await
    .unwrap_err();

  assert_eq!(error, Error::NoMoreRequestsLeft)
}

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_place_a_mine_more_than_once() {
  let TestSetup {
    mut games,
    game_id,
    added,
    ..
  } = setup_with_players(&[("TheTeam1", TeamRole::Minelayer), ("the-other-team2", TeamRole::Cloaker)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  games
    .try_place_a_mine(PlaceMineRequest {
      game_id,
      sender,
      row_index: 0,
      column_index: 0,
    })
    .await
    .unwrap();

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let error = games
    .try_place_a_mine(PlaceMineRequest {
      game_id,
      sender,
      row_index: 2,
      column_index: 2,
    })
    .await
    .unwrap_err();

  assert_eq!(error, Error::RoleAlreadyUsed)
}

#[rstest]
#[tokio::test]
async fn test_should_be_able_to_place_a_mine_on_top_of_another_teams_mine(#[values((0, 0))] coordinates: (i32, i32)) {
  let TestSetup {
    mut games,
    game_id,
    added,
    ..
  } = setup_with_players(&[("TheTeam1", TeamRole::Minelayer), ("the-other-team2", TeamRole::Minelayer)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let response = games
    .try_place_a_mine(PlaceMineRequest {
      game_id,
      sender,
      row_index: coordinates.0,
      column_index: coordinates.1,
    })
    .await
    .unwrap();

  assert_eq!(response.square.mine.unwrap().placed_by, added[0].0);

  let sender = SenderDetails {
    team_id: added[1].0,
    team_key: added[1].1.clone(),
  };

  let response = games
    .try_place_a_mine(PlaceMineRequest {
      game_id,
      sender,
      row_index: coordinates.0,
      column_index: coordinates.1,
    })
    .await
    .unwrap();

  assert_eq!(response.square.mine.unwrap().placed_by, added[1].0);
}
