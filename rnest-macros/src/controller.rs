use std::collections::HashMap;

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::{format_ident, quote, ToTokens};
use syn::{Attribute, Block, Expr, FnArg, ImplItem, ImplItemMethod, ItemImpl, PatType, ReturnType};

use crate::utils;

#[derive(Debug, Clone)]
enum ControllerMethodArg {
    Param {
        // TODO: Add ref name
        name: String, // Variable name
        ty: String,   // Type
    },
    Body {
        name: String,
        ty: String,
    },
    Query {
        name: String,
        ty: String,
    },
    Raw {
        name: String,
        ty: String,
    },
}

struct MethodOpenapiInfo {
    bearer_auth: bool,
    security: Option<TokenStream>,
    tags: Option<TokenStream>,
    summary: Option<TokenStream>,
    parameters: Option<TokenStream>,
    request_body: Option<TokenStream>,
    responses: Option<TokenStream>,
}

struct ControllerMethodInfo {
    fn_name: String,
    is_async: bool,
    is_mut_self: bool, // true: &mut self, false: &self
    args: Vec<ControllerMethodArg>,
    output_type: String,
    method: String, // TODO: Use enum in the future, currently it is one of ['get', 'post', 'delete', 'put']
    url: String,
    openapi: Option<MethodOpenapiInfo>,
}

