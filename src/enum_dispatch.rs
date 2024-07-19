use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, Data, DeriveInput, Result};

use crate::attrs::{FnArgModifier, FunctionAttribute, VariantAttribute};
#[derive(Debug)]
pub struct ContainerAttribute {
    pub trait_type: syn::Type,
}
impl Parse for ContainerAttribute {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let trait_type: syn::Type = input.parse()?;
        Ok(Self { trait_type })
    }
}

#[derive(Debug)]
pub struct ParsedVariants {
    pub name: syn::Ident,
    pub variant_type: syn::Type,
    pub attributes: VariantAttribute,
}
impl ParsedVariants {
    pub fn match_arm(&self, function: &FunctionAttribute, trait_name: &syn::Type) -> TokenStream {
        let FunctionAttribute { name, params, .. } = function;
        let Self {
            name: variant_name,
            attributes,
            ..
        } = self;
        let params: Vec<syn::Ident> = params
            .iter()
            .filter_map(|param| param.ident.clone())
            .collect();
        let self_param = match &attributes.modifier {
            Some(FnArgModifier::AsRef) => quote! {inner.as_ref()},
            Some(FnArgModifier::Deref) => quote! {*inner},
            None => quote! {inner},
        };
        quote! {
            Self::#variant_name(inner) => {
                #trait_name::#name(#self_param, #(#params),*)
            },
        }
    }
}
impl TryFrom<syn::Variant> for ParsedVariants {
    type Error = syn::Error;
    fn try_from(value: syn::Variant) -> std::result::Result<Self, Self::Error> {
        let syn::Variant {
            ident,
            fields,
            attrs,
            ..
        } = value;
        let variant_type = match &fields {
            syn::Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "EnumDispatch only works with unnamed fields",
                    ));
                }
                fields.unnamed.first().unwrap().ty.clone()
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    ident,
                    "EnumDispatch only works with unnamed fields",
                ))
            }
        };
        let attributes = attrs
            .iter()
            .find(|attr| attr.path().is_ident("enum_dispatch"))
            .map(|attr| attr.parse_args::<VariantAttribute>())
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            name: ident,
            variant_type,
            attributes,
        })
    }
}
pub fn expand(expand: DeriveInput) -> Result<TokenStream> {
    let DeriveInput {
        ident, data, attrs, ..
    } = expand;
    let Data::Enum(data) = data else {
        return Err(syn::Error::new_spanned(
            ident,
            "EnumDispatch only works with enums",
        ));
    };
    let Some(ContainerAttribute { trait_type }) = attrs
        .iter()
        .find(|attr| attr.path().is_ident("enum_dispatch"))
        .map(|attr| attr.parse_args::<ContainerAttribute>())
        .transpose()?
    else {
        return Err(syn::Error::new_spanned(
            ident,
            "EnumDispatch requires a container attribute",
        ));
    };
    let functions = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("function"))
        .map(|attr| attr.parse_args::<FunctionAttribute>())
        .collect::<Result<Vec<_>>>()?;
    let variants = data
        .variants
        .into_iter()
        .map(|variant| ParsedVariants::try_from(variant))
        .collect::<Result<Vec<_>>>()?;
    let mut function_declaractions = Vec::new();
    for function in functions {
        let match_arms = variants
            .iter()
            .map(|variant| variant.match_arm(&function, &trait_type));
        let declaration = quote! {
            #function {
                match self {
                    #(#match_arms)*
                }
            }
        };
        function_declaractions.push(declaration);
    }
    let from = variants
        .iter()
        .filter(|v| v.attributes.from)
        .map(|variant| {
            let ParsedVariants {
                name, variant_type, ..
            } = variant;
            quote! {
                impl From<#variant_type> for #ident {
                    fn from(inner: #variant_type) -> Self {
                        Self::#name(inner)
                    }
                }
            }
        });
    let result = quote! {
        impl #trait_type for #ident {
            #(#function_declaractions)*
        }
        #(#from)*
    };

    Ok(result)
}
