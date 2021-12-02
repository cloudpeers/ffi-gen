use ffi_gen::{Abi, FfiGen};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

#[proc_macro]
pub fn ffi_gen(input: TokenStream) -> TokenStream {
    let input: TokenStream2 = input.into();
    (quote! {
        #[cfg_attr(all(target_family = "wasm", target_pointer_width = "32"), ffi_gen_macro::ffi_gen_wasm_32(#input))]
        #[cfg_attr(all(target_family = "wasm", target_pointer_width = "64"), ffi_gen_macro::ffi_gen_wasm_64(#input))]
        #[cfg_attr(all(not(target_family = "wasm"), target_pointer_width = "32"), ffi_gen_macro::ffi_gen_native_32(#input))]
        #[cfg_attr(all(not(target_family = "wasm"), target_pointer_width = "64"), ffi_gen_macro::ffi_gen_native_64(#input))]
        struct Api;
    })
    .into()
}

#[proc_macro_attribute]
pub fn ffi_gen_wasm_32(input: TokenStream, _: TokenStream) -> TokenStream {
    inner_ffi_gen(input, Abi::Wasm32)
}

#[proc_macro_attribute]
pub fn ffi_gen_wasm_64(input: TokenStream, _: TokenStream) -> TokenStream {
    inner_ffi_gen(input, Abi::Wasm64)
}

#[proc_macro_attribute]
pub fn ffi_gen_native_32(input: TokenStream, _: TokenStream) -> TokenStream {
    inner_ffi_gen(input, Abi::Native32)
}

#[proc_macro_attribute]
pub fn ffi_gen_native_64(input: TokenStream, _: TokenStream) -> TokenStream {
    inner_ffi_gen(input, Abi::Native64)
}

fn inner_ffi_gen(input: TokenStream, abi: Abi) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::LitStr);
    let ffigen = FfiGen::new(input.value()).unwrap();
    let rust = ffigen.generate_rust(abi).unwrap();
    rust.parse().unwrap()
}