impl ControllerMethodInfo {
    fn gen_method(&self, block: &Block) -> TokenStream {
        let async_token = if self.is_async {
            quote! {async}
        } else {
            quote! {}
        };
        let fn_token = format_ident!("{}", self.fn_name);
        let self_token = if self.is_mut_self {
            quote! {&mut self}
        } else {
            quote! {&self}
        };
        let out_token: TokenStream = self
            .output_type
            .parse()
            .expect("Parse controller return type error");
        let arg_tokens: Vec<TokenStream> = self
            .args
            .iter()
            .map(|arg| match arg {
                ControllerMethodArg::Param { name, ty } => {
                    let name_token = format_ident!("{}", name);
                    let ty_token: TokenStream = ty.parse().expect("Parse param type error");

                    quote! {#name_token: #ty_token}
                }
                ControllerMethodArg::Body { name, ty } => {
                    let name_token = format_ident!("{}", name);
                    let ty_token: TokenStream = ty.parse().expect("Parse body type error");

                    quote! {#name_token: #ty_token}
                }
                ControllerMethodArg::Query { name, ty } => {
                    let name_token = format_ident!("{}", name);
                    let ty_token: TokenStream = ty.parse().expect("Parse query type error");

                    quote! {#name_token: #ty_token}
                }
                ControllerMethodArg::Raw { name, ty } => {
                    let name_token = format_ident!("{}", name);
                    let ty_token: TokenStream = ty.parse().expect("Parse query type error");

                    quote! {#name_token: #ty_token}
                }
            })
            .collect();

        quote! {
            #async_token fn #fn_token(#self_token, #(#arg_tokens,)*) -> #out_token
                #block
        }
    }

    fn gen_actix_web_cb(&self, struct_name: String) -> TokenStream {
        let struct_token = format_ident!("{}", struct_name);
        let method_token = format_ident!("{}", self.fn_name);
        let cb_token = format_ident!("{}", self.actix_web_cb_name());
        let out_token: TokenStream = self
            .output_type
            .parse()
            .expect("Parse controller cb return type error");
        let await_token = if self.is_async {
            quote! {.await}
        } else {
            quote! {}
        };
        // let lock_method = if self.is_mut_self {
        //     quote! {write}
        // } else {
        //     quote! {read}
        // };
        let router_param_map = self.router_param_map();
        let url_args = utils::get_args_from_url(&self.url);
        let (router_param_name_tokens, router_param_type_tokens): (
            Vec<syn::Ident>,
            Vec<syn::Ident>,
        ) = url_args
            .iter()
            .fold((vec![], vec![]), |(mut v1, mut v2), arg| {
                if let Some(ty) = router_param_map.get(arg) {
                    v1.push(format_ident!("{}", arg));
                    v2.push(format_ident!("{}", ty));
                }

                (v1, v2)
            });
        let method_args_from_cb = self.method_args_from_cb();
        let bodies: Vec<TokenStream> = self.args.iter().fold(vec![], |mut v, arg| {
            if let ControllerMethodArg::Body { name, ty } = arg {
                let name_token = format_ident!("{}", name);
                let type_token: TokenStream = ty.parse().expect("Parse body type error");
                v.push(quote! {#name_token: #type_token});
            }

            v
        });
        let queries: Vec<TokenStream> = self.args.iter().fold(vec![], |mut v, arg| {
            if let ControllerMethodArg::Query { name, ty } = arg {
                let name_token = format_ident!("{}", name);
                let type_token: TokenStream = ty.parse().expect("Parse query type error");
                v.push(quote! {#name_token: #type_token});
            }

            v
        });
        let raws: Vec<TokenStream> = self.args.iter().fold(vec![], |mut v, arg| {
            if let ControllerMethodArg::Raw { name, ty } = arg {
                let name_token = format_ident!("{}", name);
                let type_token: TokenStream = ty.parse().expect("Parse query type error");
                v.push(quote! {#name_token: #type_token});
            }

            v
        });

        quote! {
            async fn #cb_token(
                __rnest_instance: rnest::actix_web::web::Data<std::sync::Arc<#struct_token>>,
                __rnest_path: rnest::actix_web::web::Path<(#(#router_param_type_tokens,)*)>,
                #(#bodies,)*
                #(#queries,)*
                #(#raws,)*
            ) -> #out_token {
                let (#(#router_param_name_tokens,)*) = __rnest_path.into_inner();
                __rnest_instance.#method_token(#(#method_args_from_cb,)*)#await_token
            }
        }
    }

    fn actix_web_cb_name(&self) -> String {
        format!("__rnest_{}_cb", self.fn_name)
    }

    fn router_param_map(&self) -> HashMap<String, String> {
        // HashMap<name, type>
        self.args.iter().fold(HashMap::new(), |mut map, arg| {
            if let ControllerMethodArg::Param { name, ty } = arg {
                map.insert(name.clone(), ty.clone());
            }

            map
        })
    }

    fn method_args_from_cb(&self) -> Vec<TokenStream> {
        self.args
            .iter()
            .map(|arg| match arg {
                ControllerMethodArg::Param { name, .. } => {
                    let arg_token = format_ident!("{}", name);
                    quote! {#arg_token}
                }
                ControllerMethodArg::Body { name, .. } => {
                    let arg_token = format_ident!("{}", name);
                    quote! {#arg_token}
                }
                ControllerMethodArg::Query { name, .. } => {
                    let arg_token = format_ident!("{}", name);
                    quote! {#arg_token}
                }
                ControllerMethodArg::Raw { name, .. } => {
                    let arg_token = format_ident!("{}", name);
                    quote! {#arg_token}
                }
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct Controller {}

impl Controller {
    pub fn parse(attr: TokenStream, imp: ItemImpl) -> TokenStream {
        let scope_prefix = match utils::parse_string_token(&attr) {
            Ok(s) => s,
            Err(_) => abort! { attr,
                "Syntax error on controller";
                note = "Syntax is #[controller(\"/api\")]";
            },
        };
        let self_impl = Self::parse_self_impl(&imp);
        let controller_impl = Self::parse_controller_impl(&scope_prefix, &imp);

        quote! {
            #self_impl

            #controller_impl
        }
    }

    pub fn gen_openapi3(attr: TokenStream, imp: ItemImpl) -> TokenStream {
        // TODO: Optimize
        let scope_prefix = match utils::parse_string_token(&attr) {
            Ok(s) => s,
            Err(_) => abort! { attr,
                "Syntax error on controller";
                note = "Syntax is #[controller(\"/api\")]";
            },
        };

        let name = imp.self_ty;
        // TODO: optimize
        let mut methods: Vec<TokenStream> = Vec::new();
        for item in &imp.items {
            match item {
                ImplItem::Method(method) => {
                    if method.attrs.len() > 0 {
                        let info = Self::parse_controller_method_info(method);
                        methods.push(Self::gen_openapi3_with_method(scope_prefix.as_str(), info));
                    }
                }
                _ => continue,
            }
        }

        quote! {
            impl #name {
                pub fn __rnest_get_openapi3_spec() -> rnest::JsonValue {
                    let mut specs: Vec<(String, rnest::JsonValue)> = Vec::new();
                    #(specs.push((#methods)());)*

                    let mut spec_map: std::collections::HashMap<String, Vec<rnest::JsonValue>> = std::collections::HashMap::new();
                    for (url, body) in specs {
                        let mut bodies = match spec_map.remove(&url) {
                            Some(v) => v,
                            None => Vec::new(),
                        };
                        bodies.push(body);
                        spec_map.insert(url, bodies);
                    }

                    let mut paths = rnest::json!({});
                    for (url, bodies) in spec_map.into_iter() {
                        let bodies = bodies.into_iter().fold(rnest::json!({}), |mut v, i| {
                            v.as_object_mut().unwrap().extend(i.as_object().unwrap().clone());
                            v
                        });
                        paths.as_object_mut().unwrap().insert(url, bodies);
                    }

                    paths
                }
            }
        }
    }

    fn gen_openapi3_with_method(scope: &str, info: ControllerMethodInfo) -> TokenStream {
        let url = utils::normalize_url(format!("{}/{}", scope, info.url));
        let method = &info.method;

        // If openapi is enabled, generate openapi3 spec
        if let Some(mut openapi) = info.openapi {
            // If security is not set, set default security
            if openapi.security.is_none() {
                let mut toks_list = Vec::new();

                if openapi.bearer_auth {
                    toks_list.push(quote! {{
                        "bearerAuth": []
                    }});
                }

                openapi.security = Some(quote! {[
                    #(#toks_list),*
                ]});
            }

            // If parameter is not set, use default value
            let args = info.args.clone();
            openapi.parameters = Some(openapi.parameters.unwrap_or_else(move || {
                let param_toks = args
                    .iter()
                    .filter_map(|arg| match arg {
                        ControllerMethodArg::Query { name, ty } => {
                            let ty_toks = ty.parse::<TokenStream>().unwrap();

                            Some(quote! {{
                                "in": "query",
                                "name": #name,
                                "schema": <#ty_toks as rnest::OpenApiSchema>::get_schema(),
                            }})
                        }
                        ControllerMethodArg::Param { name, ty } => Some({
                            let ty_toks = ty.parse::<TokenStream>().unwrap();

                            quote! {{
                                    "in": "path",
                                    "name": #name,
                                    "required": true,
                                    "schema": <#ty_toks as rnest::OpenApiSchema>::get_schema(),
                                }
                            }
                        }),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                quote! { rnest::json!([
                    #(#param_toks),*
                ])}
            }));

            // If request body is not set and #[body] exists, set auto request body
            if let Some(first_body_type) = info
                .args
                .iter()
                // .find_map(|arg| matches!(arg, ControllerMethodArg::Body { .. }))
                .find_map(|arg| match arg {
                    ControllerMethodArg::Body { ty, .. } => {
                        Some(ty.parse::<TokenStream>().expect("Parse body type error"))
                    }
                    _ => None,
                })
            {
                openapi.request_body = Some(openapi.request_body.unwrap_or(quote! {
                    // TODO: Analyze return type
                    rnest::json!({
                        "content": {
                            "application/json": {
                                "schema": <#first_body_type as rnest::OpenApiSchema>::get_schema(),
                            }
                        }
                    })
                }));
            }

            // If response is not set, set auto response
            let ret_ty_str = info.output_type.clone();
            openapi.responses = Some(openapi.responses.unwrap_or_else(move || {
                // Function return type
                let ret_ty: TokenStream = ret_ty_str
                    .parse()
                    .expect("Parse controller cb return type error");

                quote! {
                    // TODO: Analyze return type
                    rnest::json!({
                        "200": {
                            "description": "OK",
                            "content": {
                                "application/json": {
                                    "schema": <#ret_ty as rnest::OpenApiSchema>::get_schema(),
                                }
                            }
                        }
                    })
                }
            }));

            // Generate openapi spec rust code
            let security = openapi.security.map(|toks| quote! { "security": #toks, });
            let tags = openapi.tags.map(|toks| quote! { "tags": #toks, });
            let summary = openapi.summary.map(|toks| quote! { "summary": #toks, });
            let parameters = openapi
                .parameters
                .map(|toks| quote! { "parameters": #toks, });
            let request_body = openapi
                .request_body
                .map(|toks| quote! { "requestBody": #toks, });
            let responses = openapi.responses.map(|toks| quote! { "responses": #toks, });
            quote! {
                || -> (String, rnest::JsonValue) {
                    let url = #url.to_string();
                    let body = rnest::json!({
                        #method: {
                            #security
                            #tags
                            #summary
                            #parameters
                            #request_body
                            #responses
                        }
                    });

                    (url, body)
                }
            }
        } else {
            quote! {
                || -> (String, rnest::JsonValue) {
                    let url = #url.to_string();
                    (url, rnest::json!({}))
                }
            }
        }
    }

    fn parse_self_impl(imp: &ItemImpl) -> TokenStream {
        let struct_name_token = &imp.self_ty;
        let methods: Vec<&ImplItemMethod> = imp
            .items
            .iter()
            .map(|item| match item {
                ImplItem::Method(m) => m,
                _ => panic!(
                    "Expect method type in the impl block, received: '{}'",
                    quote! {#item}
                ),
            })
            .collect();
        let method_tokens: Vec<TokenStream> = methods
            .iter()
            .map(|m| Self::parse_method(m, quote! {#struct_name_token}.to_string()))
            .collect();

        quote! {
            impl #struct_name_token {
                #(#method_tokens)*
            }
        }
    }

    fn parse_controller_impl(scope_prefix: &String, imp: &ItemImpl) -> TokenStream {
        let scope_prefix = utils::normalize_url(scope_prefix);
        let struct_name_token = &imp.self_ty;
        let struct_name = quote! {#struct_name_token}.to_string();

        // TODO: optimize
        let mut method_infos: Vec<ControllerMethodInfo> = vec![];
        for item in &imp.items {
            match item {
                ImplItem::Method(method) => {
                    if method.attrs.len() > 0 {
                        let info = Self::parse_controller_method_info(method);
                        method_infos.push(info);
                    }
                }
                _ => continue,
            }
        }

        let scope_calls: Vec<TokenStream> = method_infos
            .iter()
            .map(|info| Self::parse_controller_impl_scope_call(&scope_prefix, &struct_name, info))
            .collect();

        quote! {
            impl rnest::Controller<Self, std::sync::Arc<Self>> for #struct_name_token {
                fn configure_actix_web(instance: std::sync::Arc<Self>, cfg: &mut rnest::actix_web::web::ServiceConfig) {
                    let scope = rnest::actix_web::web::scope(#scope_prefix).app_data(rnest::actix_web::web::Data::new(instance));

                    #(#scope_calls)*

                    cfg.service(scope);
                }
            }
        }
    }

    fn parse_controller_impl_scope_call(
        scope_prefix: &String,
        struct_name: &String,
        info: &ControllerMethodInfo,
    ) -> TokenStream {
        let struct_name_token = format_ident!("{}", struct_name);
        let mut url = utils::normalize_url(&info.url);
        let http_method_token = format_ident!("{}", info.method);
        let cb_token = format_ident!("{}", info.actix_web_cb_name());

        url.push('/');

        quote! {
            let scope = scope.route(
                #url,
                rnest::actix_web::web::#http_method_token().to(#struct_name_token::#cb_token),
            );

            std::env::var("RNEST_LOG_CONTROLLER").map(|var| if var == "1" {
                log::trace!("{} {} '{}{}' registered", stringify!(#struct_name_token), stringify!(#http_method_token), #scope_prefix, #url);
            }).ok();
        }
    }

    fn parse_method(method: &ImplItemMethod, struct_name: String) -> TokenStream {
        if method.attrs.len() > 0 {
            // If method has attrs, treat it as http api method
            Self::parse_controller_method(method, struct_name)
        } else {
            // If method has no attrs, treat it as private method
            quote! {#method}
        }
    }

    fn parse_controller_method(method: &ImplItemMethod, struct_name: String) -> TokenStream {
        let info = Self::parse_controller_method_info(method);
        let method_tokens = info.gen_method(&method.block);
        let actix_web_cb = info.gen_actix_web_cb(struct_name);

        quote! {
            #method_tokens

            #actix_web_cb
        }
    }

    fn parse_controller_method_info(method: &ImplItemMethod) -> ControllerMethodInfo {
        // Parse attrs
        let mut http_method: Option<String> = None;
        let mut url: Option<String> = None;
        let mut openapi: Option<MethodOpenapiInfo> = None;
        for attr in &method.attrs {
            let http_method_attr = attr
                .path
                .segments
                .last()
                .expect(&format!("Invalid attr {}", quote! {#attr}))
                .ident
                .to_string();
            match http_method_attr.as_str() {
                "get" | "post" | "put" | "delete" => {
                    http_method = Some(http_method_attr.clone());
                    url = Some(match utils::parse_string_arg(&attr.tokens) {
                        Ok(s) => s,
                        Err(_) => abort! { attr.tokens,
                            "Syntax error on controller method";
                            note = "Syntax is #[{}(\"url\")]", http_method_attr;
                        },
                    });
                }
                "openapi" => {
                    openapi = Some(Self::parse_controller_method_openapi_info(attr));
                }
                _ => panic!("Invalid attr: {}", quote! {attr}),
            }
        }

        // Parse is_mut_self
        let is_mut_self = if method.sig.inputs.len() > 0 {
            match &method.sig.inputs[0] {
                FnArg::Receiver(rec) => {
                    if rec.reference.is_none() {
                        abort! { rec,
                            "First arg of controller method should be &self or &mut self"
                        };
                    } else {
                        rec.mutability.is_some()
                    }
                }
                arg @ _ => abort! { arg,
                    "First arg of controller method should be &self or &mut self"
                },
            }
        } else {
            abort! { method.sig,
                "Controller method must as least contain &self or &mut self"
            };
        };

        // Parse args
        let arg_tokens: Vec<_> = method.sig.inputs.iter().collect();
        let args: Vec<ControllerMethodArg> = (&arg_tokens[1..])
            .iter()
            .map(|arg| match arg {
                FnArg::Typed(pat_type) => Self::parse_controller_method_arg(pat_type),
                _ => panic!("Expect a typed arg, received '{}'", quote! {#arg}),
            })
            .collect();

        // Validate args
        if args
            .iter()
            .filter(|arg| {
                if let ControllerMethodArg::Body { .. } = arg {
                    true
                } else {
                    false
                }
            })
            .collect::<Vec<&ControllerMethodArg>>()
            .len()
            > 1
        {
            abort! { method.sig.inputs,
                "Found more than 1 body"
            }
        }

        // Parse output
        let output_type = match &method.sig.output {
            ReturnType::Default => quote! {()}.to_string(),
            ReturnType::Type(_, t) => quote! {#t}.to_string(),
        };

        ControllerMethodInfo {
            fn_name: method.sig.ident.to_string(),
            is_async: method.sig.asyncness.is_some(),
            is_mut_self,
            args,
            output_type,
            method: http_method.expect("Method is empty"),
            url: url.expect("Url is empty"),
            openapi,
        }
    }

    fn parse_controller_method_arg(arg: &PatType) -> ControllerMethodArg {
        let pat = &arg.pat;
        let ty = &arg.ty;

        // Check attr count
        if arg.attrs.len() != 1 {
            abort! { arg,
                "Attr count must be 1";
                help = "Consider use #[body] {}: {}", quote! {#pat}, quote! {#ty};
            }
        }

        // Get attr
        let attr = &arg.attrs[0];
        let attr_name = attr
            .path
            .segments
            .last()
            .expect(&format!("Invalid attr of arg '{}'", quote! {#arg}))
            .ident
            .to_string();
        match attr_name.as_str() {
            "param" => ControllerMethodArg::Param {
                name: quote! {#pat}.to_string(),
                ty: quote! {#ty}.to_string(),
            },
            "body" => ControllerMethodArg::Body {
                name: quote! {#pat}.to_string(),
                ty: quote! {#ty}.to_string(),
            },
            "query" => ControllerMethodArg::Query {
                name: quote! {#pat}.to_string(),
                ty: quote! {#ty}.to_string(),
            },
            "raw" => ControllerMethodArg::Raw {
                name: quote! {#pat}.to_string(),
                ty: quote! {#ty}.to_string(),
            },
            _ => abort! { attr,
                "Invalid attr"
            },
        }
    }

    fn parse_controller_method_openapi_info(attr: &Attribute) -> MethodOpenapiInfo {
        const PARSE_ERR_STR: &'static str = "Parse failed, syntax is #[openapi(field [= value])]";
        const ARG_HELP: &'static str = r#"Syntax is openapi(bearer_auth | security = serde_json::josn!({}) | tags = ["tag1", "tag2", ...] | summary = "STRING" | parameters = serde_json::json!({}) | requestBody = serde_json::json!({}) | responses = serde_json::json!({})"#;

        // Generate function call tokens: rorm(xxx)
        let path = attr.path.clone();
        let toks = attr.tokens.clone();
        let call_toks = quote::quote! {#path #toks};

        let args = if let Ok(call) = syn::parse2::<syn::ExprCall>(call_toks) {
            call.args
        } else {
            abort!(attr.tokens, PARSE_ERR_STR);
        };

        let mut bearer_auth = false;
        let mut security = None;
        let mut tags = None;
        let mut summary = None;
        let mut parameters = None;
        let mut request_body = None;
        let mut responses = None;

        // Parse args
        for expr in &args {
            match expr {
                Expr::Path(p) => {
                    let field_name = p.to_token_stream().to_string();
                    match field_name.as_str() {
                        // Parse bearer_auth
                        "bearer_auth" => {
                            bearer_auth = true;
                        }

                        // Error
                        _ => abort!(expr, "Syntax error while decode path"; help = ARG_HELP),
                    }
                }
                Expr::Assign(assign) => {
                    let field_name = assign.left.to_token_stream().to_string();
                    match field_name.as_str() {
                        // Parse security = serde_json::josn!({})
                        "security" => {
                            security = Some(assign.right.to_token_stream());
                        }

                        // Parse tags = ["tag1", "tag2", ...]
                        "tags" => {
                            tags = Some(assign.right.to_token_stream());
                        }

                        // Parse summary = "STRING"
                        "summary" => {
                            summary = Some(assign.right.to_token_stream());
                        }

                        // Parse parameters = serde_json::json!({})
                        "parameters" => {
                            parameters = Some(assign.right.to_token_stream());
                        }

                        // Parse request_body = serde_json::json!({})
                        "request_body" => {
                            request_body = Some(assign.right.to_token_stream());
                        }

                        // Parse responses = serde_json::json!({})
                        "responses" => {
                            responses = Some(assign.right.to_token_stream());
                        }

                        // Error
                        _ => abort!(expr, "Syntax error while decode assign"; help = ARG_HELP),
                    }
                }
                _ => abort!(expr, "Syntax error while match expr"; help = ARG_HELP),
            }
        }

        MethodOpenapiInfo {
            bearer_auth,
            security,
            tags,
            summary,
            parameters,
            request_body,
            responses,
        }
    }
}
