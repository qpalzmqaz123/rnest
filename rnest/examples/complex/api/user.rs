use rnest::OpenApiSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]

pub struct UserInfo {
    pub id: u32,
    pub name: String,
}

impl OpenApiSchema for UserInfo {
    fn get_schema() -> rnest::serde_json::Value {
        rnest::serde_json::json!({
            "type": "object",
            "properties": {
                "id": {
                    "description": "User id",
                    "type": "string"
                },
                "name": {
                    "description": "User name",
                    "type": "string"
                }
            }
        })
    }
}

#[async_trait::async_trait]
pub trait User {
    async fn get_list(&self) -> Vec<UserInfo>;
}
