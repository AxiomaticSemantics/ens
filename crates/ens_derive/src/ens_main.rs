use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

pub fn ens_main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    assert_eq!(
        input.sig.ident, "main",
        "`ens_main` can only be used on a function called 'main'."
    );

    TokenStream::from(quote! {
        #[allow(unused)]
        #input
    })
}
