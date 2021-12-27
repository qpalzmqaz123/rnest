use crate::{DiGuard, Error, Result, ScopedDi, ScopedValue};

pub struct ScopedDiGuard {
    name: String,
    guard: DiGuard,
    search_seq: Vec<String>, // [current, dep1, dep2, ...]
}

impl ScopedDiGuard {
    pub fn new(name: String, guard: DiGuard, search_seq: Vec<String>) -> Self {
        Self {
            name,
            guard,
            search_seq,
        }
    }

    pub async fn inject<T>(&self, name: &str) -> Result<T>
    where
        T: Clone + Send + 'static,
    {
        for scope in &self.search_seq {
            match self
                .guard
                .inject::<ScopedValue<T>>(&ScopedDi::scoped_name(scope, name))
                .await
            {
                Ok(v) => {
                    if v.export || self.name == v.scope {
                        return Ok(v.v);
                    } else {
                        continue;
                    }
                }
                Err(Error::KeyNotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }

        Err(Error::KeyNotFound(format!(
            "'{}' not found in scope list {:?}",
            name, self.search_seq
        )))
    }

    pub fn inject_value<T>(&self, name: &str) -> Result<T>
    where
        T: Clone + Send + 'static,
    {
        for scope in &self.search_seq {
            match self
                .guard
                .inject_value::<ScopedValue<T>>(&ScopedDi::scoped_name(scope, name))
            {
                Ok(v) => {
                    if v.export || self.name == v.scope {
                        return Ok(v.v);
                    } else {
                        continue;
                    }
                }
                Err(Error::KeyNotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }

        Err(Error::KeyNotFound(format!(
            "Direct '{}' not found in scope list {:?}",
            name, self.search_seq
        )))
    }
}
