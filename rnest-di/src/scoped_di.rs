use futures::Future;

use crate::{Di, Error, Result, ScopedDiGuard};

#[derive(Clone)]
pub struct ScopedValue<T: Clone + Send + 'static> {
    pub scope: String,
    pub v: T,
    pub export: bool,
}

pub struct ScopedDi {
    di: Di,
    name: String,
    search_seq: Vec<String>, // [current, dep1, dep2, ...]
}

impl ScopedDi {
    pub fn new(di: Di, name: &str, deps: &[&str]) -> Self {
        Self {
            di,
            name: name.into(),
            search_seq: deps.iter().fold(vec![name.to_string()], |mut seq, dep| {
                seq.push(dep.to_string());
                seq
            }),
        }
    }

    pub fn register_value<T>(&self, name: &str, value: T, export: bool) -> Result<()>
    where
        T: Clone + Send + 'static,
    {
        Ok(self.di.register_value(
            &Self::scoped_name(&self.name, name),
            ScopedValue {
                scope: self.name.clone(),
                v: value,
                export,
            },
        )?)
    }

    pub fn register_factory<Fn, Fut, T>(&self, name: &str, f: Fn, export: bool) -> Result<()>
    where
        Fn: FnOnce(ScopedDiGuard) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>> + Send,
        T: Clone + Send + 'static,
    {
        let search_seq = self.search_seq.clone();
        let scope = self.name.clone();

        Ok(self.di.register_factory(
            &Self::scoped_name(&self.name, name),
            move |di| async move {
                Ok(ScopedValue {
                    scope: scope.clone(),
                    v: f(ScopedDiGuard::new(scope, di, search_seq)).await?,
                    export,
                })
            },
        )?)
    }

    pub async fn inject<T>(&self, name: &str) -> Result<T>
    where
        T: Clone + Send + 'static,
    {
        for scope in &self.search_seq {
            match self
                .di
                .inject::<ScopedValue<T>>(&Self::scoped_name(scope, name))
                .await
            {
                Ok(v) => {
                    if v.export || self.name == v.scope {
                        return Ok(v.v);
                    } else {
                        return Err(Error::InjectPrivateProvider(name.into()));
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
                .di
                .inject_value::<ScopedValue<T>>(&Self::scoped_name(scope, name))
            {
                Ok(v) => {
                    if v.export || self.name == v.scope {
                        return Ok(v.v);
                    } else {
                        return Err(Error::InjectPrivateProvider(name.into()));
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

    pub fn scoped_name(scope: &str, name: &str) -> String {
        format!("{}#{}", scope, name)
    }
}
