use proc_macro2::{Span, TokenStream};
use syn::{Data, Field, Ident, Lit, Meta, NestedMeta, Variant};

use super::RustlerAttr;

///
/// A parsing context struct.
///
/// `Context` holds information usable for different codegen modules.
///
pub(crate) struct Context<'a> {
    pub attrs: Vec<RustlerAttr>,
    pub ident: &'a proc_macro2::Ident,
    pub ident_with_lifetime: proc_macro2::TokenStream,
    pub variants: Option<Vec<&'a Variant>>,
    pub struct_fields: Option<Vec<&'a Field>>,
}

impl<'a> Context<'a> {
    pub fn from_ast(ast: &'a syn::DeriveInput) -> Self {
        let mut attrs: Vec<_> = ast
            .attrs
            .iter()
            .map(Context::get_rustler_attrs)
            .flatten()
            .collect();

        //
        // Default: generate encoder and decoder
        //
        if !Context::encode_decode_attr_set(&attrs) {
            attrs.push(RustlerAttr::Encode);
            attrs.push(RustlerAttr::Decode);
        }

        let has_lifetime = match ast.generics.lifetimes().count() {
            0 => false,
            1 => true,
            _ => panic!("Struct can only have one lifetime argument"),
        };

        let ident = &ast.ident;
        let ident_with_lifetime = if has_lifetime {
            quote! { #ident <'a> }
        } else {
            quote! { #ident }
        };

        let variants = match ast.data {
            Data::Enum(ref data_enum) => Some(data_enum.variants.iter().collect()),
            _ => None,
        };

        let struct_fields = match ast.data {
            Data::Struct(ref data_struct) => Some(data_struct.fields.iter().collect()),
            _ => None,
        };

        Self {
            attrs,
            ident,
            ident_with_lifetime,
            variants,
            struct_fields,
        }
    }

    pub fn encode(&self) -> bool {
        self.attrs.iter().any(|attr| match attr {
            RustlerAttr::Encode => true,
            _ => false,
        })
    }

    pub fn decode(&self) -> bool {
        self.attrs.iter().any(|attr| match attr {
            RustlerAttr::Decode => true,
            _ => false,
        })
    }

    pub fn field_atoms(&self) -> Option<Vec<TokenStream>> {
        self.struct_fields.as_ref().map(|struct_fields| {
            struct_fields
                .iter()
                .map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let ident_str = ident.to_string();

                    let atom_fun = Ident::new(&format!("atom_{}", ident_str), Span::call_site());

                    quote! {
                        #atom_fun = #ident_str,
                    }
                })
                .collect()
        })
    }

    fn encode_decode_attr_set(attrs: &[RustlerAttr]) -> bool {
        attrs.iter().any(|attr| match attr {
            RustlerAttr::Encode => true,
            RustlerAttr::Decode => true,
            _ => false,
        })
    }

    fn get_rustler_attrs(attr: &syn::Attribute) -> Vec<RustlerAttr> {
        attr.path
            .segments
            .iter()
            .filter_map(|segment| {
                let meta = attr.parse_meta().expect("can parse meta");
                match segment.ident.to_string().as_ref() {
                    "rustler" => Context::parse_rustler(&meta),
                    "tag" => Context::try_parse_tag(&meta),
                    "module" => Context::try_parse_module(&meta),
                    _ => None,
                }
            })
            .flatten()
            .collect()
    }

    fn parse_rustler(meta: &Meta) -> Option<Vec<RustlerAttr>> {
        if let Meta::List(ref list) = meta {
            return Some(
                list.nested
                    .iter()
                    .map(Context::parse_nested_rustler)
                    .collect(),
            );
        }

        panic!("Expected encode and/or decode in rustler attribute");
    }

    fn parse_nested_rustler(nested: &NestedMeta) -> RustlerAttr {
        if let NestedMeta::Meta(ref meta) = nested {
            if let Meta::Path(ref path) = meta {
                match path.segments[0].ident.to_string().as_ref() {
                    "encode" => return RustlerAttr::Encode,
                    "decode" => return RustlerAttr::Decode,
                    other => panic!("Unexpected literal {}", other),
                }
            }
        }

        panic!("Expected encode and/or decode in rustler attribute");
    }

    fn try_parse_tag(meta: &Meta) -> Option<Vec<RustlerAttr>> {
        if let Meta::NameValue(ref name_value) = meta {
            if let Lit::Str(ref tag) = name_value.lit {
                return Some(vec![RustlerAttr::Tag(tag.value())]);
            }
        }
        panic!("Cannot parse module")
    }

    fn try_parse_module(meta: &Meta) -> Option<Vec<RustlerAttr>> {
        if let Meta::NameValue(name_value) = meta {
            if let Lit::Str(ref module) = name_value.lit {
                let ident = format!("Elixir.{}", module.value());
                return Some(vec![RustlerAttr::Module(ident)]);
            }
        }
        panic!("Cannot parse tag")
    }
}
