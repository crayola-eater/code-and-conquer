use rstest::*;

mod test_helpers;
use test_helpers::{setup_with_players, start_game, TestSetup};

use code_and_conquer::types::{
  AttackRequest, AttackResponse, Command, CommandResponse, DefendRequest, DefendResponse, Error, GameStatus, QueryGameRequest,
  SenderDetails, TeamRole,
};

#[rstest]
#[tokio::test]
async fn test_should_be_able_to_defend_an_unattacked_square(
  #[values((0, 0), (1, 2), (3, 4), (1, 3), (4, 1))] coordinates: (i32, i32),
) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(&[("a", TeamRole::Spy), ("b", TeamRole::Minelayer), ("c", TeamRole::Cloaker)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let defend = DefendRequest {
    game_id,
    row_index: coordinates.0,
    column_index: coordinates.1,
    sender,
  };

  let command = Command::Defend(defend);

  let DefendResponse { square, requests_left } = match games.try_process_command(command).await.unwrap() {
    CommandResponse::Defend(response) => response,
    unexpected => unreachable!("{unexpected:?}"),
  };

  assert_eq!(square.game_id, game_id);
  assert_eq!(square.row, coordinates.0);
  assert_eq!(square.column, coordinates.1);
  assert_eq!(square.owner_id, None);
  assert_eq!(requests_left, 29);

  let response = {
    let query = QueryGameRequest { game_id };

    let command = Command::QueryGame(query);

    match games.try_process_command(command).await.unwrap() {
      CommandResponse::QueryGame(response) => response,
      unexpected => unreachable!("{unexpected:?}"),
    }
  };

  let team = response.game.teams.iter().find(|team| team.id == added[0].0).unwrap();

  let elapsed = (chrono::Utc::now() - team.time_of_last_command.unwrap())
    .to_std()
    .unwrap()
    .as_millis();

  assert_eq!(team.id, added[0].0);
  assert_eq!(team.key, added[0].1);
  assert!(!team.role_used);
  assert_eq!(team.requests_left, 29);
  assert!(elapsed < 1_000, "elapsed {elapsed:?}");
}

#[rstest]
#[tokio::test]
async fn test_should_be_able_to_defend_an_attacked_square(
  #[values((0, 0), (1, 2), (3, 4), (1, 3), (4, 1))] coordinates: (i32, i32),
) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(&[("a", TeamRole::Spy), ("b", TeamRole::Minelayer), ("c", TeamRole::Cloaker)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let attack = AttackRequest {
    game_id,
    row_index: coordinates.0,
    column_index: coordinates.1,
    sender,
  };

  let command = Command::Attack(attack);

  let AttackResponse {
    conquered,
    requests_left,
    square,
  } = match games.try_process_command(command).await.unwrap() {
    CommandResponse::Attack(response) => response,
    unexpected => unreachable!("{unexpected:?}"),
  };

  assert!(!conquered);
  assert_eq!(requests_left, 29);
  assert_eq!(square.health, 59);
  assert_eq!(square.owner_id, None);
  assert_eq!(square.game_id, game_id);
  assert_eq!(square.row, coordinates.0);
  assert_eq!(square.column, coordinates.1);

  let sender = SenderDetails {
    team_id: added[1].0,
    team_key: added[1].1.clone(),
  };

  let defend = DefendRequest {
    game_id,
    row_index: coordinates.0,
    column_index: coordinates.1,
    sender,
  };

  let command = Command::Defend(defend);

  let DefendResponse { square, requests_left } = match games.try_process_command(command).await.unwrap() {
    CommandResponse::Defend(response) => response,
    unexpected => unreachable!("{unexpected:?}"),
  };

  assert_eq!(square.game_id, game_id);
  assert_eq!(square.row, coordinates.0);
  assert_eq!(square.column, coordinates.1);
  assert_eq!(square.owner_id, None);
  assert_eq!(requests_left, 29)
}

#[rstest]
#[tokio::test]

async fn test_should_not_be_able_to_defend_square_when_game_has_not_started(
  #[values((0, 0), (1, 2), (3, 4), (1, 3), (4, 1))] coordinates: (i32, i32),
) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(&[("a", TeamRole::Spy), ("b", TeamRole::Minelayer), ("c", TeamRole::Cloaker)])
    .await
    .unwrap();

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let defend = DefendRequest {
    game_id,
    row_index: coordinates.0,
    column_index: coordinates.1,
    sender,
  };

  let command = Command::Defend(defend);

  let error = games.try_process_command(command).await.unwrap_err();

  assert_eq!(
    error,
    Error::InvalidGameStatus {
      current: GameStatus::WaitingForRegistrations,
      required: GameStatus::Started,
      action: "defend square"
    }
  );
}

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_defend_square_when_game_id_is_invalid(
  #[values(-1, 10, -555, 0)] invalid_game_id: i32,
  #[values((0, 0), (1, 2), (3, 4), (1, 3), (4, 1))] coordinates: (i32, i32),
) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(&[("a", TeamRole::Spy), ("b", TeamRole::Minelayer), ("c", TeamRole::Cloaker)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let defend = DefendRequest {
    game_id: invalid_game_id,
    row_index: coordinates.0,
    column_index: coordinates.1,
    sender,
  };

  let command = Command::Defend(defend);

  let error = games.try_process_command(command).await.unwrap_err();

  assert_eq!(
    error,
    Error::InvalidGameId {
      game_id: invalid_game_id
    }
  );
}

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_defend_square_when_team_id_is_invalid(
  #[values("a", "b", "c", "d")] invalid_team_key: String,
  #[values((0, 0), (1, 2), (3, 4), (1, 3), (4, 1))] coordinates: (i32, i32),
) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(&[("a", TeamRole::Spy), ("b", TeamRole::Minelayer), ("c", TeamRole::Cloaker)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: invalid_team_key,
  };

  let defend = DefendRequest {
    game_id,
    row_index: coordinates.0,
    column_index: coordinates.1,
    sender,
  };

  let command = Command::Defend(defend);

  let error = games.try_process_command(command).await.unwrap_err();

  assert_eq!(error, Error::InvalidCredentials);
}

#[rstest]
#[tokio::test]
async fn test_should_not_be_able_to_defend_a_square_when_no_requests_left(
  #[values((0, 0), (1, 2), (3, 4), (1, 3), (4, 1))] coordinates: (i32, i32),
) {
  let TestSetup {
    mut games,
    game_id,
    added,
  } = setup_with_players(&[("a", TeamRole::Spy), ("b", TeamRole::Minelayer), ("c", TeamRole::Cloaker)])
    .await
    .unwrap();

  start_game(&mut games, game_id, added[0].0, added[0].1.clone()).await;

  for i in 0..30 {
    let sender = SenderDetails {
      team_id: added[0].0,
      team_key: added[0].1.clone(),
    };

    let defend = DefendRequest {
      game_id,
      row_index: coordinates.0,
      column_index: coordinates.1,
      sender,
    };

    let command = Command::Defend(defend);

    let DefendResponse { square, requests_left } = match games.try_process_command(command).await.unwrap() {
      CommandResponse::Defend(response) => response,
      unexpected => unreachable!("{unexpected:?}"),
    };

    assert_eq!(square.game_id, game_id);
    assert_eq!(square.row, coordinates.0);
    assert_eq!(square.column, coordinates.1);
    assert_eq!(square.owner_id, None);
    assert_eq!(requests_left, 29 - i)
  }

  let sender = SenderDetails {
    team_id: added[0].0,
    team_key: added[0].1.clone(),
  };

  let defend = DefendRequest {
    game_id,
    row_index: coordinates.0,
    column_index: coordinates.1,
    sender,
  };

  let command = Command::Defend(defend);

  let error = games.try_process_command(command).await.unwrap_err();

  assert_eq!(error, Error::NoMoreRequestsLeft);
}
