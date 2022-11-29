#[macro_use]
mod utils;

mod controller;
mod module;
mod openapi;
mod provider;

use controller::Controller;
use module::Module;
use openapi::Openapi;
use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use provider::Provider;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Item};

#[proc_macro_derive(
    Module,
    attributes(imports, controllers, providers, exports, on_module_init)
)]
#[proc_macro_error]
pub fn derive_module(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item);
    let module = Module::parse(input);
    let gen = module.gen();
    let openapi = module.gen_openapi3();

    (quote! {
        #gen
        #openapi
    })
    .into()
}

#[proc_macro_derive(Provider, attributes(default, default_fn, on_module_init))]
#[proc_macro_error]
pub fn derive_provider(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item);
    let provider = Provider::parse(input);
    let gen = provider.gen();

    gen.into()
}

#[proc_macro_attribute]
#[proc_macro_error]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item: Item = parse_macro_input!(item);
    let imp = match item {
        Item::Impl(imp) => imp,
        i @ _ => abort! { i,
            "#[controller] only used for impl block"
        },
    };

    let stream = Controller::parse(attr.clone().into(), imp.clone());
    let openapi = Controller::gen_openapi3(attr.into(), imp);

    (quote! {
        #stream
        #openapi
    })
    .into()
}

#[proc_macro_derive(OpenApiSchema, attributes(openapi))]
#[proc_macro_error]
pub fn derive_openapi_schema(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item);
    let openapi = Openapi::parse(&input);
    let gen = openapi.gen();

    gen.into()
}
