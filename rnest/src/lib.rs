mod module;

pub use actix_web::{
    self,
    web::{Json, Query},
    HttpResponse,
};
pub use rnest_di::{Di, ScopedDi};
pub use rnest_error::{Error, Result};
pub use rnest_macros::{controller, main, Module, Provider};

pub use module::{Controller, Module, Provider};

#[macro_export]
macro_rules! new {
    ($main_module:ident) => {{
        log::trace!("Create di");
        let mut di = rnest::Di::new();

        // Import
        <$main_module as rnest::Module>::import(&mut di);
        log::trace!("Di: {}", di);

        // Create http server
        let di = std::sync::Arc::new(std::sync::Mutex::new(di));
        actix_web::HttpServer::new(move || {
            let app = actix_web::App::new()
                .wrap(actix_web::middleware::NormalizePath::default())
                .service(<$main_module as rnest::Module>::scope(
                    &mut di.clone().lock().unwrap(),
                ));

            app
        })
    }};
}
