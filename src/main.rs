use game_core::games;
use game_core::types::{CreateAndJoinRequest, Games, TeamRole};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect("postgres://postgres:password@localhost:5432/postgres")
    .await?;

  games::setup_database(&pool).await?;

  let mut games = Games::try_new(pool).await?;

  let created = games
    .try_create_and_join_a_game(CreateAndJoinRequest {
      display_name: "Yes".into(),
      team_role: TeamRole::Minelayer,
    })
    .await?;

  println!("{created:#?}");

  Ok(())
}
