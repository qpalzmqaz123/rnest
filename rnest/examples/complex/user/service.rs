use super::UserStore;
use crate::api::{User, UserInfo};
use rnest::Provider;
use std::sync::Arc;

#[derive(Provider)]
#[on_module_init(init)]
pub struct UserService {
    store: Arc<UserStore>,
}

impl UserService {
    fn init(&self) {
        log::info!("init");
    }
}

#[async_trait::async_trait]
impl User for UserService {
    async fn get_list(&self) -> Vec<UserInfo> {
        self.store
            .get_list()
            .await
            .into_iter()
            .map(|(id, name)| UserInfo { id, name })
            .collect()
    }
}
