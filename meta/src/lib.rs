use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn capi_safe_unwind(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let fail_return = syn::parse_macro_input!(attr as syn::Expr);

    let signature = &func.sig;
    let body = &func.block;
    let attributes = &func.attrs;
    let vis = &func.vis;

    quote! {
        #(#attributes)*
        #vis #signature {
            let result = std::panic::catch_unwind(|| #body);
            match result {
                Ok(result) => result,
                Err(_) => #fail_return,
            }
        }
    }.into()
}