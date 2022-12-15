use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use actix_web::{
    web::{Form, Json, Path, Query},
    HttpRequest, HttpResponse,
};

use crate::{ValidatedForm, ValidatedJson, ValidatedPath, ValidatedQsQuery, ValidatedQuery};

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
impl_schema_for_integer! {isize}
impl_schema_for_integer! {usize}

impl_schema_for_number! {f32}
impl_schema_for_number! {f64}

impl_schema_for_boolean! {bool}

impl_schema_for_string! {&str}
impl_schema_for_string! {String}

impl OpenApiSchema for HttpResponse {
    fn get_schema() -> serde_json::Value {
        crate::json!({})
    }
}

impl OpenApiSchema for HttpRequest {
    fn get_schema() -> serde_json::Value {
        crate::json!({})
    }
}

impl<T: OpenApiSchema> OpenApiSchema for Vec<T> {
    fn get_schema() -> serde_json::Value {
        crate::json!({
            "type": "array",
            "items": T::get_schema(),
        })
    }
}

impl<T: OpenApiSchema> OpenApiSchema for Option<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for Json<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for Query<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for Path<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for Form<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for ValidatedJson<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for ValidatedQuery<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for ValidatedPath<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for ValidatedForm<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for ValidatedQsQuery<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema, E> OpenApiSchema for Result<T, E> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for Rc<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for Arc<T> {
    fn get_schema() -> serde_json::Value {
        T::get_schema()
    }
}

impl<K, T: OpenApiSchema> OpenApiSchema for HashMap<K, T> {
    fn get_schema() -> serde_json::Value {
        crate::json!({
            "type": "object",
            "additionalProperties": T::get_schema(),
        })
    }
}

impl<T: OpenApiSchema> OpenApiSchema for HashSet<T> {
    fn get_schema() -> serde_json::Value {
        <Vec<T>>::get_schema()
    }
}

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

#[cfg(test)]
#[allow(unused)]
mod test {
    use std::collections::{HashMap, HashSet};

    use crate::{self as rnest, openapi};

    use rnest::OpenApiSchema;

