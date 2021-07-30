mod module;
mod openapi;

pub use actix_web::{
    self,
    web::{Json, Query},
    HttpRequest, HttpResponse,
};
pub use module::{Controller, Module, Provider};
pub use openapi::{OpenApiBuilder, OpenApiSchema};
pub use rnest_di::{Di, ScopedDi};
pub use rnest_error::{Error, Result};
pub use rnest_macros::{controller, main, Module, Provider};
pub use serde_json::{json, Value as JsonValue};

#[macro_export]
macro_rules! new {
    ($main_module:ident) => {{
        $crate::new!($main_module, app => { app })
    }};
    ($main_module:ident, $app:ident => $cb:block) => {{
        log::trace!("Create di");
        let mut di = rnest::Di::new();

        // Import
        <$main_module as rnest::Module>::import(&mut di);
        log::trace!("Di: {}", di);

        // Create http server
        let di = std::sync::Arc::new(std::sync::Mutex::new(di));
        actix_web::HttpServer::new(move || {
            let app = actix_web::App::new();
            let app = (|$app: rnest::actix_web::App<_, _>| $cb)(app);
            let app = app.configure(|cfg| {
                <$main_module as rnest::Module>::configure_actix_web(
                    &mut di.clone().lock().expect("Lock di error"),
                    cfg,
                );
            });

            app
        })
    }};
}

#[macro_export]
macro_rules! openapi_builder {
    ($main_module:ident) => {{
        let mut cache: std::collections::HashMap<String, rnest::JsonValue> =
            std::collections::HashMap::new();
        $main_module::__rnest_gen_openapi3_spec(&mut cache);
        let paths = cache.into_iter().fold(rnest::json!({}), |mut obj, (_, v)| {
            obj.as_object_mut()
                .unwrap()
                .extend(v.as_object().unwrap().clone());
            obj
        });
        $crate::OpenApiBuilder::new(paths)
    }};
}
