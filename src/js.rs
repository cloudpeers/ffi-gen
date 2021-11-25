use crate::{Abi, AbiFunction, AbiObject, AbiType, Interface, PrimType};
use genco::prelude::*;

pub struct JsGenerator {
    abi: Abi,
}

impl Default for JsGenerator {
    fn default() -> Self {
        Self { abi: Abi::Wasm(32) }
    }
}

impl JsGenerator {
    pub fn generate(&self, iface: Interface) -> js::Tokens {
        quote! {
            // a node fetch polyfill that won't trigger webpack
            // idea borrowed from:
            // https://github.com/dcodeIO/webassembly/blob/master/src/index.js#L223
            let fs;
            function fetch_polyfill(file) {
                return new Promise((resolve, reject) => {
                    (fs || (fs = eval("equire".replace(/^/, 'r'))("fs"))).readFile(
                        file,
                        function(err, data) {
                            return (err)
                                ? reject(err)
                                : resolve({
                                    arrayBuffer: () => Promise.resolve(data),
                                    ok: true,
                                });
                        }
                    );
                });
            }

            const fetchFn = (typeof fetch === "function" && fetch) || fetch_polyfill;

            // gets the wasm at a url and instantiates it.
            // checks if streaming instantiation is available and uses that
            function fetchAndInstantiate(url, imports) {
                return fetchFn(url)
                    .then((resp) => {
                        if (!resp.ok) {
                            throw new Error("Got a ${resp.status} fetching wasm @ ${url}");
                        }

                        const wasm = "application/wasm";
                        const type = resp.headers && resp.headers.get("content-type");

                        return (WebAssembly.instantiateStreaming && type === wasm)
                            ? WebAssembly.instantiateStreaming(resp, imports)
                            : resp.arrayBuffer().then(buf => WebAssembly.instantiate(buf, imports));
                        })
                        .then(result => result.instance);
            }

            const dropRegistry = new FinalizationRegistry(drop => drop());

            class Box {
                constructor(ptr, destructor) {
                    this.ptr = ptr;
                    this.dropped = false;
                    this.moved = false;
                    dropRegistry.register(this, destructor);
                    this.destructor = destructor;
                }

                borrow() {
                    if (this.dropped) {
                        throw new Error("use after free");
                    }
                    if (this.moved) {
                        throw new Error("use after move");
                    }
                    return this.ptr;
                }

                move() {
                    if (this.dropped) {
                        throw new Error("use after free");
                    }
                    if (this.moved) {
                        throw new Error("can't move value twice");
                    }
                    this.moved = true;
                    dropRegistry.unregister(this);
                    return this.ptr;
                }

                drop() {
                    if (this.dropped) {
                        throw new Error("double free");
                    }
                    if (this.moved) {
                        throw new Error("can't drop moved value");
                    }
                    this.dropped = true;
                    dropRegistry.unregister(this);
                    this.destructor();
                }
            }

            class Api {
                async fetch(url, imports) {
                    this.instance = await fetchAndInstantiate(url, imports);
                }

                allocate(size, align) {
                    return this.instance.exports.allocate(size, align);
                }

                deallocate(ptr, size, align) {
                    this.instance.exports.deallocate(ptr, size, align);
                }

                drop(symbol, ptr) {
                    this.instance.exports[symbol](0, ptr);
                }

                #(for func in iface.functions() => #(self.generate_function(func)))
            }

            #(for obj in iface.objects() => #(self.generate_object(obj)))

            module.exports = {
                Api: Api,
                #(for obj in iface.objects() => #(&obj.name): #(&obj.name),)
            }
        }
    }

