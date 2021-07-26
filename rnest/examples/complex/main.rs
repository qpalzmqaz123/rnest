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

    rnest::new!(MainModule, app => {
        let json_cfg = rnest::actix_web::web::JsonConfig::default().limit(1024 * 1024);
        app
            .app_data(json_cfg)
            .wrap(rnest::actix_web::middleware::NormalizePath::default())
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;

    Ok(())
}
