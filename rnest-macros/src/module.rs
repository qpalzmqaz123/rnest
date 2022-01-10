use proc_macro2::{Ident, TokenStream};
use proc_macro_error::abort;
use quote::{format_ident, quote};
use syn::DeriveInput;

#[derive(Debug)]
struct Provider {
    r#struct: String,
    export: bool,
}

#[derive(Debug)]
pub struct Module {
    name: String,                       // Module name
    imports: Vec<(String, String)>,     // Vec<(Type, StructName)>
    controllers: Vec<(String, String)>, // Vec<(Type, StructName)>
    providers: Vec<(String, Provider)>,
    on_module_init: Option<String>,
}

impl Module {
    pub fn parse(item: DeriveInput) -> Self {
        let mut imports: Vec<(String, String)> = Vec::new();
        let mut controllers: Vec<(String, String)> = Vec::new();
        let mut providers: Vec<(String, Provider)> = Vec::new();
        let mut on_module_init: Option<String> = None;

        for attr in item.attrs {
            match attr
                .path
                .segments
                .first()
                .unwrap()
                .ident
                .to_string()
                .as_str()
            {
                "imports" => {
                    let tokens = attr.tokens;
                    let tmp_stream = quote! { f #tokens };
                    let list: syn::ExprCall = parse2! { tmp_stream,
                        "Syntax error on module imports";
                        note = "Syntax is #[imports(MODULE_A, MODULEB,)]";
                    };
                    for v in list.args {
                        match v {
                            syn::Expr::Path(p) => {
                                let module = p.path.segments.last().unwrap();
                                let module_str = quote! {#module}.to_string();
                                imports
                                    .push((format!("std::sync::Arc<{}>", module_str), module_str));
                            }
                            _ => abort! { v,
                                "Expect a path str";
                                help = "Consider use 'XXXModule'";
                            },
                        }
                    }
                }
                "controllers" => {
                    let tokens = attr.tokens;
                    let tmp_stream = quote! { f #tokens };
                    let list: syn::ExprCall = parse2! { tmp_stream,
                        "Syntax error on module controllers";
                        note = "Syntax is #[controllers(CONTROLLER_A, CONTROLLERB,)]";
                    };
                    for v in list.args {
                        match v {
                            syn::Expr::Path(p) => {
                                let controller = p.path.segments.last().unwrap();
                                let controller_str = quote! {#controller}.to_string();
                                controllers.push((
                                    format!("std::sync::Arc<{}>", controller_str),
                                    controller_str,
                                ));
                            }
                            _ => abort! { v,
                                "Expect a path str";
                                help = "Consider use 'XXXController'";
                            },
                        }
                    }
                }
                "providers" => {
                    let list: syn::ExprTuple = parse2! { attr.tokens,
                        "Syntax error on module providers";
                        note = "Syntax is #[providers(PROVIDER_A as TYPE_A, PROVIDERB as TYPE_B,)]";
                    };
                    for v in list.elems {
                        match v {
                            syn::Expr::Cast(e) => {
                                let provider = e.expr;
                                let provider_type = e.ty;
                                providers.push((
                                    quote! {#provider_type}.to_string(),
                                    Provider {
                                        r#struct: quote! {#provider}.to_string(),
                                        export: false,
                                    },
                                ));
                            }
                            _ => abort! { v,
                                "Expect a cast expr";
                                help = "Consider use '{} as Arc<{}Trait>'", quote! {#v}, quote! {#v};
                            },
                        }
                    }
                }
                "exports" => {
                    let list: syn::TypeTuple = parse2! { attr.tokens,
                        "Syntax error on module exports";
                        note = "Syntax is #[exports(TYPE_A, TYPE_B,)]";
                    };
                    for v in list.elems {
                        let key = quote! {#v}.to_string();
                        // TODO: Optimize
                        let prod = providers.iter_mut().find(|(name, _)| name == &key);
                        match prod {
                            Some((_, p)) => p.export = true,
                            None => abort! { v,
                                "The exported type '{}' cannot be found in the providers", &key
                            },
                        };
                    }
                }
                // TODO: Cleanup
                "on_module_init" => {
                    let expr: syn::ExprParen = parse2! { attr.tokens,
                        "Syntax error on on_module_init";
                        note = "Syntax is #[on_module_init(INIT_FN)], INIT_FN is like fn init(&mut self) {}";
                    };
                    match &*expr.expr {
                        syn::Expr::Path(p) => match p.path.segments.len() {
                            0 => abort! { p,
                                "Function name is required"
                            },
                            1 => on_module_init = Some(quote! {#p}.to_string()),
                            _ => abort! { p,
                                "Invalid path";
                                help = "Consider remove '::'";
                            },
                        },
                        _ => abort! { expr,
                            "Invalid on_module_init arguments";
                            note = "Argument must be a function name";
                        },
                    }
                }
                attr @ _ => abort! { attr,
                    "Invalid attr: {}", attr
                },
            }
        }

        Self {
            name: item.ident.to_string(),
            imports,
            controllers,
            providers,
            on_module_init,
        }
    }

    pub fn gen(&self) -> TokenStream {
        let module_name = format_ident!("{}", self.name);
        let scoped_di_fn = self.gen_scoped_di();
        let import_fn = self.gen_import();
        let configure_actix_web_fn = self.gen_configure_actix_web_fn();

        // TODO: Cleanup
        let on_module_init_expr = if let Some(func) = &self.on_module_init {
            let func = format_ident!("{}", func);
            quote! {
                self.#func()
                    .await
                    .map_err(|e| rnest::Error::User(format!("Init module '{}' error: {}", std::any::type_name::<Self>(), e)))?;
            }
        } else {
            quote! {}
        };

        quote! {
            #[async_trait::async_trait]
            impl rnest::Module for #module_name {
                #scoped_di_fn

                #import_fn

                #configure_actix_web_fn
            }

            impl #module_name {
                async fn __rnest_init(&mut self) -> rnest::Result<()> {
                    #on_module_init_expr
                    log::info!("{} initialized", stringify!(#module_name));

                    Ok(())
                }
            }
        }
    }

    pub fn gen_openapi3(&self) -> TokenStream {
        let module_name = format_ident!("{}", self.name);
        let controllers: Vec<Ident> = self
            .controllers
            .iter()
            .map(|(_, ctrl)| format_ident!("{}", ctrl))
            .collect();
        let imports: Vec<Ident> = self
            .imports
            .iter()
            .map(|m| format_ident!("{}", m.1))
            .collect();

        quote! {
            impl #module_name {
                pub fn __rnest_gen_openapi3_spec(cache: &mut std::collections::HashMap<String, rnest::JsonValue>) {
                    let name = stringify!(#module_name).to_string();

                    // Check if spec already in cache
                    if cache.contains_key(&name) {
                        return;
                    }

                    // Generate from imports
                    #(#imports::__rnest_gen_openapi3_spec(cache);)*

                    // Generate self
                    cache.insert(name, Self::__rnest_get_openapi3_spec_self());
                }

                fn __rnest_get_openapi3_spec_self() -> rnest::JsonValue {
                    let mut paths = rnest::json!({});
                    let mut obj = paths.as_object_mut().unwrap();
                    #(obj.extend(#controllers::__rnest_get_openapi3_spec().as_object().unwrap().clone());)*

                    paths
                }
            }
        }
    }

    fn gen_scoped_di(&self) -> TokenStream {
        let module_name = &self.name;
        let import_modules = self
            .imports
            .iter()
            .map(|(_, v)| v)
            .collect::<Vec<&String>>();

        quote! {
            fn scoped_di(di: &mut rnest::Di) -> rnest::ScopedDi {
                di.scope(#module_name, &[#(#import_modules),*])
            }
        }
    }

    fn gen_import(&self) -> TokenStream {
        let module_name = &self.name;
        let import_module_ids = self
            .imports
            .iter()
            .map(|(_, m)| format_ident!("{}", m))
            .collect::<Vec<proc_macro2::Ident>>();
        let provider_factories: Vec<TokenStream> = self
            .providers
            .iter()
            .map(|(r#type, value)| self.gen_provider_factory(&value.r#struct, r#type, value.export))
            .collect();
        let controller_factories: Vec<TokenStream> = self
            .controllers
            .iter()
            .map(|(r#type, struct_name)| self.gen_provider_factory(&struct_name, r#type, false))
            .collect();
        let provider_injectors: Vec<TokenStream> = self
            .providers
            .iter()
            .map(|(r#type, _)| self.gen_provider_injector(r#type, module_name))
            .collect();
        let controller_injectors: Vec<TokenStream> = self
            .controllers
            .iter()
            .map(|(r#type, _)| self.gen_provider_injector(r#type, module_name))
            .collect();

        quote! {
            async fn import(di: &mut rnest::Di) -> rnest::Result<()> {
                let module_name = #module_name;

                if di.contains(module_name)? {
                    return Ok(());
                }

                log::trace!("Import module: '{}'", module_name);

                // Import submodule
                log::trace!("Import submodules of module: '{}'", module_name);
                #(<#import_module_ids as rnest::Module>::import(di).await?;)*

                // Register providers
                log::trace!("Register providers of module: '{}'", module_name);
                let mut scoped_di = Self::scoped_di(di);
                #(#provider_factories;)*

                // Register controllers
                log::trace!("Register controllers of module: '{}'", module_name);
                #(#controller_factories;)*

                // Init providers
                log::trace!("Init providers of module: '{}'", module_name);
                #(#provider_injectors;)*

                // Init controllers
                log::trace!("Init controllers of module: '{}'", module_name);
                #(#controller_injectors)*

                // Create instance
                let mut instance = Self {};

                // Init module
                log::trace!("Init module: '{}'", module_name);
                instance.__rnest_init().await?;

                // Save module
                di.register_value(#module_name, std::sync::Arc::new(instance))?;

                Ok(())
            }
        }
    }

    fn gen_configure_actix_web_fn(&self) -> TokenStream {
        let import_actix_web_configure_calls: Vec<TokenStream> = self
            .imports
            .iter()
            .map(|(_, name)| Self::gen_import_actix_web_configure_call(name))
            .collect();
        let controller_configure_actix_web_calls: Vec<TokenStream> = self
            .controllers
            .iter()
            .map(|(ty, name)| Self::gen_controller_configure_actix_web_call(&self.name, name, ty))
            .collect();

        quote! {
            fn configure_actix_web(di: &mut rnest::Di, cfg: &mut rnest::actix_web::web::ServiceConfig) -> rnest::Result<()> {
                #(#import_actix_web_configure_calls)*

                #(#controller_configure_actix_web_calls)*

                Ok(())
            }
        }
    }

    fn gen_import_actix_web_configure_call(module_name: &String) -> TokenStream {
        let module_token = format_ident!("{}", module_name);

        quote! {
            <#module_token as rnest::Module>::configure_actix_web(di, cfg)?;
        }
    }

    fn gen_controller_configure_actix_web_call(
        _module_name: &String,
        controller_name: &String,
        ty: &String,
    ) -> TokenStream {
        let controller_name_token = format_ident!("{}", controller_name);
        let type_token: TokenStream = ty.parse().expect(&format!("Parse type '{}' error", ty));

        quote! {
            <#controller_name_token as rnest::Controller<#controller_name_token, _>>::configure_actix_web(
                // Direct inject value use sync code
                <Self as rnest::Module>::scoped_di(di).inject_value::<#type_token>(#ty)?,
                cfg,
            );
        }
    }

    fn gen_provider_factory(
        &self,
        provider_name: &String,
        r#type: &String,
        export: bool,
    ) -> TokenStream {
        let provider_id = format_ident!("{}", provider_name);
        let provider_type = r#type;
        let provider_type_id: TokenStream = r#type.parse().unwrap();

        quote! {
            log::trace!("Register provider factory to '{}', name: '{}', type: '{}', export: {}", module_name, stringify!(#provider_id), stringify!(#provider_type_id), #export);
            scoped_di.register_factory(
                #provider_type,
                |scoped_di| async move {
                    // Create provider instance
                    let mut instance: #provider_id = <#provider_id as rnest::Provider<#provider_id>>::register(scoped_di).await?;

                    // Init provider
                    instance.__rnest_init().await?;

                    // Create di instance
                    let mut di_instance: #provider_type_id = std::sync::Arc::new(instance);

                    Ok(di_instance)
                },
                #export,
            )?;
        }
    }

    fn gen_provider_injector(&self, r#type: &String, _module: &String) -> TokenStream {
        let provider_type = r#type;
        let provider_type_id: TokenStream = r#type.parse().unwrap();

        quote! {
            log::trace!("Init provider in '{}', key: '{}', type: '{}'", module_name, stringify!(#provider_type_id), #provider_type);
            scoped_di.inject::<#provider_type_id>(#provider_type).await?;
        }
    }
}

fn _print<S: syn_serde::Syn>(s: &S) {
    println!("+++ {}", syn_serde::json::to_string_pretty(s));
}