    #[test]
    fn test1() {
        #[derive(Debug, OpenApiSchema)]
        struct A {
            #[openapi(rename = "A", description = "Field A", example = 1)]
            a: u32,
            #[openapi(example = "Example b", enums = ["a", "b"])]
            b: String,
            c: Option<f64>,
            #[openapi(schema = rnest::json!({
                "type": "string",
                "example": "Example d",
            }))]
            d: String,
        }

        assert_eq!(
            A::get_schema(),
            rnest::json!({
                "type": "object",
                "required": ["A", "b", "d"],
                "properties": {
                    "A": {
                        "type": "integer",
                        "description": "Field A",
                        "example": 1,
                    },
                    "b": {
                        "type": "string",
                        "example": "Example b",
                        "enum": ["a", "b"],
                    },
                    "c": {
                        "type": "number",
                    },
                    "d": {
                        "type": "string",
                        "example": "Example d",
                    },
                },
            })
        );
    }

    #[test]
    fn test2() {
        #[derive(Debug, OpenApiSchema)]
        enum A {
            #[openapi(description = "desc a")]
            A,
            #[openapi(rename = "b", description = "desc b")]
            B(u32),
            #[openapi(description = "desc c")]
            C {
                #[openapi(description = "aa")]
                a: u32,
                #[openapi(rename = "B", example = 123)]
                b: u32,
            },
        }

        assert_eq!(
            A::get_schema(),
            rnest::json!({
                "oneOf": [
                    {
                        "type": "string",
                        "example": "A",
                        "description": "desc a"
                    },
                    {
                        "type": "object",
                        "required": ["b"],
                        "description": "desc b",
                        "properties": {
                            "b": {
                                "type": "integer"
                            },
                        },
                    },
                    {
                        "type": "object",
                        "required": ["C"],
                        "description": "desc c",
                        "properties": {
                            "C": {
                                "type": "object",
                                "required": ["a", "B"],
                                "properties": {
                                    "a": {
                                        "type": "integer",
                                        "description": "aa",
                                    },
                                    "B": {
                                        "type": "integer",
                                        "example": 123,
                                    },
                                },
                            },
                        },
                    },
                ]
            })
        );
    }

    #[test]
    fn test3() {
        #[derive(Debug, OpenApiSchema)]
        struct B {
            #[openapi(example = 10)]
            b: u32,
        }

        #[derive(Debug, OpenApiSchema)]
        #[openapi(tag = "type")]
        enum A {
            #[openapi(description = "desc a")]
            A,
            #[openapi(rename = "b", description = "desc b")]
            B(B),
            #[openapi(description = "desc c")]
            C { a: u32 },
        }

        assert_eq!(
            A::get_schema(),
            rnest::json!({
                "oneOf": [
                    {
                        "type": "object",
                        "required": ["type"],
                        "properties": {
                            "type": {
                                "type": "string",
                                "example": "A",
                                "description": "desc a",
                            },
                        },
                    },
                    {
                        "type": "object",
                        "required": ["b", "type"],
                        "properties": {
                            "type": {
                                "type": "string",
                                "example": "b",
                                "description": "desc b",
                            },
                            "b": {
                                "type": "integer",
                                "example": 10,
                            },
                        },
                    },
                    {
                        "type": "object",
                        "required": ["a", "type"],
                        "properties": {
                            "type": {
                                "type": "string",
                                "example": "C",
                                "description": "desc c",
                            },
                            "a": {
                                "type": "integer",
                            },
                        },
                    },
                ],
            })
        );
    }

    #[test]
    fn test4() {
        #[derive(Debug, OpenApiSchema)]
        struct A {
            aa: u32,
        }

        #[derive(Debug, OpenApiSchema)]
        struct B {
            b: u32,
            #[openapi(flatten)]
            a: A,
        }

        assert_eq!(
            B::get_schema(),
            rnest::json!({
                "type": "object",
                "required": ["b", "aa"],
                "properties": {
                    "b": {
                        "type": "integer",
                    },
                    "aa": {
                        "type": "integer",
                    },
                },
            })
        );
    }

    #[test]
    fn test5() {
        #[derive(Debug, OpenApiSchema)]
        struct A {
            aa: u32,
        }

        #[derive(Debug, OpenApiSchema)]
        #[openapi(tag = "type")]
        enum B {
            M,
            N {
                #[openapi(flatten)]
                a: A,
                b: u32,
            },
        }

        assert_eq!(
            B::get_schema(),
            rnest::json!({
                "oneOf": [
                    {
                        "type": "object",
                        "required": ["type"],
                        "properties": {
                            "type": {
                                "type": "string",
                                "example": "M",
                            }
                        },
                    },
                    {
                        "type": "object",
                        "required": ["aa", "b", "type"],
                        "properties": {
                            "type": {
                                "type": "string",
                                "example": "N",
                            },
                            "aa": {
                                "type": "integer",
                            },
                            "b": {
                                "type": "integer",
                            },
                        },
                    },
                ],
            })
        );
    }

    #[test]
    fn test6() {
        #[derive(Debug, OpenApiSchema)]
        struct A {
            aa1: u32,
            aa2: String,
        }

        #[derive(Debug, OpenApiSchema)]
        struct B {
            bb1: HashMap<u32, A>,
            bb2: HashSet<A>,
        }

        assert_eq!(
            B::get_schema(),
            rnest::json!({
                "type": "object",
                "required": ["bb1", "bb2"],
                "properties": {
                    "bb1": {
                        "type": "object",
                        "additionalProperties": {
                            "type": "object",
                            "required": ["aa1", "aa2"],
                            "properties": {
                                "aa1": {
                                    "type": "integer",
                                },
                                "aa2": {
                                    "type": "string",
                                },
                            },
                        },
                    },
                    "bb2": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["aa1", "aa2"],
                            "properties": {
                                "aa1": {
                                    "type": "integer",
                                },
                                "aa2": {
                                    "type": "string",
                                },
                            },
                        },
                    },
                },
            })
        );
    }
}
