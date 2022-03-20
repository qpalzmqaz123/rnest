use rnest::{controller, HttpRequest, Module, Provider, Query};
use serde::Deserialize;

#[derive(Deserialize)]
struct QueryInfo {
    msg: String,
}

#[derive(Provider)]
struct HelloController {}

#[controller("/")]
impl HelloController {
    #[get("/hello/{name:.*}")]
    fn hello(
        &self,
        #[param] name: String,
        #[query] info: Query<QueryInfo>,
        #[raw] req: HttpRequest,
    ) -> String {
        println!("req: {:?}", req);

        format!("Hello {}: {}", name, info.msg)
    }
}

#[derive(Module)]
#[controllers(HelloController)]
struct HelloModule {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    rnest::new!(HelloModule).bind("0.0.0.0:8080")?.run().await?;

    Ok(())
}
