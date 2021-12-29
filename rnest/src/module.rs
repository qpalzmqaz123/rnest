use crate::{Di, Result, ScopedDi, ScopedDiGuard};

#[async_trait::async_trait]
pub trait Module {
    async fn import(di: &mut Di) -> Result<()>;
    fn scoped_di(di: &mut Di) -> ScopedDi;
    fn configure_actix_web(di: &mut Di, cfg: &mut actix_web::web::ServiceConfig) -> Result<()>;
}

#[async_trait::async_trait]
pub trait Provider<T> {
    async fn register(scoped_di: ScopedDiGuard) -> Result<T>;
}

pub trait Controller<T, I: Clone>: Provider<T> {
    fn configure_actix_web(instance: I, cfg: &mut actix_web::web::ServiceConfig);
}
