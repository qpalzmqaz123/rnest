mod api;
mod rest_api;
mod user;

use rest_api::RestApiModule;
use rnest::{controller, HttpResponse, Module, Provider};
use user::UserModule;

#[derive(Module)]
#[imports(RestApiModule)]
#[controllers(SpecController)]
struct MainModule {}

#[derive(Provider)]
#[on_module_init(init)]
struct SpecController {
    #[default("".to_string())]
    spec: String,
}

#[controller("/spec")]
impl SpecController {
    #[get("/")]
    fn get(&self) -> HttpResponse {
        HttpResponse::Ok()
            .insert_header(("content-type", "application/json"))
            .body(self.spec.clone())
    }

    async fn init(&mut self) -> Result<(), String> {
        self.spec = rnest::openapi_builder!(MainModule)
            .version("1.0.0")
            .title("Example")
            .add_bearer_auth("bearerAuth")
            .build()
            .to_string();

        Ok(())
    }
}

#[rnest::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    rnest::new!(MainModule, app => {
        let json_cfg = rnest::actix_web::web::JsonConfig::default().limit(1024 * 1024);
        app
            .app_data(json_cfg)
            .wrap(rnest::actix_web::middleware::NormalizePath::new(rnest::actix_web::middleware::TrailingSlash::Always))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;

    Ok(())
}
