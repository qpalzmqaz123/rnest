use crate::api::{User, UserInfo};
use rnest::{controller, HttpResponse, Json, Provider};
use std::sync::Arc;

#[derive(Provider)]
pub struct UserController {
    user: Arc<dyn User>,
}

#[controller("/user")]
impl UserController {
    #[post("/")]
    #[openapi(bearer_auth, tags = ["user"], summary = "Add new user")]
    async fn add(&self, #[body] info: Json<UserInfo>) -> HttpResponse {
        log::info!("TODO: add user: {:?}", info.0);
        HttpResponse::Ok().finish()
    }

    #[delete("/{id}")]
    #[openapi(bearer_auth, tags = ["user"], summary = "Delete user")]
    async fn del(&self, #[param] id: u32) -> HttpResponse {
        log::info!("TODO: delete user: {}", id);
        HttpResponse::Ok().finish()
    }

    #[get("/")]
    #[openapi(bearer_auth, tags = ["user"], summary = "Get user list")]
    async fn get_list(&self) -> Json<Vec<UserInfo>> {
        Json(self.user.get_list().await)
    }
}
