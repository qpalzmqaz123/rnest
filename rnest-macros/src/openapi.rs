use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::{quote, ToTokens};
use syn::{Attribute, Data, DataEnum, DataStruct, DeriveInput, Expr, Fields, Ident, Lit, Type};

#[derive(Debug)]
enum AttrInfo {
    Rename(String),
    Desc(String),
    Example(TokenStream),
    Enums(TokenStream),
    Schema(TokenStream),
    Tag(String),
    Flatten,
    Skip,
}

pub struct StructFieldInfo<'a> {
    name: &'a Ident,
    ty: &'a Type,
    rename: Option<String>,
    description: Option<String>,
    example: Option<TokenStream>,
    enums: Option<TokenStream>,
    schema: Option<TokenStream>,
    flatten: bool,
    skip: bool,
}

pub struct StructInfo<'a> {
    name: &'a Ident,
    fields: Vec<StructFieldInfo<'a>>,
}

pub enum EnumFieldInfo<'a> {
    /// None
    Unit {
        name: &'a Ident,
        rename: Option<String>,
        description: Option<String>,
    },

    /// Red(u8)
    Unnamed {
        name: &'a Ident,
        ty: &'a Type,
        rename: Option<String>,
        description: Option<String>,
    },

    /// Color {r: u8, g: u8; b: u8}
    Named {
        name: &'a Ident,
        fields: Vec<StructFieldInfo<'a>>,
        rename: Option<String>,
        description: Option<String>,
    },
}

pub struct EnumInfo<'a> {
    name: &'a Ident,
    tag: Option<String>,
    fields: Vec<EnumFieldInfo<'a>>,
}

