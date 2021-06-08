use rnest_error::{Error, Result};
use std::{any::Any, collections::HashMap};

pub struct Di {
    factory_map: HashMap<String, Box<dyn Any>>,
    instance_map: HashMap<String, Box<dyn Any>>,
    // FIXME: Temporarily use stack to detect circular dependencies
    stack: Vec<String>,
}

unsafe impl Send for Di {}

impl Di {
    pub fn new() -> Self {
        Self {
            factory_map: HashMap::new(),
            instance_map: HashMap::new(),
            stack: Vec::new(),
        }
    }

    pub fn contains<S: Into<String>>(&self, name: S) -> bool {
        let name: String = name.into();
        self.factory_map.contains_key(&name) || self.instance_map.contains_key(&name)
    }

    pub fn register_value<S, T>(&mut self, name: S, value: T)
    where
        S: Into<String>,
        T: 'static + Clone,
    {
        let name = name.into();

        log::trace!("Register value: '{}'", name);

        self.instance_map.insert(name, Box::new(value));
    }

    pub fn register_factory<S, F, T>(&mut self, name: S, factory: F)
    where
        S: Into<String>,
        F: 'static + FnOnce(&mut Di) -> Result<T>,
        T: 'static + Clone,
    {
        let name = name.into();

        log::trace!("Register factory: '{}'", name);

        let factory: Box<dyn FnOnce(&mut Di) -> Result<T>> = Box::new(factory);
        self.factory_map.insert(name, Box::new(factory));
    }

    pub fn inject<S, T>(&mut self, name: S) -> Result<T>
    where
        S: Into<String>,
        T: 'static + Clone,
    {
        let name: String = name.into();

        log::trace!("Try to inject: '{}'", name);

        self.stack.push(name.clone());

        // Check circular dependencies
        if (self.stack[0..self.stack.len() - 1]).contains(&name) {
            return Err(Error::CircularDependencies { name });
        }

        match self.instance_map.get(&name) {
            Some(instance) => {
                let instance = instance
                    .downcast_ref::<T>()
                    .ok_or(Error::TypeMismatch { name: name.clone() })?;
                return Ok(instance.clone());
            }
            None => {
                let v = self
                    .factory_map
                    .remove(&name)
                    .ok_or(Error::FactoryNotFound { name: name.clone() })?;
                let factory = v
                    .downcast::<Box<dyn FnOnce(&mut Di) -> Result<T>>>()
                    .map_err(|_| Error::TypeMismatch { name: name.clone() })?;
                let instance = factory(self)?;

                self.instance_map.insert(name, Box::new(instance.clone()));
                return Ok(instance);
            }
        }
    }

    pub fn scope<'a, S>(&'a mut self, scope: S, import_scopes: &'static [&str]) -> ScopedDi<'a>
    where
        S: Into<String> + AsRef<str>,
    {
        ScopedDi::new(self, scope, import_scopes)
    }
}

impl std::fmt::Display for Di {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Di {{\n")?;
        for (name, v) in &self.instance_map {
            write!(f, "  Instance('{}' - {:?})\n", name, v.type_id())?;
        }
        for (name, v) in &self.factory_map {
            write!(f, "  Factory('{}' - {:?})\n", name, v.type_id())?;
        }
        write!(f, "}}\n")?;

        Ok(())
    }
}

pub struct ScopedDi<'a> {
    di: &'a mut Di,
    scope: String,
    import_scopes: &'static [&'static str],
}

impl<'a> ScopedDi<'a> {
    fn new<S>(di: &'a mut Di, scope: S, import_scopes: &'static [&str]) -> Self
    where
        S: Into<String> + AsRef<str>,
    {
        Self {
            di,
            scope: scope.into(),
            import_scopes,
        }
    }

    pub fn contains<S>(&self, name: S) -> bool
    where
        S: Into<String>,
    {
        let name = Self::scoped_name(&self.scope, name.into());
        self.di.contains(name)
    }

    pub fn register_value<S, T>(&mut self, name: S, value: T, export: bool)
    where
        S: Into<String>,
        T: 'static + Clone,
    {
        let name = Self::scoped_name(&self.scope, name.into());
        let value = ScopedValue { export, value };
        self.di.register_value(name, value);
    }

    pub fn register_factory<S, F, T>(&mut self, name: S, factory: F, export: bool)
    where
        S: Into<String>,
        F: 'static + Fn(&mut ScopedDi) -> Result<T>,
        T: 'static + Clone,
    {
        let scope = self.scope.clone();
        let import_scopes = self.import_scopes;
        let name = Self::scoped_name(&self.scope, name.into());
        let factory = move |di: &mut Di| -> Result<ScopedValue<T>> {
            let mut scoped_di = di.scope(scope, import_scopes);
            Ok(ScopedValue {
                export,
                value: factory(&mut scoped_di)?,
            })
        };
        self.di.register_factory(name, factory);
    }

    pub fn inject<S, T>(&mut self, name: S) -> Result<T>
    where
        S: Into<String>,
        T: 'static + Clone,
    {
        let name = name.into();

        if self.di.stack.len() == 0 {
            let res = self
                .internal_inject(&name)
                .map_err(|_| Error::ScopedInjectError {
                    name,
                    stack: self.di.stack.clone(),
                });

            self.di.stack.clear();

            res
        } else {
            self.internal_inject(&name)
        }
    }

    fn internal_inject<S, T>(&mut self, name: S) -> Result<T>
    where
        S: Into<String>,
        T: 'static + Clone,
    {
        let name: String = name.into();

        // Find in the current scope
        if let Ok(v) = self
            .di
            .inject::<_, ScopedValue<T>>(Self::scoped_name(&self.scope, &name))
        {
            return Ok(v.value);
        }

        // Find in the imported scopes
        for scope in self.import_scopes {
            if let Ok(v) = self
                .di
                .inject::<_, ScopedValue<T>>(Self::scoped_name(scope, &name))
            {
                if v.export {
                    return Ok(v.value);
                }
            }
        }

        Err(Error::FactoryNotFound { name })
    }

    fn scoped_name<S1: AsRef<str>, S2: AsRef<str>>(scope: S1, name: S2) -> String {
        format!("{}#{}", scope.as_ref(), name.as_ref())
    }
}

#[derive(Clone)]
struct ScopedValue<T: 'static + Clone> {
    export: bool,
    value: T,
}
