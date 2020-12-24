use rnest::{controller, Module, Provider};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Provider)]
struct HelloController {}

#[controller("/")]
impl HelloController {
    #[get("/hello/{name}")]
    fn hello(&self, #[param] name: String) -> String {
        format!("Hello {}", name)
    }
}

#[derive(Module)]
#[controllers(
    HelloController as Arc<RwLock<HelloController>>,
)]
struct HelloModule {}

#[rnest::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    rnest::new!(HelloModule).bind("0.0.0.0:8080")?.run().await?;

    Ok(())
}
