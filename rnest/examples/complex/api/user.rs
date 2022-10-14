use rnest::OpenApiSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, OpenApiSchema)]
pub struct UserInfo {
    #[openapi(description = "User id")]
    pub id: u32,

    #[openapi(description = "User name")]
    pub name: String,
}

#[async_trait::async_trait]
pub trait User: Sync + Send {
    async fn get_list(&self) -> Vec<UserInfo>;
}
