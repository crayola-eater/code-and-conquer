use crate::types::TeamRole;

#[derive(Debug)]
pub struct CreateAndJoinRequest {
  pub display_name: String,
  pub team_role: TeamRole,
}

#[derive(Debug)]
pub struct CreateAndJoinResponse {
  pub game_id: i32,
  pub team_id: i32,
  pub team_key: String,
}
