use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use crate::{Di, Error, Result};

#[derive(Clone)]
pub struct DiGuard {
    di: Di,
    pub(crate) inject_stack: Arc<Mutex<Vec<String>>>,
    inject_set: Arc<Mutex<HashSet<String>>>,
}

impl DiGuard {
    pub fn new(di: Di) -> Self {
        Self {
            di,
            inject_stack: Arc::new(Mutex::new(Vec::new())),
            inject_set: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn inject<T>(&self, name: &str) -> Result<T>
    where
        T: Clone + Send + 'static,
    {
        if self.inject_set.lock()?.contains(name) {
            return Err(Error::CircularDependency(format!(
                "name: '{}', stack: {:?}",
                name,
                self.inject_stack.lock()?
            )));
        }

        log::trace!("Guard inject: '{}'", name);
        self.inject_stack.lock()?.push(name.into());
        self.inject_set.lock()?.insert(name.into());

        let value = self.di.internal_inject(name, self.clone()).await?;
        log::trace!("Guard inject ok: '{}'", name);

        Ok(value)
    }

    pub fn inject_value<T>(&self, name: &str) -> Result<T>
    where
        T: Clone + Send + 'static,
    {
        log::trace!("Guard direct inject: '{}'", name);
        let value = self.di.inject_value(name)?;
        log::trace!("Guard direct inject ok: '{}'", name);

        Ok(value)
    }
}
