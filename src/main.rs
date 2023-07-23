use game_core::core::games;
use game_core::types::{Command, CreateAndJoinRequest, Games, TeamRole};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect("postgres://postgres:password@localhost:5432/postgres")
    .await?;

  games::setup_database(&pool).await?;

  let mut games = Games::try_new(pool).await?;

  let command = CreateAndJoinRequest {
    display_name: "Yes".into(),
    team_role: TeamRole::Minelayer,
  };

  let created = games.try_process_command(Command::CreateAndJoin(command)).await?;

  println!("{created:#?}");

  Ok(())
}
