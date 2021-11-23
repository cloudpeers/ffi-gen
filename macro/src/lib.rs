use ffi_gen::{Abi, Interface, RustGenerator};
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
    inner_ffi_gen(input, Abi::Wasm(32))
}

#[proc_macro_attribute]
pub fn ffi_gen_wasm_64(input: TokenStream, _: TokenStream) -> TokenStream {
    inner_ffi_gen(input, Abi::Wasm(64))
}

#[proc_macro_attribute]
pub fn ffi_gen_native_32(input: TokenStream, _: TokenStream) -> TokenStream {
    inner_ffi_gen(input, Abi::Native(32))
}

#[proc_macro_attribute]
pub fn ffi_gen_native_64(input: TokenStream, _: TokenStream) -> TokenStream {
    inner_ffi_gen(input, Abi::Native(64))
}

fn inner_ffi_gen(input: TokenStream, abi: Abi) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::LitStr);
    let s = std::fs::read_to_string(input.value()).unwrap();
    let iface = Interface::parse(&s).unwrap();
    let mut gen = RustGenerator::new(abi);
    let tokens = gen.generate(iface);
    let s = tokens.to_file_string().unwrap();
    s.parse().unwrap()
}
