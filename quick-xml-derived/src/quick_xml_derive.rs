use std::collections::{HashSet, HashMap};
use std::hash::{Hash, Hasher};

use proc_macro2::{TokenStream};
use quote::{quote};
use syn::punctuated::Punctuated;
use syn::{Data, DataStruct, Fields, DeriveInput, Ident, Token, LitStr, Attribute};
use syn::parse::{Parse, ParseStream};
use phf::phf_map;

mod kw {
    use syn::custom_keyword;
    custom_keyword!(xmlns);
    custom_keyword!(pre);
}

#[derive(Debug)]
pub struct QuickXmlNamespace {
    prefix: Option<String>,
    uri: String,
}

pub fn impl_quick_xml_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    eprintln!("THIS IS A TEST!!!!!");
    // Same as before
    let fields = match input.data {
        Data::Struct(DataStruct { fields: Fields::Named(fields), .. }) => fields.named,
        _ => panic!("this derive macro only works on structs with named fields"),
    };

    //input.attrs.iter().filter(|attr| attr.path.is_ident("qxml")).collect::<Vec<_>>();
    let declaration_attrs = input.attrs
        .iter()
        .filter(|attr| attr.path.is_ident("qxml"))
        .collect::<Vec<_>>();
    let mut declaration_meta = QXmlDeclarationMeta::default();
    for attr in declaration_attrs {
        let to_merge = attr.parse_args_with(QXmlDeclarationMeta::parse)?;
        declaration_meta.merge(to_merge)?;
    }
        // .try_fold(QXmlDeclarationMeta::default(), |meta, attr| {
        //     let to_merge = attr.parse_args_with(QXmlDeclarationMeta::parse)?;
        //     meta.merge(&to_merge)?;
        //     Ok(meta)
        // })?;
    
    let field_metas = fields
        .into_iter()
        .map(|f| {
            let field_attrs = f
                .attrs
                .iter()
                .filter(|attr| attr.path.is_ident("qxml"))
                .collect::<Vec<_>>();
            let mut field_meta = QXmlFieldMeta::default();
            for attr in field_attrs {
                let to_merge = attr.parse_args_with(QXmlFieldMeta::parse)?;
                field_meta.merge(to_merge)?;
            }
            
                // .try_fold(QXmlFieldMeta::default(), |meta, attr| {
                //     let to_merge = attr.parse_args_with(QXmlFieldMeta::parse)?;
                //     meta.merge(to_merge)
                // })?;
            field_meta.identifier = f.ident;
            Ok(field_meta)
            })
        .collect::<syn::Result<Vec<QXmlFieldMeta>>>()?;

    let mut block_meta = QXmlInsideBlockMeta::default();
    for field_meta in field_metas {
        let field_ident = match field_meta.identifier {
            Some(ident) => ident,
            None => continue
        };
        let elem_prefix = match field_meta.element_prefix {
            Some(elem) => elem,
            None => continue
        };
        block_meta.field_prefix_collection.push((field_ident, elem_prefix.prefix));
    }
            
        //     let visibility = meta.vis.unwrap_or_else(|| parse_quote! { pub });
        //     let method_name = meta.name.unwrap_or_else(|| f.ident.clone().expect("a named field"));
        //     let field_name = f.ident;
        //     let field_ty = f.ty;

        //     Ok(quote! {
        //         #visibility fn #method_name(&self) -> &#field_ty {
        //             &self.#field_name
        //         }
        //     })
        // })
        // .collect::<syn::Result<TokenStream>>()?;

    let st_name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #st_name #ty_generics #where_clause {
            fn namespace_declarations() -> &'static [(&'static str, &'static str)] {
                &[("C", "http://asdfasdf"), ("B", "asdf"), ("", "")]
            }

            fn identifier_prefix_map() -> phf::Map<&'static str, &'static str> {
                phf_map! {
                    "foo" => "C", 
                    "bar" => "B",
                    "jax" => "B"
                }
            }
        }
    })
    // fn identifier_prefix_map -> HashMap<String, String> {
    //     HashMap::from([("foo", "C"), ("bar", "B"), ("jax", "B")])
    // }
}

fn namespace_declarations() -> &'static [(&'static str, &'static str)] {
    &[("C", "http://asdfasdf"), ("B", "asdf"), ("", "")]
    //vec![("C", "http://asdfasdf"), ("B", "asdf"), ("", "")]
}

fn identifier_prefix_map() -> phf::Map<&'static str, &'static str> {
    phf_map! {
        "foo" => "C", 
        "bar" => "B",
        "jax" => "B"
    }
}

fn get_quick_xml_meta() -> &'static QuickXmlItemMeta {
    &QuickXmlItemMeta {
        namespace_declarations: &[("C", "http://asdfasdf"), ("B", "asdf"), ("", "")],
        identifier_prefix_map: phf_map! {
            "foo" => "C", 
            "bar" => "B",
            "jax" => "B"
        },
    }
}

