extern crate proc_macro;
mod quick_xml_derive;
use quick_xml_derive::impl_quick_xml_derive;

use proc_macro::{TokenStream};
use syn;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(QuickXml, attributes(qxml))]
pub fn quick_xml_derive(input: TokenStream) -> TokenStream {
    eprintln!("THIS IS A TEST!!!!!");
    let input = parse_macro_input!(input as DeriveInput);
    impl_quick_xml_derive(input).unwrap_or_else(syn::Error::into_compile_error).into()
}
