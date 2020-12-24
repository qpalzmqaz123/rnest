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
                    let list: syn::ExprTuple = parse2! { attr.tokens,
                        "Syntax error on module imports";
                        note = "Syntax is #[imports(MODULE_A as TYPE_A, MODULEB as TYPE_B,)]";
                    };
                    for v in list.elems {
                        match v {
                            syn::Expr::Cast(e) => {
                                let module = e.expr;
                                let module_type = e.ty;
                                imports.insert(
                                    quote! {#module_type}.to_string(),
                                    quote! {#module}.to_string(),
                                );
                            }
                            _ => abort! { v,
                                "Expect a cast expr";
                                help = "Consider use '{} as Arc<RwLock<{}>>'", quote! {#v}, quote! {#v};
                            },
                        }
                    }
                }
                "controllers" => {
                    // TODO: Optimize
                    let list: syn::ExprTuple = parse2! { attr.tokens,
                        "Syntax error on module controllers";
                        note = "Syntax is #[controllers(CONTROLLER_A as TYPE_A, CONTROLLERB as TYPE_B,)]";
                    };
                    for v in list.elems {
                        match v {
                            syn::Expr::Cast(e) => {
                                let controller = e.expr;
                                let controller_type = e.ty;
                                controllers.insert(
                                    quote! {#controller_type}.to_string(),
                                    quote! {#controller}.to_string(),
                                );
                            }
                            _ => abort! { v,
                                "Expect a cast expr";
                                help = "Consider use '{} as Arc<RwLock<{}>>'", quote! {#v}, quote! {#v};
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
                                help = "Consider use '{} as Arc<RwLock<{}>>'", quote! {#v}, quote! {#v};
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
        let scope_fn = self.gen_scope();

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

                #scope_fn
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
                if di.contains(#module_name) {
                    return;
                }

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
                instance.__rnest_init();

                // Save module
                di.register_value(#module_name, Arc::new(RwLock::new(instance)));
            }
        }
    }

    fn gen_scope(&self) -> TokenStream {
        let import_scope_calls: Vec<TokenStream> = self
            .imports
            .iter()
            .map(|(_, name)| Self::gen_import_scope_call(name))
            .collect();
        let controller_scope_calls: Vec<TokenStream> = self
            .controllers
            .iter()
            .map(|(ty, name)| Self::gen_controller_scope_call(&self.name, name, ty))
            .collect();

        quote! {
            fn scope(di: &mut rnest::Di) -> actix_web::Scope {
                let scope = actix_web::web::scope("");

                #(#import_scope_calls)*

                #(#controller_scope_calls)*

                scope
            }
        }
    }

    fn gen_import_scope_call(module_name: &String) -> TokenStream {
        let module_token = format_ident!("{}", module_name);

        quote! {
            let scope = scope.service(<#module_token as rnest::Module>::scope(di));
        }
    }

    fn gen_controller_scope_call(
        module_name: &String,
        controller_name: &String,
        ty: &String,
    ) -> TokenStream {
        let controller_name_token = format_ident!("{}", controller_name);
        let type_token: TokenStream = ty.parse().expect(&format!("Parse type '{}' error", ty));

        quote! {
            let scope = scope.service(
                <#controller_name_token as rnest::Controller<#controller_name_token, _>>::scope(
                    <Self as rnest::Module>::scoped_di(di).inject::<_, #type_token>(#ty).expect(
                        &format!(
                            "Cannot inject controller '{}' from module '{}', please check if it is defined in provider",
                            #ty,
                            #module_name,
                        )
                    )
                )
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
            scoped_di.register_factory(
                #provider_type,
                |scoped_di| {
                    // Create provider instance
                    let mut instance: #provider_id = <#provider_id as rnest::Provider<#provider_id>>::register(scoped_di)?;

                    // Init provider
                    instance.__rnest_init();

                    // Create di instance
                    let mut di_instance: #provider_type_id = Arc::new(RwLock::new(instance));

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