struct QuickXmlItemMeta {
    namespace_declarations: &'static [(&'static str, &'static str)],
    identifier_prefix_map: phf::Map<&'static str, &'static str>
}
struct ByUnconsumedKeyword {}

impl ByUnconsumedKeyword {
    fn new() -> Self {
        ByUnconsumedKeyword {
        }
    }
}

#[derive(Clone)]
struct Namespace {
    xmlns_token: kw::xmlns,
    prefix: Option<Ident>,
    uri: LitStr
}

impl Hash for Namespace {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.prefix.hash(state);
    }
}

impl PartialEq for Namespace {
    fn eq(&self, other: &Self) -> bool {
        self.prefix == other.prefix
    }
}

impl Eq for Namespace {}

impl Namespace {
    fn is_default_namespace(&self) -> bool {
        self.prefix.is_none()
    }
}

impl Parse for Namespace {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut prefix: Option<Ident> = None;
        let xmlns_token: kw::xmlns = input.parse()?;
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![:]) {
            let _: Token![:] = input.parse()?;
            prefix = Some(input.parse::<Ident>()?);
        } 
        let _: Token![=] = input.parse()?;
        let uri = input.parse()?;
        Ok(Self {xmlns_token, prefix, uri})
    }
}
#[derive(Default)]
struct QXmlDeclarationMeta {
    declared_namespaces_set: HashSet<Namespace>,
    declared_namespaces: Vec<Namespace>
}

impl QXmlDeclarationMeta {
    fn merge(&mut self, other: QXmlDeclarationMeta) -> syn::Result<&mut Self> {
        for other_namespace in other.declared_namespaces {
            if let Some(this_namespace) = self.declared_namespaces_set.get(&other_namespace) {
                if other_namespace.is_default_namespace() {
                    let mut error = syn::Error::new_spanned(other_namespace.xmlns_token, "duplicate default namespace declaration, first here");
                    error.combine(syn::Error::new_spanned(this_namespace.xmlns_token, "second here"));
                    return Err(error);
                } else {
                    let mut error = syn::Error::new_spanned(other_namespace.prefix.as_ref().unwrap(), "duplicate namespace declaration, first one here");
                    error.combine(syn::Error::new_spanned(this_namespace.prefix.as_ref().unwrap(), "second here"));
                    return Err(error);
                }
            }
            self.declared_namespaces.push(other_namespace.clone());
            self.declared_namespaces_set.insert(other_namespace.clone());
        }
        Ok(self)
    }
}

impl Parse for QXmlDeclarationMeta {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut meta = Self::default();
        loop {
            if input.is_empty() {
                return Ok(meta);
            }
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::xmlns) { 
                
                let namespace: Namespace = input.parse()?;
                let declared_namespaces = Vec::from([namespace.clone()]);
                let declared_namespaces_set = HashSet::from([namespace]);
                let to_merge = Self { declared_namespaces, declared_namespaces_set };
                meta.merge(to_merge)?;
            } else {
                return Err(lookahead.error());
            }
        }
    }
}

#[derive(Clone)]
struct ElementPrefix {
    prefix_token: kw::pre,
    prefix: Ident,
}

impl Parse for ElementPrefix {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let prefix_token: kw::pre = input.parse()?;
        let _: Token![:] = input.parse()?;
        let prefix: Ident = input.parse()?;
        Ok(Self { prefix_token, prefix})
    }
}

#[derive(Default)]
struct QXmlFieldMeta {
    element_prefix: Option<ElementPrefix>,
    identifier: Option<Ident>
}

impl QXmlFieldMeta {
    fn merge(&mut self, other: QXmlFieldMeta) -> syn::Result<&mut Self> {
        match (self.element_prefix.as_ref(), other.element_prefix) {
            (None, None) => Ok(self),
            (Some(_), None) => Ok(self),
            (None, Some(elem)) => {
                self.element_prefix = Some(elem);
                Ok(self)
            }
            (Some(this_elem), Some(other_elem)) => {
                let mut error = syn::Error::new_spanned(this_elem.prefix_token, "redundant prefix argument");
                error.combine(syn::Error::new_spanned(other_elem.prefix_token, "note: first one here"));
                Err(error)
            }
        }
    }
}

impl Parse for QXmlFieldMeta {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut meta = Self::default();
        loop {
            if input.is_empty() {
                return Ok(meta);
            }
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::pre) { 
                let element_prefix_unopt: ElementPrefix = input.parse()?;
                let element_prefix = Some(element_prefix_unopt);
                let to_merge = Self {identifier: None, element_prefix };
                meta.merge(to_merge)?;
            } else {
                return Err(lookahead.error());
            }
        }
    }
}

#[derive(Default)]
struct QXmlInsideBlockMeta {
    field_prefix_collection: Vec<(Ident, Ident)>
}
