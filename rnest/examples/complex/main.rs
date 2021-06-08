mod api;
mod rest_api;
mod user;

use rest_api::RestApiModule;
use rnest::Module;
use user::UserModule;

#[derive(Module)]
#[imports(RestApiModule)]
struct MainModule {}

#[rnest::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "trace");
    env_logger::init();

    rnest::new!(MainModule).bind("0.0.0.0:8080")?.run().await?;

    Ok(())
}
