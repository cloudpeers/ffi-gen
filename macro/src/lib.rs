use ffi_gen::{Abi, Interface, RustGenerator};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

#[proc_macro]
pub fn ffi_gen(input: TokenStream) -> TokenStream {
    let input: TokenStream2 = input.into();
    quote! {
        #[cfg_attr(target_family = "wasm", ffi_gen_macro::ffi_gen_wasm(#input))]
        #[cfg_attr(not(target_family = "wasm"), ffi_gen_macro::ffi_gen_native(#input))]
        struct Api;
    }.into()
}

#[proc_macro_attribute]
pub fn ffi_gen_wasm(input: TokenStream, _: TokenStream) -> TokenStream {
    inner_ffi_gen(input, Abi::Wasm32)
}

#[proc_macro_attribute]
pub fn ffi_gen_native(input: TokenStream, _: TokenStream) -> TokenStream {
    inner_ffi_gen(input, Abi::Native)
}

fn inner_ffi_gen(input: TokenStream, abi: Abi) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::LitStr);
    let s = std::fs::read_to_string(input.value()).unwrap();
    let iface = Interface::parse(&s).unwrap();
    let gen = RustGenerator::new(abi);
    let tokens = gen.generate(iface);
    let s = tokens.to_file_string().unwrap();
    s.parse().unwrap()
}
