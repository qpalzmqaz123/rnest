pub trait OpenApiSchema {
    fn get_schema() -> serde_json::Value;
}

macro_rules! impl_schema_for_string {
    ($type:ty) => {
        impl OpenApiSchema for $type {
            fn get_schema() -> serde_json::Value {
                serde_json::json!({
                    "type": "string",
                })
            }
        }
    };
}

macro_rules! impl_schema_for_number {
    ($type:ty) => {
        impl OpenApiSchema for $type {
            fn get_schema() -> serde_json::Value {
                serde_json::json!({
                    "type": "number",
                })
            }
        }
    };
}

macro_rules! impl_schema_for_integer {
    ($type:ty) => {
        impl OpenApiSchema for $type {
            fn get_schema() -> serde_json::Value {
                serde_json::json!({
                    "type": "integer",
                })
            }
        }
    };
}

macro_rules! impl_schema_for_boolean {
    ($type:ty) => {
        impl OpenApiSchema for $type {
            fn get_schema() -> serde_json::Value {
                serde_json::json!({
                    "type": "boolean",
                })
            }
        }
    };
}

impl_schema_for_integer! {i8}
impl_schema_for_integer! {u8}
impl_schema_for_integer! {i16}
impl_schema_for_integer! {u16}
impl_schema_for_integer! {i32}
impl_schema_for_integer! {u32}
impl_schema_for_integer! {i64}
impl_schema_for_integer! {u64}

impl_schema_for_number! {f32}
impl_schema_for_number! {f64}

impl_schema_for_boolean! {bool}

impl_schema_for_string! {&str}
impl_schema_for_string! {String}

pub struct OpenApiBuilder {
    version: String,
    title: String,
    paths: serde_json::Value,
    security_schemes: serde_json::Value,
}

impl OpenApiBuilder {
    pub fn new(paths: serde_json::Value) -> Self {
        Self {
            paths,
            version: "0.0.1".to_string(),
            title: "API".to_string(),
            security_schemes: serde_json::json!({}),
        }
    }

    pub fn version<S: Into<String>>(mut self, v: S) -> Self {
        self.version = v.into();
        self
    }

    pub fn title<S: Into<String>>(mut self, v: S) -> Self {
        self.title = v.into();
        self
    }

    pub fn add_basic_auth<S: Into<String>>(mut self, name: S) -> Self {
        if let Some(obj) = self.security_schemes.as_object_mut() {
            obj.insert(
                name.into(),
                serde_json::json!({
                    "type": "http",
                    "scheme": "basic",
                }),
            );
        }
        self
    }

    pub fn add_bearer_auth<S: Into<String>>(mut self, name: S) -> Self {
        if let Some(obj) = self.security_schemes.as_object_mut() {
            obj.insert(
                name.into(),
                serde_json::json!({
                    "type": "http",
                    "scheme": "bearer",
                }),
            );
        }
        self
    }

    pub fn build(self) -> serde_json::Value {
        serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "version": self.version,
                "title": self.title,
            },
            "paths": self.paths,
            "components": {
                "securitySchemes": self.security_schemes,
            },
        })
    }
}
