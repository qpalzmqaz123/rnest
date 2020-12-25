use crate::{Di, Result, ScopedDi};

pub trait Module {
    fn import(di: &mut Di);
    fn scoped_di(di: &mut Di) -> ScopedDi;
    fn configure_actix_web(di: &mut Di, cfg: &mut actix_web::web::ServiceConfig);
}

pub trait Provider<T> {
    fn register(scoped_di: &mut ScopedDi) -> Result<T>;
}

pub trait Controller<T, I: Clone>: Provider<T> {
    fn configure_actix_web(instance: I, cfg: &mut actix_web::web::ServiceConfig);
}
