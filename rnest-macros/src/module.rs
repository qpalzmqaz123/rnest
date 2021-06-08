use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::DeriveInput;

#[derive(Debug)]
struct Provider {
    r#struct: String,
    export: bool,
}

#[derive(Debug)]
pub struct Module {
    name: String,                         // Module name
    imports: HashMap<String, String>,     // HashMap<Type, StructName>
    controllers: HashMap<String, String>, // HashMap<Type, StructName>
    providers: HashMap<String, Provider>,
    on_module_init: Option<String>,
}

impl Module {
    pub fn parse(item: DeriveInput) -> Self {
        let mut imports: HashMap<String, String> = HashMap::new();
        let mut controllers: HashMap<String, String> = HashMap::new();
        let mut providers: HashMap<String, Provider> = HashMap::new();
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
                                    .insert(format!("std::sync::Arc<{}>", module_str), module_str);
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
                                controllers.insert(
                                    format!("std::sync::Arc<{}>", controller_str),
                                    controller_str,
                                );
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
                                providers.insert(
                                    quote! {#provider_type}.to_string(),
                                    Provider {
                                        r#struct: quote! {#provider}.to_string(),
                                        export: false,
                                    },
                                );
                            }
                            _ => abort! { v,
                                "Expect a cast expr";
                                help = "Consider use '{} as Arc<{}Trait + Sync + Send>'", quote! {#v}, quote! {#v};
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
                        match providers.get_mut(&key) {
                            Some(p) => p.export = true,
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
            quote! {self.#func();}
        } else {
            quote! {}
        };

        quote! {
            impl rnest::Module for #module_name {
                #scoped_di_fn

                #import_fn

                #configure_actix_web_fn
            }

            impl #module_name {
                fn __rnest_init(&mut self) {
                    #on_module_init_expr
                    log::info!("{} initialized", stringify!(#module_name));
                }
            }
        }
    }

    fn gen_scoped_di(&self) -> TokenStream {
        let module_name = &self.name;
        let import_modules = self.imports.values().collect::<Vec<&String>>();

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
            .values()
            .map(|m| format_ident!("{}", m))
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
            fn import(di: &mut rnest::Di) {
                let module_name = #module_name;

                if di.contains(module_name) {
                    return;
                }

                log::trace!("Import module: '{}'", module_name);

                // Import submodule
                #(<#import_module_ids as rnest::Module>::import(di);)*

                // Register providers
                let mut scoped_di = Self::scoped_di(di);
                #(#provider_factories;)*

                // Register controllers
                #(#controller_factories;)*

                // Init providers
                #(#provider_injectors;)*

                // Init controllers
                #(#controller_injectors)*

                // Create instance
                let mut instance = Self {};

                // Init module
                log::trace!("Init module: '{}'", module_name);
                instance.__rnest_init();

                // Save module
                di.register_value(#module_name, std::sync::Arc::new(instance));
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
            fn configure_actix_web(di: &mut rnest::Di, cfg: &mut rnest::actix_web::web::ServiceConfig) {
                #(#import_actix_web_configure_calls)*

                #(#controller_configure_actix_web_calls)*
            }
        }
    }

    fn gen_import_actix_web_configure_call(module_name: &String) -> TokenStream {
        let module_token = format_ident!("{}", module_name);

        quote! {
            <#module_token as rnest::Module>::configure_actix_web(di, cfg);
        }
    }

    fn gen_controller_configure_actix_web_call(
        module_name: &String,
        controller_name: &String,
        ty: &String,
    ) -> TokenStream {
        let controller_name_token = format_ident!("{}", controller_name);
        let type_token: TokenStream = ty.parse().expect(&format!("Parse type '{}' error", ty));

        quote! {
            <#controller_name_token as rnest::Controller<#controller_name_token, _>>::configure_actix_web(
                <Self as rnest::Module>::scoped_di(di).inject::<_, #type_token>(#ty).expect(
                    &format!(
                        "Cannot inject controller '{}' from module '{}', please check if it is defined in provider",
                        #ty,
                        #module_name,
                    )
                ),
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
            log::trace!("Register provider factory to '{}', name: '{}', type: '{}', export: {}", module_name, #provider_type, stringify!(#provider_type_id), #export);
            scoped_di.register_factory(
                #provider_type,
                |scoped_di| {
                    // Create provider instance
                    let mut instance: #provider_id = <#provider_id as rnest::Provider<#provider_id>>::register(scoped_di)?;

                    // Init provider
                    instance.__rnest_init();

                    // Create di instance
                    let mut di_instance: #provider_type_id = std::sync::Arc::new(instance);

                    Ok(di_instance)
                },
                #export,
            );
        }
    }

    fn gen_provider_injector(&self, r#type: &String, module: &String) -> TokenStream {
        let provider_type = r#type;
        let provider_type_id: TokenStream = r#type.parse().unwrap();

        quote! {
            log::trace!("Init provider in '{}', key: '{}', type: '{}'", module_name, stringify!(#provider_type_id), #provider_type);
            scoped_di.inject::<_, #provider_type_id>(#provider_type).expect(
                &format!(
                    "Cannot inject '{}' from module '{}', please check if it is defined in provider or imported from submodule",
                    #provider_type,
                    #module,
                )
            );
        }
    }
}

fn _print<S: syn_serde::Syn>(s: &S) {
    println!("+++ {}", syn_serde::json::to_string_pretty(s));
}
