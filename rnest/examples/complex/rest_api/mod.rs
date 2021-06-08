mod user_controller;

use crate::UserModule;
use rnest::Module;
use user_controller::UserController;

#[derive(Module)]
#[imports(UserModule)]
#[controllers(UserController)]
pub struct RestApiModule {}
