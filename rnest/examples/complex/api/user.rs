use rnest::OpenApiSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, OpenApiSchema, Validate)]
pub struct UserInfo {
    #[openapi(description = "User id")]
    #[validate(range(min = 1, max = 10000))]
    pub id: u32,

    #[openapi(description = "User name")]
    pub name: String,
}

#[async_trait::async_trait]
pub trait User: Sync + Send {
    async fn get_list(&self) -> Vec<UserInfo>;
}
