use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use syn::{Data, DeriveInput, Expr, Pat};

#[derive(Debug)]
struct FieldDefaultFn {
    arg_types: Vec<syn::Type>,
    closure: syn::ExprClosure,
}

#[derive(Debug)]
struct Field {
    r#type: String,
    default: Option<String>, // Default code
    default_fn: Option<FieldDefaultFn>,
}

#[derive(Debug)]
pub struct Provider {
    name: String,
    fields: HashMap<String, Field>,
    on_module_init: Option<String>,
}

impl Provider {
    pub fn parse(item: DeriveInput) -> Self {
        // Validate input token
        let struct_data = match item.data {
            Data::Struct(s) => s,
            _ => panic!("Expect a struct"),
        };

        // Get fields
        let fields = Self::parse_fields(&item.ident.to_string(), &struct_data.fields);

        // Get on_module_init function name
        let on_module_init = Self::get_on_module_init_fn(&item.attrs);

        Self {
            name: item.ident.to_string(),
            fields,
            on_module_init,
        }
    }

    pub fn gen(&self) -> TokenStream {
        let provider_name_id = format_ident!("{}", self.name);
        let fields: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|(name, field)| self.gen_field(name, field))
            .collect();
        let on_module_init_expr = if let Some(func) = &self.on_module_init {
            let func = format_ident!("{}", func);
            quote! {
                self.#func()
                    .await
                    .map_err(|e| rnest::Error::User(format!("Init provider '{}' error: {}", std::any::type_name::<Self>(), e)))?;
            }
        } else {
            quote! {}
        };

        quote! {
            #[async_trait::async_trait]
            impl rnest::Provider<Self> for #provider_name_id {
                async fn register(scoped_di: rnest::ScopedDiGuard) -> rnest::Result<Self> {
                    Ok(Self {
                        #(#fields,)*
                    })
                }
            }

            // TODO: Use trait
            impl #provider_name_id {
                pub async fn __rnest_init(&mut self) -> rnest::Result<()> {
                    #on_module_init_expr
                    log::info!("{} initialized", stringify!(#provider_name_id));

                    Ok(())
                }
            }
        }
    }

    fn parse_fields(struct_name: &String, input_fields: &syn::Fields) -> HashMap<String, Field> {
        let mut fields: HashMap<String, Field> = HashMap::new();

        for field in input_fields {
            let name = field
                .ident
                .as_ref()
                .expect(&format!("Field has no name in struct '{}'", struct_name,))
                .to_string();
            let ty = &field.ty;
            let field_type = quote! {#ty}.to_string();
            let default = Self::get_default(field);
            let default_fn = Self::get_default_fn(field);

            fields.insert(
                name,
                Field {
                    r#type: field_type,
                    default,
                    default_fn,
                },
            );
        }

        fields
    }

    fn get_on_module_init_fn(attrs: &Vec<syn::Attribute>) -> Option<String> {
        for attr in attrs {
            let path = &attr.path;
            let path_str = quote! {#path}.to_string();
            match path_str.as_str() {
                "on_module_init" => {
                    let expr = syn::parse2::<syn::ExprParen>(attr.tokens.clone())
                        .expect(&format!("Invalid on_module_init {}", attr.tokens));
                    match *expr.expr {
                        syn::Expr::Path(p) => return Some(quote! {#p}.to_string()),
                        e @ _ => {
                            panic!("Invalid on_module_init expr {}, expect a path", quote! {#e})
                        }
                    }
                }
                _ => continue,
            }
        }

        None
    }

    fn get_default(field: &syn::Field) -> Option<String> {
        for attr in &field.attrs {
            if let Some(first) = attr.path.segments.first() {
                match first.ident.to_string().as_str() {
                    "default" => {
                        let tokens = &attr.tokens;
                        return Some(quote! {#tokens}.to_string());
                    }
                    _ => continue,
                }
            }
        }

        None
    }

    fn get_default_fn(field: &syn::Field) -> Option<FieldDefaultFn> {
        for attr in &field.attrs {
            if let Some(first) = attr.path.segments.first() {
                match first.ident.to_string().as_str() {
                    "default_fn" => {
                        // Parse closure
                        let expr = match syn::parse2::<syn::Expr>(attr.tokens.clone()) {
                            Ok(v) => v,
                            Err(e) => abort!(attr.tokens, "Expect expr: {}", e),
                        };

                        // Get paren
                        let paren = match expr {
                            Expr::Paren(v) => v,
                            _ => abort!(expr, "Expect paren"),
                        };

                        // Get closure
                        let closure = match paren.expr.as_ref() {
                            Expr::Closure(v) => v,
                            _ => abort!(paren, "Expect closure"),
                        };

                        // Get arg types
                        let arg_types = closure
                            .inputs
                            .iter()
                            .map(|pat| {
                                // Get pat type
                                let pat_ty = match pat {
                                    Pat::Type(v) => v,
                                    _ => abort!(pat, "Expect type"),
                                };

                                // Get type
                                let ty = pat_ty.ty.as_ref().clone();

                                ty
                            })
                            .collect::<Vec<_>>();

                        return Some(FieldDefaultFn {
                            arg_types,
                            closure: closure.clone(),
                        });
                    }
                    _ => continue,
                }
            }
        }

        None
    }

    fn gen_field(&self, name: &String, field: &Field) -> TokenStream {
        let name_id = format_ident!("{}", name);
        let field_type = &field.r#type;

        if let Some(default) = &field.default {
            let default: TokenStream = default.parse().unwrap();
            quote! {
                #name_id: #default
            }
        } else if let Some(default_fn) = &field.default_fn {
            let closure = &default_fn.closure;
            let args = default_fn
                .arg_types
                .iter()
                .map(|arg_ty| {
                    let inject_name = arg_ty.to_token_stream().to_string();
                    quote! {
                        scoped_di.inject::<#arg_ty>(#inject_name).await?
                    }
                })
                .collect::<Vec<_>>();

            quote! {
                #name_id: (#closure)(#(#args),*).await?
            }
        } else {
            quote! {
                #name_id: scoped_di.inject(#field_type).await?
            }
        }
    }
}

fn _print<S: syn_serde::Syn>(s: &S) {
    println!("+++ {}", syn_serde::json::to_string_pretty(s));
}
