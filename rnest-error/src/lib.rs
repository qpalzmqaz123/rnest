use custom_error::custom_error;

custom_error! {pub Error
    FactoryNotFound{name: String} = "Factory not found '{name}'",
    TypeMismatch{name: String} = "Type mismatch '{name}'",
    ScopedInjectError{name: String, stack: Vec<String>} = @{ format!("Scoped di inject error '{}', stack: '{:?}'", name, stack) },
    CircularDependencies{name: String} = "Circular dependencies found '{name}'",
}

pub type Result<T> = std::result::Result<T, Error>;
