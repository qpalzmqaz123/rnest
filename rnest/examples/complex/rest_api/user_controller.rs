use crate::api::User;
use rnest::{controller, HttpResponse, Provider};
use std::sync::Arc;

#[derive(Provider)]
pub struct UserController {
    user: Arc<dyn User + Sync + Send>,
}

#[controller("/user")]
impl UserController {
    #[get("/")]
    async fn get_list(&self) -> HttpResponse {
        HttpResponse::Ok().json(self.user.get_list().await)
    }
}