pub enum Openapi<'a> {
    Struct(StructInfo<'a>),
    Enum(EnumInfo<'a>),
}

impl<'a> Openapi<'a> {
    pub fn parse(input: &'a DeriveInput) -> Self {
        let name = &input.ident;
        let mut tag = None;

        // Parse attr
        for attr in &input.attrs {
            if attr.path.to_token_stream().to_string() != "openapi" {
                continue;
            }

            for attr_info in parse_openapi_attr(&attr) {
                match attr_info {
                    AttrInfo::Tag(t) => tag = Some(t),
                    _ => abort!(attr, "Invalid attr"),
                }
            }
        }

        match &input.data {
            Data::Struct(st) => Self::Struct(Self::parse_struct(name, st)),
            Data::Enum(enu) => Self::Enum(Self::parse_enum(name, enu, tag)),
            _ => abort!(input, "#[openapi] only works for structs and enums"),
        }
    }

    pub fn gen(&self) -> TokenStream {
        match self {
            Self::Struct(st) => Self::gen_struct(st),
            Self::Enum(enu) => Self::gen_enum(enu),
        }
    }

    fn parse_struct(st_name: &'a Ident, st: &'a DataStruct) -> StructInfo<'a> {
        let mut fields = vec![];
        for field in &st.fields {
            let name = field
                .ident
                .as_ref()
                .unwrap_or_else(|| abort!(field, "field must have a name"));
            let ty = &field.ty;

            fields.push(get_field_info_from_attr(name, ty, &field.attrs));
        }

        StructInfo {
            name: st_name,
            fields,
        }
    }

    fn parse_enum(enu_name: &'a Ident, enu: &'a DataEnum, tag: Option<String>) -> EnumInfo<'a> {
        let mut fields = vec![];

        for var in &enu.variants {
            let name = &var.ident;
            let mut rename = None;
            let mut description = None;

            // Parse attr
            for attr in &var.attrs {
                if attr.path.to_token_stream().to_string() != "openapi" {
                    continue;
                }

                for attr_info in parse_openapi_attr(&attr) {
                    match attr_info {
                        AttrInfo::Rename(name) => rename = Some(name),
                        AttrInfo::Desc(name) => description = Some(name),
                        _ => abort!(
                            attr,
                            "only #[openapi(rename = \"name\", description = \"desc\")] is supported at enum key node"
                        ),
                    }
                }
            }

            match &var.fields {
                Fields::Named(named_fields) => {
                    let mut inner_fields = vec![];
                    for field in &named_fields.named {
                        let name = field
                            .ident
                            .as_ref()
                            .unwrap_or_else(|| abort!(field, "field must have a name"));
                        let ty = &field.ty;

                        inner_fields.push(get_field_info_from_attr(name, ty, &field.attrs));
                    }

                    fields.push(EnumFieldInfo::Named {
                        name,
                        fields: inner_fields,
                        rename,
                        description,
                    });
                }
                Fields::Unnamed(unnamed_fields) => {
                    if unnamed_fields.unnamed.len() != 1 {
                        abort!(unnamed_fields, "enum fields length must 1");
                    }

                    let field = unnamed_fields
                        .unnamed
                        .first()
                        .unwrap_or_else(|| abort!(unnamed_fields, "Cannot get first field"));

                    fields.push(EnumFieldInfo::Unnamed {
                        name,
                        ty: &field.ty,
                        rename,
                        description,
                    });
                }
                Fields::Unit => {
                    fields.push(EnumFieldInfo::Unit {
                        name,
                        rename,
                        description,
                    });
                }
            }
        }

        EnumInfo {
            name: enu_name,
            tag,
            fields,
        }
    }

    fn gen_struct(st: &StructInfo) -> TokenStream {
        let name = st.name;

        let requireds_toks = gen_requireds_toks(&st.fields);
        let properties_toks = gen_properties_toks(&st.fields);

        quote! {
            impl rnest::OpenApiSchema for #name {
                fn get_schema() -> serde_json::Value {
                    let mut requireds = Vec::<String>::new();
                    let mut properties = std::collections::HashMap::<String, serde_json::Value>::new();

                    // Init requireds
                    #(#requireds_toks)*

                    // Init properties
                    #(#properties_toks)*

                    rnest::json!({
                        "type": "object",
                        "required": requireds,
                        "properties": properties,
                    })
                }
            }
        }
    }

    fn gen_enum(enu: &EnumInfo) -> TokenStream {
        let name = enu.name;
        let fields_toks = enu
            .fields
            .iter()
            .map(|field| match field {
                EnumFieldInfo::Named {
                    name,
                    fields,
                    rename,
                    description
                } => {
                    let name = rename.clone().unwrap_or(name.to_string());
                    let requireds_toks = gen_requireds_toks(fields);
                    let properties_toks = gen_properties_toks(fields);
                    let description_toks = description.as_ref().map_or_else(|| quote! {}, |desc| quote! { "description": #desc, });

                    if let Some(tag) = &enu.tag {
                        quote! {
                            (|| {
                                let mut requireds = Vec::<String>::new();
                                let mut properties = std::collections::HashMap::<String, serde_json::Value>::new();

                                // Init requireds
                                #(#requireds_toks)*

                                // Init properties
                                #(#properties_toks)*

                                // Append tag to requireds
                                requireds.push(#tag.into());

                                // Append tag schema to properties
                                properties.insert(#tag.into(), rnest::json!({
                                    "type": "string",
                                    "example": #name,
                                    #description_toks
                                }));

                                rnest::json!({
                                    "type": "object",
                                    "required": requireds,
                                    "properties": properties,
                                })
                            })()
                        }
                    } else {
                        quote! {
                            (|| {
                                let mut requireds = Vec::<String>::new();
                                let mut properties = std::collections::HashMap::<String, serde_json::Value>::new();

                                // Init requireds
                                #(#requireds_toks)*

                                // Init properties
                                #(#properties_toks)*

                                rnest::json!({
                                    "type": "object",
                                    "required": [#name],
                                    #description_toks
                                    "properties": {
                                        #name: {
                                            "type": "object",
                                            "required": requireds,
                                            "properties": properties,
                                        },
                                    },
                                })
                            })()
                        }
                    }
                }
                EnumFieldInfo::Unnamed { name, ty, rename, description } => {
                    let name = rename.clone().unwrap_or(name.to_string());
                    let description_toks = description.as_ref().map_or_else(|| quote! {}, |desc| quote! { "description": #desc, });

                    if let Some(tag) = &enu.tag {
                        quote! {
                            (|| {
                                let mut schema = <#ty as rnest::OpenApiSchema>::get_schema();
                                #[allow(clippy::indexing_slicing)]
                                if let Some(arr) = schema["required"].as_array_mut() {
                                    arr.push(#tag.into());
                                }
                                #[allow(clippy::indexing_slicing)]
                                if let Some(obj) = schema["properties"].as_object_mut() {
                                    obj.insert(#tag.into(), rnest::json!({
                                        "type": "string",
                                        "example": #name,
                                        #description_toks
                                    }));
                                }

                                schema
                            })()
                        }
                    } else {
                        quote! {
                            {
                                "type": "object",
                                "required": [#name],
                                #description_toks
                                "properties": {
                                    #name: <#ty as rnest::OpenApiSchema>::get_schema(),
                                },
                            }
                        }
                    }
                }
                EnumFieldInfo::Unit { name, rename, description } => {
                    let name = rename.clone().unwrap_or(name.to_string());
                    let description_toks = description.as_ref().map_or_else(|| quote! {}, |desc| quote! { "description": #desc, });

                    if let Some(tag) = &enu.tag {
                        quote! {
                            {
                                "type": "object",
                                "required": [#tag],
                                "properties": {
                                    #tag: {
                                        "type": "string",
                                        "example": #name,
                                        #description_toks
                                    }
                                },
                            }
                        }
                    } else {
                        quote! {
                            {
                                "type": "string",
                                "example": #name,
                                #description_toks
                            }
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        quote! {
            impl rnest::OpenApiSchema for #name {
                fn get_schema() -> serde_json::Value {
                    rnest::json!({
                        "oneOf": [
                            #(#fields_toks),*
                        ],
                    })
                }
            }
        }
    }
}

fn gen_requireds_toks(fields: &[StructFieldInfo]) -> Vec<TokenStream> {
    let mut list = vec![];
    for field in fields {
        if let Type::Path(p) = field.ty {
            if let Some(seg) = p.path.segments.first() {
                if seg.ident == "Option" {
                    // If type is option, required is false
                    continue;
                }
            }
        }

        // Append name to requireds
        let toks = if field.flatten {
            // Flatten field
            let ty = field.ty;
            quote! {
                {
                    let schema = <#ty as rnest::OpenApiSchema>::get_schema();

                    // Append field requireds to self requireds
                    #[allow(clippy::indexing_slicing)]
                    if let Some(arr) = schema["required"].as_array() {
                        for val in arr {
                            if let Some(req) = val.as_str() {
                                requireds.push(req.into());
                            }
                        }
                    }
                }
            }
        } else {
            // Normal field
            if field.skip {
                continue;
            }

            let name = field.rename.clone().unwrap_or(field.name.to_string());
            quote! {
                requireds.push(#name.into());
            }
        };

        list.push(toks);
    }

    list
}

fn gen_properties_toks(fields: &[StructFieldInfo]) -> Vec<TokenStream> {
    let mut list = vec![];
    for field in fields {
        let name = field.rename.clone().unwrap_or(field.name.to_string());
        let ty = field.ty;

        // Process description
        let description_toks = if let Some(desc) = &field.description {
            quote! { obj.insert("description".into(), rnest::json!(#desc)); }
        } else {
            quote! {}
        };

        // Process example
        let example_toks = if let Some(ex) = &field.example {
            quote! { obj.insert("example".into(), rnest::json!(#ex)); }
        } else {
            quote! {}
        };

        // Process enums
        let enum_toks = if let Some(enus) = &field.enums {
            quote! { obj.insert("enum".into(), rnest::json!(#enus)); }
        } else {
            quote! {}
        };

        // Generate toks
        let toks = if let Some(schema) = &field.schema {
            // Custom schema
            quote! {
                properties.insert(#name.into(), rnest::json!(#schema));
            }
        } else {
            // Auto generated schema

            if field.skip {
                continue;
            } else if field.flatten {
                // Flatten field
                quote! {
                    {
                        let schema = <#ty as rnest::OpenApiSchema>::get_schema();

                        // Append field properties to self properties
                        #[allow(clippy::indexing_slicing)]
                        if let Some(prop) = schema["properties"].as_object() {
                            properties.extend(prop.clone());
                        }
                    }
                }
            } else {
                // Normal field
                quote! {
                    {
                        let mut schema = <#ty as rnest::OpenApiSchema>::get_schema();
                        if let Some(mut obj) = schema.as_object_mut() {
                            #description_toks
                            #example_toks
                            #enum_toks
                        }

                        properties.insert(#name.into(), schema);
                    }
                }
            }
        };

        list.push(toks);
    }

    list
}

fn get_field_info_from_attr<'a>(
    name: &'a Ident,
    ty: &'a Type,
    attrs: &[Attribute],
) -> StructFieldInfo<'a> {
    let mut rename = None;
    let mut description = None;
    let mut example = None;
    let mut enums = None;
    let mut schema = None;
    let mut flatten = false;
    let mut skip = false;

    // Parse attr
    for attr in attrs {
        if attr.path.to_token_stream().to_string() != "openapi" {
            continue;
        }

        for attr_info in parse_openapi_attr(&attr) {
            match attr_info {
                AttrInfo::Rename(name) => rename = Some(name),
                AttrInfo::Desc(desc) => description = Some(desc),
                AttrInfo::Example(ex) => example = Some(ex),
                AttrInfo::Enums(enus) => enums = Some(enus),
                AttrInfo::Schema(sch) => schema = Some(sch),
                AttrInfo::Tag(_) => {
                    abort!(attr, "`#[openapi(tag)]` is only allowed on enum header")
                }
                AttrInfo::Flatten => flatten = true,
                AttrInfo::Skip => skip = true,
            }
        }
    }

    StructFieldInfo {
        name,
        ty,
        rename,
        description,
        example,
        enums,
        schema,
        flatten,
        skip,
    }
}

fn parse_openapi_attr(attr: &Attribute) -> Vec<AttrInfo> {
    const PARSE_ERR_STR: &'static str = "Parse failed, syntax is #[openapi(field [= value])]";
    const ARG_HELP: &'static str = r#"Syntax is openapi(rename = "NAME" | description = "DESC" | example = (NUM | "STRING") | enums = ["STRING", ...] | schema = {} | tag = "NAME" | flatten | skip, ...)"#;

    // Generate function call tokens: rorm(xxx)
    let path = attr.path.clone();
    let toks = attr.tokens.clone();
    let call_toks = quote::quote! {#path #toks};

    let args = if let Ok(call) = syn::parse2::<syn::ExprCall>(call_toks) {
        call.args
    } else {
        abort!(attr.tokens, PARSE_ERR_STR);
    };

    let mut attrs = Vec::<AttrInfo>::new();

    // Parse args
    for expr in &args {
        match expr {
            Expr::Path(p) => {
                let field_name = p.to_token_stream().to_string();
                match field_name.as_str() {
                    // Parse flatten
                    "flatten" => attrs.push(AttrInfo::Flatten),

                    // Parse skip
                    "skip" => attrs.push(AttrInfo::Skip),

                    // Error
                    _ => abort!(expr, "Syntax error while decode path"; help = ARG_HELP),
                }
            }
            Expr::Assign(assign) => {
                let field_name = assign.left.to_token_stream().to_string();
                match field_name.as_str() {
                    // Parse rename = "NAME"
                    "rename" => attrs.push(AttrInfo::Rename(get_str(&assign.right))),

                    // Parse description = "DESC"
                    "description" => attrs.push(AttrInfo::Desc(get_str(&assign.right))),

                    // Parse example = (NUM | "STRING")
                    "example" => attrs.push(AttrInfo::Example(assign.right.to_token_stream())),

                    // Parse enums = ["STRING", ...]
                    "enums" => attrs.push(AttrInfo::Enums(assign.right.to_token_stream())),

                    // Parse schema = {}
                    "schema" => attrs.push(AttrInfo::Schema(assign.right.to_token_stream())),

                    // Parse tag = NAME
                    "tag" => attrs.push(AttrInfo::Tag(get_str(&assign.right))),

                    // Error
                    _ => abort!(expr, "Syntax error while decode assign"; help = ARG_HELP),
                }
            }
            _ => abort!(expr, "Syntax error while match expr"; help = ARG_HELP),
        }
    }

    attrs
}

/// Get string from expr
fn get_str(expr: &Expr) -> String {
    if let Expr::Lit(lit) = expr {
        if let Lit::Str(s) = &lit.lit {
            return s.value();
        }
    }

    abort!(expr, "Expect string")
}
