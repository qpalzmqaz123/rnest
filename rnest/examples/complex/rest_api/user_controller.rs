use crate::api::{User, UserInfo};
use rnest::{controller, HttpResponse, Json, Provider, ValidatedJson};
use std::sync::Arc;

#[derive(Provider)]
#[on_module_init(init)]
pub struct UserController {
    user: Arc<dyn User>,

    #[default_fn(|user: Arc<dyn User>| async move {
        Result::<usize, String>::Ok(user.get_list().await.len())
    })]
    user_count_on_startup: usize,
}

#[controller("/user")]
impl UserController {
    async fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("user_count_on_startup: {}", self.user_count_on_startup);

        Ok(())
    }

    #[post("/")]
    #[openapi(bearer_auth, tags = ["user"], summary = "Add new user")]
    async fn add(&self, #[body] info: ValidatedJson<UserInfo>) -> HttpResponse {
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
