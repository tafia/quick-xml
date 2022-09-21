extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn;

#[proc_macro_derive(QuickXml, attributes(xmlns, xmlpre))]
pub fn quick_xml_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    impl_quick_xml_derive(&ast)
}

fn impl_quick_xml_derive(ast: &syn::ItemStruct) -> TokenStream {

    let attrs = &ast.attrs;
    let xmlns_attrs = attrs.iter().filter(|a| a.path.is_ident("xmlns")).collect();
    let xmlpre_attrs = attrs.iter().filter(|a| a.path.is_ident("xmlpre")).collect();
    if xmlns_attrs.len() + xmlpre_attrs.len() == 0 {
        ast.
    }
    for attr in attrs.iter().filter(|a| a.path.is_ident("xmlns")) {
        let mut attr_token_trees = attr.clone().tokens.into_iter();
        let first_o: Option<TokenTree> = attr_token_trees.next();
        let second_o: Option<TokenTree> = first_o.as_ref().and_then(|_| attr_token_trees.next());
        let third_o: Option<TokenTree> = second_o.as_ref().and_then(|_| attr_token_trees.next());
        let fourth_o: Option<TokenTree> = third_o.as_ref().and_then(|_| attr_token_trees.next());
        let fifth_o: Option<TokenTree> = fourth_o.as_ref().and_then(|_| attr_token_trees.next());
        if let Option<TokenTree>(fifth_o) = fifth {
            syn::Error::new_spanned(fifth.into_token_stream(), "xmlns should on")
        }

        let first = match first_o {
            Some(TokenTree::Punct(first)) => first,
            _ => {
                panic!("xmlns improperly formatted");
            }
        };
        
        let mut prefix: Option<String> = None;
        let mut uri: Option<String> = None;

        if first.to_string() == ":" {

            let second = match second_o {
                Some(TokenTree::Ident(ref second)) => second,
                _ => {
                    panic!("xmlns improperly formatted");
                }
            };
            prefix = Some(second.to_string());
            let third = match third_o {
                Some(TokenTree::Punct(ref third)) => third,
                _ => {
                    panic!("xmlns improperly formatted");
                }
            };
            if third.to_string() != "=" {
                panic!("xmlns improperly formatted");
            }
            let fourth = match fourth_o {
                Some(TokenTree::Literal(ref fourth)) => fourth,
                _ => {
                    panic!("xmlns improperly formatted");
                }
            };
            uri = Some(fourth.to_string());
        }
        
        if first.to_string() == "=" {
            if third_o.is_some() {
                panic!("xmlns improperly formatted");
            }
            let second = match second_o {
                Some(TokenTree::Literal(ref second)) => second,
                _ => {
                    panic!("xmlns improperly formatted");
                }
            };
            uri = Some(second.to_string());
        }
    }

    let name = &ast.ident;
    let gen = quote! {
        impl HelloMacro for #name {
            fn hello_macro() {
                println!(
                    "Hello, Macro! My name is {}!",
                    stringify!(#name)
                );
            }
        }
    };
    gen.into()
}
