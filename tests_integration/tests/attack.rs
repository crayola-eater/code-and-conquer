use game_core::types::{AttackRequest, AttackResponse, Error, GameStatus, SenderDetails, TeamRole};
use rstest::*;
use tests_integration::{setup_with_players, start_game, TestSetup};

#[rstest]
#[case::one_player(&[("1", TeamRole::Spy)])]
#[case::three_players_different_roles(&[
  ("()0", TeamRole::Cloaker),
  ("iroN", TeamRole::Minelayer),
  ("smarties-🤓", TeamRole::Spy),
])]
#[tokio::test]
async fn test_should_not_be_able_to_attack_unless_game_has_started(#[case] teams: &[(&str, TeamRole)]) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(teams).await.unwrap();

  for (j, (team_id, team_key)) in added.iter().enumerate() {
    for _ in 0..30 {
      let (row, column) = (0, j as i32);
      let sender = SenderDetails {
        team_id: *team_id,
        team_key: team_key.clone(),
      };

      let error = games
        .try_attack_a_square(AttackRequest {
          row_index: row,
          column_index: column,
          game_id,
          sender,
        })
        .await
        .unwrap_err();

      assert!(matches!(
        error,
        Error::InvalidGameStatus {
          current: GameStatus::WaitingForRegistrations,
          required: GameStatus::Started,
          ..
        }
      ));
    }
  }
}

#[rstest]
#[case::one_player(&[("", TeamRole::Spy)])]
#[case::four_players_different_roles(&[
  ("___!Team_1", TeamRole::Minelayer),
  ("team_2", TeamRole::Spy),
  ("smarties-🤓", TeamRole::Cloaker),
  ("transparent%%", TeamRole::Cloaker),
])]
#[tokio::test]
async fn test_should_be_able_to_attack_different_squares_without_conquering(#[case] teams: &[(&str, TeamRole)]) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(teams).await.unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  for (i, (team_id, team_key)) in added.iter().enumerate() {
    for j in 0..30 {
      let (row, column) = (0, i as i32);
      let sender = SenderDetails {
        team_id: *team_id,
        team_key: team_key.clone(),
      };

      let AttackResponse {
        square,
        conquered,
        requests_left,
      } = games
        .try_attack_a_square(AttackRequest {
          row_index: row,
          column_index: column,
          game_id,
          sender,
        })
        .await
        .unwrap();

      assert!(!conquered);
      assert_eq!(requests_left, 29 - j);
      assert_eq!(square.row, row);
      assert_eq!(square.column, column);
      assert!(square.mine.is_none());
      assert_eq!(square.owner_id, None);
      assert_eq!(square.health, 59 - j)
    }
  }
}

#[rstest]
#[case::two_players(&[("t1", TeamRole::Spy), ("t3", TeamRole::Spy)])]
#[case::four_players_different_roles(&[
  ("f", TeamRole::Minelayer),
  ("a", TeamRole::Spy),
  ("C-🤓", TeamRole::Cloaker),
  ("t?", TeamRole::Cloaker),
])]
#[tokio::test]
async fn test_should_be_able_to_conquer_a_square(
  #[case] teams: &[(&str, TeamRole)],
  #[values((0, 0), (2, 4), (0, 3), (1, 2), (3, 4), (4, 0))] coordinates: (i32, i32),
) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(teams).await.unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let mut iter = added
    .iter()
    .flat_map(|(team_id, team_key)| (0..30).map(move |j| (29 - j, *team_id, team_key.clone())))
    .enumerate();

  let (row, column) = coordinates;

  for (i, (expected_requests_left, team_id, team_key)) in iter.by_ref().take(59) {
    let sender = SenderDetails { team_id, team_key };

    let AttackResponse {
      square,
      conquered,
      requests_left,
    } = games
      .try_attack_a_square(AttackRequest {
        row_index: row,
        column_index: column,
        game_id,
        sender,
      })
      .await
      .unwrap();

    assert!(!conquered);
    assert_eq!(requests_left, expected_requests_left);
    assert_eq!(square.row, row);
    assert_eq!(square.column, column);
    assert!(square.mine.is_none());
    assert_eq!(square.owner_id, None);
    let expected_health = 59 - i as i32;
    assert_eq!(square.health, expected_health);
  }

  {
    let (_, (expected_requests_left, team_id, team_key)) = iter.next().unwrap();

    let sender = SenderDetails { team_id, team_key };
    let AttackResponse {
      square,
      conquered,
      requests_left,
    } = games
      .try_attack_a_square(AttackRequest {
        row_index: row,
        column_index: column,
        game_id,
        sender,
      })
      .await
      .unwrap();

    assert!(conquered);
    assert_eq!(requests_left, expected_requests_left);
    assert_eq!(square.row, row);
    assert_eq!(square.column, column);
    assert!(square.mine.is_none());
    assert_eq!(square.owner_id, Some(team_id));
    assert_eq!(square.health, 120);
  }
}
