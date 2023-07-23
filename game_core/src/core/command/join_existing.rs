use crate::types::TeamRole;

#[derive(Debug)]
pub struct JoinExistingRequest {
  pub game_id: i32,
  pub display_name: String,
  pub team_role: TeamRole,
}

#[derive(Debug)]
pub struct JoinExistingResponse {
  pub team_id: i32,
  pub team_key: String,
}
