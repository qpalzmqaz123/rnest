use crate::{Di, Result, ScopedDi};

pub trait Module {
    fn import(di: &mut Di);
    fn scoped_di(di: &mut Di) -> ScopedDi;
    fn scope(di: &mut Di) -> actix_web::Scope;
}

pub trait Provider<T> {
    fn register(scoped_di: &mut ScopedDi) -> Result<T>;
}

pub trait Controller<T, I: Clone>: Provider<T> {
    fn scope(instance: I) -> actix_web::Scope;
}
