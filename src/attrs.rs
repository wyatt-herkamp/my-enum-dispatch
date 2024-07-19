use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::parse::{Lookahead1, Parse};
use syn::{FnArg, Result};
mod keywords {
    syn::custom_keyword!(from);
    syn::custom_keyword!(deref);
    syn::custom_keyword!(as_ref);
    syn::custom_keyword!(modifier);
}
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

#[derive(Debug, Default)]
pub struct VariantAttribute {
    pub from: bool,
    pub modifier: Option<FnArgModifier>,
}
impl Parse for VariantAttribute {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut from = false;
        let mut modifier = None;
        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(keywords::from) {
                input.parse::<keywords::from>()?;
                from = true;
            } else if lookahead.peek(keywords::modifier) {
                input.parse::<keywords::modifier>()?;
                input.parse::<syn::Token![=]>()?;

                modifier = Some(input.parse()?);
            } else {
                break;
            }
            if input.is_empty() {
                break;
            }
            input.parse::<syn::Token![,]>()?;
        }
        Ok(Self { from, modifier })
    }
}
#[derive(Debug)]
pub enum FnArgModifier {
    AsRef,
    Deref,
}
impl Parse for FnArgModifier {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let look = input.lookahead1();
        if look.peek(keywords::as_ref) {
            input.parse::<keywords::as_ref>()?;
            Ok(Self::AsRef)
        } else if look.peek(keywords::deref) {
            input.parse::<keywords::deref>()?;
            Ok(Self::Deref)
        } else {
            Err(look.error())
        }
    }
}
#[derive(Debug)]
pub struct FunctionParam {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,
}
impl Parse for FunctionParam {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let fn_arg = FnArg::parse(input)?;
        match fn_arg {
            FnArg::Typed(pat) => {
                let ident = match &*pat.pat {
                    syn::Pat::Ident(pat) => Some(pat.ident.clone()),
                    _ => None,
                };
                Ok(Self { ident, ty: *pat.ty })
            }
            FnArg::Receiver(receiver) => {
                if receiver.reference.is_none() {
                    return Ok(Self {
                        ident: None,
                        ty: syn::parse_quote! {self},
                    });
                }
                if receiver.mutability.is_some() && receiver.reference.is_some() {
                    return Ok(Self {
                        ident: None,
                        ty: syn::parse_quote! {&mut self},
                    });
                }
                Ok(Self {
                    ident: None,
                    ty: syn::parse_quote! {&self},
                })
            }
        }
    }
}

#[derive(Debug)]
pub struct FunctionAttribute {
    pub fn_keyword: syn::Token![fn],
    pub name: syn::Ident,
    pub self_param: syn::Type,
    pub params: Vec<FunctionParam>,
    pub return_type: Option<syn::Type>,
}
impl ToTokens for FunctionAttribute {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let FunctionAttribute {
            fn_keyword,
            name,
            self_param,
            params,
            return_type,
        } = self;
        let params: Vec<TokenStream> = params
            .iter()
            .filter(|FunctionParam { ident, .. }| ident.is_some())
            .map(|FunctionParam { ident, ty }| {
                let ident = ident.as_ref().unwrap();
                quote_spanned! {ident.span()=> #ident: #ty}
            })
            .collect();
        let return_type = return_type
            .as_ref()
            .map(|ty| {
                quote! {
                    -> #ty
                }
            })
            .unwrap_or_else(|| quote! {});
        tokens.extend(quote! {
            #fn_keyword #name(#self_param, #(#params),*) #return_type
        });
    }
}
impl Parse for FunctionAttribute {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let fn_keyword: syn::Token![fn] = input.parse()?;
        let name: syn::Ident = input.parse()?;
        let mut params = if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            syn::punctuated::Punctuated::<FunctionParam, syn::token::Comma>::parse_terminated(
                &content,
            )?
            .into_iter()
            .collect()
        } else {
            Vec::new()
        };
        // Must have a self parameter and remove it
        let self_param = params.remove(0);
        if !is_self_param(&self_param.ty) {
            return Err(input.error("expected `self` parameter"));
        }
        let return_type = if input.peek(syn::token::RArrow) {
            input.parse::<syn::token::RArrow>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self {
            fn_keyword,
            name,
            params,
            return_type,
            self_param: self_param.ty,
        })
    }
}
fn is_self_param(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Reference(ty) => {
            if let syn::Type::Path(path) = &*ty.elem {
                if path.path.is_ident("self") || path.path.is_ident("Self") {
                    return true;
                }
            }
        }
        syn::Type::Path(path) => {
            if path.path.is_ident("self") || path.path.is_ident("Self") {
                return true;
            }
        }
        _ => {}
    }
    false
}

#[cfg(test)]
mod tests {
    use quote::quote_spanned;
    use syn::Attribute;

    use super::FunctionAttribute;

    #[test]
    fn test_function_attribute() {
        let input: Attribute = syn::parse_quote! {
            #[function(fn test(self, param1: u8, param2: u8) -> u8)]
        };
        println!("{:#?}", input);
        let attribute = input
            .parse_args::<FunctionAttribute>()
            .expect("Failed to parse attribute");
        assert_eq!(attribute.name.to_string(), "test");
        assert_eq!(attribute.params.len(), 2);
        assert!(attribute.return_type.is_some());
        println!("{:#?}", attribute);
    }
    #[test]
    fn test_function_attribute_no_return() {
        let input: Attribute = syn::parse_quote! {
            #[function(fn test(&self, param: u8,param2: u8))]
        };
        let attribute = input
            .parse_args::<FunctionAttribute>()
            .expect("Failed to parse attribute");
        assert_eq!(attribute.name.to_string(), "test");
        assert_eq!(attribute.params.len(), 2);
        assert!(attribute.return_type.is_none());
        println!("{:#?}", attribute);
    }
}
