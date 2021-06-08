mod service;
mod store;

use crate::api::User;
use rnest::Module;
use service::UserService;
use std::sync::Arc;
use store::UserStore;

#[derive(Module)]
#[providers(
    UserService as Arc<dyn User + Sync + Send>,
    UserStore as Arc<UserStore>,
)]
#[exports(
    Arc<dyn User + Sync + Send>,
)]
pub struct UserModule {}
