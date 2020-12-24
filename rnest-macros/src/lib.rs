#[macro_use]
mod utils;

mod controller;
mod module;
mod provider;

use controller::Controller;
use module::Module;
use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use provider::Provider;
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

    gen.into()
}

#[proc_macro_derive(Provider, attributes(default, on_module_init))]
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

    let stream = Controller::parse(attr.into(), imp);
    stream.into()
}

#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr: proc_macro2::TokenStream = attr.into();
    let item: proc_macro2::TokenStream = item.into();

    (quote::quote! {
        #[actix_web::main#attr]
        #item
    })
    .into()
}
