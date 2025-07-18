use crate::models::User;
use shared::types::ApiResponse;

pub fn handle_user_request(user_id: u32) -> ApiResponse<User> {
    let user = User::find(user_id);
    ApiResponse::ok(user)
}