    fn generate_object(&self, obj: AbiObject) -> js::Tokens {
        quote! {
            class #(&obj.name) {
                constructor(api, box) {
                    this.api = api;
                    this.box = box;
                }

                #(for method in obj.methods => #(self.generate_function(method)))

                drop() {
                    this.box.drop();
                }
            }
        }
    }

    fn generate_function(&self, func: AbiFunction) -> js::Tokens {
        let api_arg = if func.is_static {
            quote!(api,)
        } else {
            quote!()
        };
        let api = if func.object.is_some() && func.is_static {
            quote!(api)
        } else if func.object.is_some() {
            quote!(this.api)
        } else {
            quote!(this)
        };
        let self_arg = if func.needs_self() {
            quote!(this.box.borrow(),)
        } else {
            quote!()
        };
        let static_ = if func.is_static {
            quote!(static)
        } else {
            quote!()
        };
        quote! {
            #static_ #(&func.name)(#api_arg #(for (name, _) in &func.args => #name,)) {
                #(for (name, ty) in &func.args => #(self.generate_lower(&api, name, ty)))
                const ret = #(&api).instance.exports.#(format!("__{}", func.fqn()))(
                    #self_arg
                    #(for (name, ty) in &func.args => #(self.generate_arg(name, ty))));
                #(self.generate_lift(&api, func.ret.as_ref()))
                #(for (name, ty) in &func.args => #(self.generate_lower_cleanup(name, ty)))
                #(self.generate_return_stmt(func.ret.as_ref()))
            }
        }
    }

    fn generate_lower(&self, api: &js::Tokens, name: &str, ty: &AbiType) -> js::Tokens {
        match ty {
            AbiType::Prim(_) => quote!(),
            AbiType::RefStr | AbiType::String => quote! {
                const #(name)_ptr = #api.allocate(#name.length, 1);
                const #(name)_buf = new Uint8Array(#api.instance.exports.memory.buffer, #(name)_ptr, #name.length);
                const #(name)_encoder = new TextEncoder();
                #(name)_encoder.encodeInto(#name, #(name)_buf);
            },
            AbiType::RefSlice(ty) | AbiType::Vec(ty) => {
                let (size, align) = self.abi.layout(*ty);
                quote! {
                    const #(name)_ptr = this.allocate(#name.length * #size, #align);
                    const #(name)_buf = new #(self.generate_array(*ty))(
                        #api.instance.exports.memory.buffer, #(name)_ptr, #name.length);
                    #(name)_buf.set(#name, 0);
                }
            }
            AbiType::Box(_) => quote!(const #(name)_ptr = #(name).move();),
            AbiType::Ref(_) => quote!(const #(name)_ptr = #(name).borrow();),
            AbiType::Object(_) => todo!(),
        }
    }

    fn generate_arg(&self, name: &str, ty: &AbiType) -> js::Tokens {
        match ty {
            AbiType::Prim(_) => quote!(#name,),
            AbiType::RefStr | AbiType::RefSlice(_) => quote!(#(name)_ptr, #name.length,),
            AbiType::String | AbiType::Vec(_) => quote!(#(name)_ptr, #name.length, #name.length,),
            AbiType::Box(_) | AbiType::Ref(_) => quote!(#(name)_ptr,),
            AbiType::Object(_) => todo!(),
        }
    }

    fn generate_lower_cleanup(&self, name: &str, ty: &AbiType) -> js::Tokens {
        match ty {
            AbiType::Prim(_) | AbiType::String | AbiType::Vec(_) => quote!(),
            AbiType::RefStr => quote! {
                if (#name.length > 0) {
                    this.deallocate(#(name)_ptr, #name.length, 1);
                }
            },
            AbiType::RefSlice(ty) => {
                let (size, align) = self.abi.layout(*ty);
                quote! {
                    if (#name.length > 0) {
                        this.deallocate(#(name)_ptr, #name.length * #size, #align);
                    }
                }
            }
            AbiType::Box(_) | AbiType::Ref(_) => quote!(),
            AbiType::Object(_) => todo!(),
        }
    }

    fn generate_lift(&self, api: &js::Tokens, ret: Option<&AbiType>) -> js::Tokens {
        if let Some(ret) = ret {
            match ret {
                AbiType::Prim(_) => quote!(),
                AbiType::RefStr => quote! {
                    const buf = new Uint8Array(#api.instance.exports.memory.buffer, ret[0], ret[1]);
                    const decoder = new TextDecoder();
                    const ret_str = decoder.decode(buf);
                },
                AbiType::String => quote! {
                    const buf = new Uint8Array(#api.instance.exports.memory.buffer, ret[0], ret[1]);
                    const decoder = new TextDecoder();
                    const ret_str = decoder.decode(buf);
                    if (ret[2] > 0) {
                        #api.deallocate(ret[0], ret[2], 1);
                    }
                },
                AbiType::RefSlice(ty) => {
                    let (size, _align) = self.abi.layout(*ty);
                    quote! {
                        const buf = new #(self.generate_array(*ty))(
                            #api.instance.exports.memory.buffer, ret[0], ret[1] * #size);
                        const ret_arr = Array.from(buf);
                    }
                }
                AbiType::Vec(ty) => {
                    let (size, align) = self.abi.layout(*ty);
                    quote! {
                        const buf = new #(self.generate_array(*ty))(
                            #api.instance.exports.memory.buffer, ret[0], ret[1]);
                        const ret_arr = Array.from(buf);
                        if (ret[2] > 0) {
                            #api.deallocate(ret[0], ret[2] * #size, #align);
                        }
                    }
                }
                AbiType::Box(ident) => {
                    let destructor = format!("drop_box_{}", ident);
                    quote! {
                        const destructor = () => { #api.drop(#_(#destructor), ret) };
                        const ret_box = new Box(ret, destructor);
                    }
                }
                AbiType::Ref(ident) => panic!("invalid return type `&{}`", ident),
                AbiType::Object(ident) => {
                    let destructor = format!("drop_box_{}", ident);
                    quote! {
                        const destructor = () => { #api.drop(#_(#destructor), ret) };
                        const ret_box = new Box(ret, destructor);
                        const ret_obj = new #ident(#api, ret_box);
                    }
                }
            }
        } else {
            quote!()
        }
    }

    fn generate_return_stmt(&self, ret: Option<&AbiType>) -> js::Tokens {
        if let Some(ret) = ret {
            match ret {
                AbiType::Prim(_) => quote!(return ret;),
                AbiType::RefStr | AbiType::String => quote!(return ret_str;),
                AbiType::RefSlice(_) | AbiType::Vec(_) => quote!(return ret_arr;),
                AbiType::Box(_) | AbiType::Ref(_) => quote!(return ret_box;),
                AbiType::Object(_) => quote!(return ret_obj;),
            }
        } else {
            quote!()
        }
    }

    fn generate_array(&self, ty: PrimType) -> js::Tokens {
        match ty {
            PrimType::U8 => quote!(Uint8Array),
            PrimType::U16 => quote!(Uint16Array),
            PrimType::U32 => quote!(Uint32Array),
            PrimType::U64 => quote!(BigUint64Array),
            PrimType::Usize => quote!(Uint32Array),
            PrimType::I8 => quote!(Int8Array),
            PrimType::I16 => quote!(Int16Array),
            PrimType::I32 => quote!(Int32Array),
            PrimType::I64 => quote!(BigInt64Array),
            PrimType::Isize => quote!(Int32Array),
            PrimType::F32 => quote!(Float32Array),
            PrimType::F64 => quote!(Float64Array),
            PrimType::Bool => quote!(Uint8Array),
        }
    }
}

pub struct WasmMultiValueShim;

impl WasmMultiValueShim {
    pub fn generate(path: &str, iface: Interface) -> rust::Tokens {
        let args = Self::generate_args(iface);
        if !args.is_empty() {
            quote! {
                let ret = Command::new("multi-value-reverse-polyfill")
                    .arg(#_(#path))
                    #(for arg in args => .arg(#arg))
                    .status()
                    .unwrap()
                    .success();
                assert!(ret);
            }
        } else {
            quote! {
                let ret = Command::new("cp")
                    .arg(#_(#path))
                    .arg(#_(#(path).multivalue.wasm))
                    .status()
                    .unwrap()
                    .success();
                assert!(ret);
            }
        }
    }

    fn generate_args(iface: Interface) -> Vec<String> {
        let mut funcs = vec![];
        for func in iface.into_functions() {
            if let Some(ret) = Self::generate_return(func.ret.as_ref()) {
                funcs.push(format!("\"__{} {}\"", &func.name, ret))
            }
        }
        funcs
    }

    fn generate_return(ret: Option<&AbiType>) -> Option<&'static str> {
        if let Some(ret) = ret {
            match ret {
                AbiType::Prim(_) | AbiType::Box(_) | AbiType::Ref(_) | AbiType::Object(_) => None,
                AbiType::RefStr | AbiType::RefSlice(_) => Some("i32 i32"),
                AbiType::String | AbiType::Vec(_) => Some("i32 i32 i32"),
            }
        } else {
            None
        }
    }
}

#[cfg(feature = "test_runner")]
pub mod test_runner {
    use super::*;
    use crate::{Abi, RustGenerator};
    use anyhow::Result;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use trybuild::TestCases;

    pub fn compile_pass(iface: &str, rust: rust::Tokens, js: js::Tokens) -> Result<()> {
        let iface = Interface::parse(iface)?;
        let mut rust_file = NamedTempFile::new()?;
        let mut rust_gen = RustGenerator::new(Abi::Wasm(32));
        let rust_tokens = rust_gen.generate(iface.clone());
        let mut js_file = NamedTempFile::new()?;
        let js_gen = JsGenerator::default();
        let js_tokens = js_gen.generate(iface.clone());

        let library_tokens = quote! {
            #rust_tokens
            #rust

            extern "C" {
                fn __panic(ptr: isize, len: usize);
            }

            pub fn panic(msg: &str) {
                unsafe { __panic(msg.as_ptr() as _, msg.len()) };
            }
        };

        let library_file = NamedTempFile::new()?;
        let bin_tokens = quote! {
            #js_tokens

            async function main() {
                const assert = require("assert");
                const api = new Api();
                await api.fetch(#_(#(library_file.as_ref().to_str().unwrap()).multivalue.wasm), {
                    env: {
                        __panic: (ptr, len) => {
                            const buf = new Uint8Array(api.instance.exports.memory.buffer, ptr, len);
                            const decoder = new TextDecoder();
                            throw decoder.decode(buf);
                        }
                    }
                });
                #js
            }
            main();
        };

        let library = library_tokens.to_file_string()?;
        rust_file.write_all(library.as_bytes())?;
        let bin = bin_tokens.to_file_string()?;
        js_file.write_all(bin.as_bytes())?;

        let wasm_multi_value =
            WasmMultiValueShim::generate(library_file.as_ref().to_str().unwrap(), iface);

        let runner_tokens: rust::Tokens = quote! {
            fn main() {
                use std::process::Command;
                let ret = Command::new("rustc")
                    .arg("--crate-name")
                    .arg("compile_pass")
                    .arg("--crate-type")
                    .arg("cdylib")
                    .arg("-o")
                    .arg(#(quoted(library_file.as_ref().to_str().unwrap())))
                    .arg("--target")
                    .arg("wasm32-unknown-unknown")
                    .arg(#(quoted(rust_file.as_ref().to_str().unwrap())))
                    .status()
                    .unwrap()
                    .success();
                assert!(ret);
                //println!("{}", #_(#bin));
                #wasm_multi_value
                let ret = Command::new("node")
                    .arg("--expose-gc")
                    .arg(#(quoted(js_file.as_ref().to_str().unwrap())))
                    .status()
                    .unwrap()
                    .success();
                assert!(ret);
            }
        };

        let mut runner_file = NamedTempFile::new()?;
        let runner = runner_tokens.to_file_string()?;
        runner_file.write_all(runner.as_bytes())?;

        let test = TestCases::new();
        test.pass(runner_file.as_ref());
        Ok(())
    }

    #[test]
    fn no_args_no_ret() {
        compile_pass(
            "hello_world fn();",
            quote!(
                pub fn hello_world() {}
            ),
            quote!(api.hello_world();),
        )
        .unwrap();
    }
}
