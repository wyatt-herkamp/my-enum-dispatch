use proc_macro::TokenStream;
use syn::DeriveInput;
pub(crate) mod attrs;
mod enum_dispatch;
#[proc_macro_derive(EnumDispatch, attributes(enum_dispatch, function))]
pub fn enum_dispatch(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    match enum_dispatch::expand(input) {
        Ok(ok) => ok.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
