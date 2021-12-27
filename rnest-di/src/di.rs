use std::{
    any::Any,
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex},
};

use futures::future::BoxFuture;

use crate::{DiGuard, Error, Result, ScopedDi};

#[derive(Clone)]
pub struct Di {
    factory_map: Arc<Mutex<HashMap<String, Box<dyn Any + Send>>>>,
    value_map: Arc<Mutex<HashMap<String, Box<dyn Any + Send>>>>,
}

impl Di {
    pub fn new() -> Self {
        Self {
            factory_map: Arc::new(Mutex::new(HashMap::new())),
            value_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn scope(&self, name: &str, deps: &[&str]) -> ScopedDi {
        ScopedDi::new(self.clone(), name, deps)
    }

    pub fn contains(&self, name: &str) -> Result<bool> {
        Ok(
            self.factory_map.lock()?.contains_key(name)
                || self.value_map.lock()?.contains_key(name),
        )
    }

    pub fn register_value<T>(&self, name: &str, value: T) -> Result<()>
    where
        T: Clone + Send + 'static,
    {
        let name = name.to_string();

        log::trace!("Register value: {}", name);

        // Convert value to any
        let value: Box<dyn Any + Send> = Box::new(value);

        // Save value
        self.value_map.lock()?.insert(name, value);

        Ok(())
    }

    pub fn register_factory<Fn, Fut, T>(&self, name: &str, f: Fn) -> Result<()>
    where
        Fn: FnOnce(DiGuard) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>> + Send,
        T: Clone + Send + 'static,
    {
        let name = name.to_string();

        log::trace!("Register factory: {}", name);

        // Generate factory
        let factory: Box<dyn Send + FnOnce(DiGuard) -> BoxFuture<'static, Result<T>>> =
            Box::new(|di| Box::pin(async { f(di).await }));

        // Convert factory to any
        let factory_any: Box<dyn Any + Send> = Box::new(factory);

        // Save factory
        self.factory_map.lock()?.insert(name, factory_any);

        Ok(())
    }

    pub async fn inject<T>(&self, name: &str) -> Result<T>
    where
        T: Clone + Send + 'static,
    {
        let guard = DiGuard::new(self.clone());

        Ok(guard.inject(name).await?)
    }

    pub fn inject_value<T>(&self, name: &str) -> Result<T>
    where
        T: Clone + Send + 'static,
    {
        let name = name.to_string();

        log::trace!("Try direct inject from value map: {}", name);

        // Lookup from value_map
        if let Some(value_any) = self.value_map.lock()?.get(&name) {
            // Convert any to value
            if let Some(value) = value_any.downcast_ref::<T>() {
                return Ok(value.clone());
            } else {
                return Err(Error::TypeMismatch(format!(
                    "Direct downcast value failed, key: '{}'",
                    name
                )));
            }
        } else {
            return Err(Error::KeyNotFound(format!("Direct key: {}", name,)));
        }
    }

    pub(crate) async fn internal_inject<T>(&self, name: &str, guard: DiGuard) -> Result<T>
    where
        T: Clone + Send + 'static,
    {
        let name = name.to_string();

        log::trace!("Inject: {}", name);

        // Drop lock immediately
        let factory_any = self.factory_map.lock()?.remove(&name);

        let value = if let Some(factory_any) = factory_any {
            log::trace!("Try inject from factory map: {}", name);

            // Convert any to factory
            let factory = factory_any
                .downcast::<Box<dyn Send + FnOnce(DiGuard) -> BoxFuture<'static, Result<T>>>>()
                .map_err(|_| {
                    Error::TypeMismatch(format!("Downcast factory failed, key: '{}'", name))
                })?;

            // Get value
            let value = factory(guard).await?;

            // Copy value and convert to any
            let value_any: Box<dyn Any + Send> = Box::new(value.clone());

            // Save value
            self.value_map.lock()?.insert(name, value_any);

            value
        } else {
            log::trace!("Try inject from value map: {}", name);

            // Lookup from value_map
            if let Some(value_any) = self.value_map.lock()?.get(&name) {
                // Convert any to value
                if let Some(value) = value_any.downcast_ref::<T>() {
                    value.clone()
                } else {
                    return Err(Error::TypeMismatch(format!(
                        "Downcast value failed, key: '{}'",
                        name
                    )));
                }
            } else {
                return Err(Error::KeyNotFound(format!(
                    "key: {}, stack: {:?}",
                    name,
                    guard.inject_stack.lock()?
                )));
            }
        };

        Ok(value)
    }
}

impl std::fmt::Display for Di {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Di {{\n")?;
        if let Ok(map) = self.value_map.lock() {
            for (name, v) in &*map {
                write!(f, "  Value('{}' - {:?})\n", name, v.type_id())?;
            }
        }
        if let Ok(map) = self.factory_map.lock() {
            for (name, v) in &*map {
                write!(f, "  Factory('{}' - {:?})\n", name, v.type_id())?;
            }
        }
        write!(f, "}}\n")?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[tokio::test]
    async fn test_simple() {
        let di = Di::new();

        di.register_factory("a", |_| async { Ok(1i32) }).unwrap();
        di.register_factory(
            "b",
            |di| async move { Ok(di.inject::<i32>("a").await? + 1) },
        )
        .unwrap();
        di.register_value("c", 10i32).unwrap();

        println!("{}", di);

        assert_eq!(di.inject::<i32>("a").await.unwrap(), 1i32);
        assert_eq!(di.inject::<i32>("b").await.unwrap(), 2i32);
        assert_eq!(di.inject::<i32>("c").await.unwrap(), 10i32);
        assert_eq!(di.contains("a").unwrap(), true);
        assert_eq!(di.contains("b").unwrap(), true);
        assert_eq!(di.contains("c").unwrap(), true);
        assert_eq!(di.contains("d").unwrap(), false);
    }

    #[tokio::test]
    async fn test_not_found() {
        let di = Di::new();

        di.register_factory("a", |di| async move { Ok(di.inject::<()>("c").await?) })
            .unwrap();

        match di.inject::<i32>("b").await {
            Err(Error::KeyNotFound(_)) => {}
            r @ _ => panic!("{:?}", r),
        }

        match di.inject::<()>("a").await {
            Err(Error::KeyNotFound(_)) => {}
            r @ _ => panic!("{:?}", r),
        }
    }

    #[tokio::test]
    async fn test_circular_dep() {
        let di = Di::new();

        di.register_factory(
            "a",
            |di| async move { Ok(di.inject::<i32>("b").await? + 1) },
        )
        .unwrap();
        di.register_factory(
            "b",
            |di| async move { Ok(di.inject::<i32>("a").await? + 1) },
        )
        .unwrap();

        match di.inject::<i32>("b").await {
            Err(Error::CircularDependency(_)) => {}
            r @ _ => panic!("{:?}", r),
        }

        match di.inject::<i32>("a").await {
            Err(Error::KeyNotFound(_)) => {}
            r @ _ => panic!("{:?}", r),
        }
    }
}
