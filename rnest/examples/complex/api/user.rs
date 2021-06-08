use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]

pub struct UserInfo {
    pub id: u32,
    pub name: String,
}

#[async_trait::async_trait]
pub trait User {
    async fn get_list(&self) -> Vec<UserInfo>;
}
