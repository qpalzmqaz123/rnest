use crate::api::{User, UserInfo};
use rnest::{controller, HttpResponse, OpenApiSchema, Provider};
use std::sync::Arc;

#[derive(Provider)]
pub struct UserController {
    user: Arc<dyn User>,
}

#[controller("/user")]
impl UserController {
    #[get("/")]
    #[openapi_schema(get_list_schema)]
    async fn get_list(&self) -> HttpResponse {
        HttpResponse::Ok().json(self.user.get_list().await)
    }

    fn get_list_schema() -> rnest::JsonValue {
        rnest::json!({
            "security": [
                {
                    "bearerAuth": []
                }
            ],
            "tags": [
                "user"
            ],
            "summary": "Get user list",
            "responses": {
                "200": {
                    "description": "ok",
                    "content": {
                        "application/json": {
                            "schema": UserInfo::get_schema()
                        }
                    }
                }
            }
        })
    }
}
