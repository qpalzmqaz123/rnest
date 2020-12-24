use custom_error::custom_error;

custom_error! {pub Error
    FactoryNotFound{name: String} = "Factory not found '{name}'",
    TypeMismatch{name: String} = "Type mismatch '{name}'",
}

pub type Result<T> = std::result::Result<T, Error>;
