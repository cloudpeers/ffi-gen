use crate::import::Instr;
use crate::{Abi, AbiFunction, AbiObject, AbiType, FunctionType, Interface, NumType, Var};
use genco::prelude::*;

pub struct JsGenerator {
    abi: Abi,
}

impl Default for JsGenerator {
    fn default() -> Self {
        Self { abi: Abi::Wasm32 }
    }
}

#[derive(Default)]
pub struct TsGenerator;

impl TsGenerator {
    pub fn generate(&self, iface: Interface) -> js::Tokens {
        quote! {
            /* tslint:disable */
            /* eslint:disable */
            export class Api {
              constructor();

              fetch(url, imports): Promise<void>;

              #(for func in iface.functions() join (#<line>#<line>) => #(self.generate_function(func)))

              drop(): void;
            }

            #(for obj in iface.objects() => #(self.generate_object(obj)))
        }
    }

    fn generate_function(&self, func: AbiFunction) -> js::Tokens {
        let args = quote!(#(for (name, ty) in &func.args join (, ) => #name: #(self.generate_return_type(Some(ty)))));
        let ret = self.generate_return_type(func.ret.as_ref());
        match &func.ty {
            FunctionType::Constructor(_) => {
                quote!(static #(&func.name)(api: Api, #args): #ret;)
            }
            _ => {
                quote!(#(&func.name)#args: #ret;)
            }
        }
    }

    fn generate_return_type(&self, ret: Option<&AbiType>) -> js::Tokens {
        if let Some(ret) = ret {
            match ret {
                AbiType::Num(prim) => match prim {
                    NumType::U8
                    | NumType::U16
                    | NumType::U32
                    | NumType::I8
                    | NumType::I16
                    | NumType::I32
                    | NumType::F32
                    | NumType::F64 => quote!(number),
                    NumType::U64 | NumType::I64 => quote!(BigInt),
                },
                AbiType::Isize | AbiType::Usize => quote!(number),
                AbiType::Bool => quote!(boolean),
                AbiType::RefStr | AbiType::String => quote!(string),
                AbiType::RefSlice(prim) | AbiType::Vec(prim) => {
                    // TODO String etcs
                    quote!(Array<#(&self.generate_return_type(Some(&AbiType::Num(*prim))))>)
                }
                AbiType::RefObject(i) | AbiType::Object(i) => quote!(#(i)),
                AbiType::Option(_) => todo!(),
                AbiType::Result(_) => todo!(),
                AbiType::Future(_) => todo!(),
                AbiType::Stream(_) => todo!(),
            }
        } else {
            quote!(void)
        }
    }
    fn generate_object(&self, obj: AbiObject) -> js::Tokens {
        quote! {
            export class #(&obj.name) {
                #(for method in obj.methods join (#<line>#<line>) => #(self.generate_function(method)))

                drop(): void;
            }
        }
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

                #(for func in iface.functions() => #(self.generate_function(&func)))
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

                #(for method in obj.methods => #(self.generate_function(&method)))

                drop() {
                    this.box.drop();
                }
            }
        }
    }

    fn generate_function(&self, func: &AbiFunction) -> js::Tokens {
        let ffi = self.abi.import(func);
        let api = match &func.ty {
            FunctionType::Constructor(_) => quote!(api),
            FunctionType::Method(_) => quote!(this.api),
            FunctionType::Function => quote!(this),
        };
        let args = quote!(#(for (name, _) in &func.args => #name,));
        let body = quote!(#(for instr in &ffi.instr => #(self.generate_instr(&api, instr))));
        match &func.ty {
            FunctionType::Constructor(_) => quote! {
                static #(&func.name)(api, #args) {
                    #body
                }
            },
            _ => quote! {
                #(&func.name)(#args) {
                    #body
                }
            },
        }
    }

    fn generate_instr(&self, api: &js::Tokens, instr: &Instr) -> js::Tokens {
        match instr {
            Instr::BorrowSelf(out) => quote!(const #(self.ident(out)) = this.box.borrow();),
            Instr::BorrowObject(in_, out) => {
                quote!(const #(self.ident(out)) = #(self.ident(in_)).box.borrow();)
            }
            Instr::MoveObject(in_, out)
            | Instr::MoveFuture(in_, out)
            | Instr::MoveStream(in_, out) => {
                quote!(const #(self.ident(out)) = #(self.ident(in_)).box.move();)
            }
            Instr::MakeObject(obj, box_, drop, out) => quote! {
                const #(self.ident(box_))_0 = () => { #api.drop(#_(#drop), #(self.ident(box_))); };
                const #(self.ident(box_))_1 = new Box(#(self.ident(box_)), #(self.ident(box_))_0);
                const #(self.ident(out)) = new #obj(#api, #(self.ident(box_))_1);
            },
            Instr::BindArg(arg, out) => quote!(const #(self.ident(out)) = #arg;),
            Instr::BindRet(ret, idx, out) => {
                quote!(const #(self.ident(out)) = #(self.ident(ret))[#(*idx)];)
            }
            // Casts below i32 are no ops as wasm only has i32 and i64
            // TODO
            Instr::LowerNum(in_, out, _num) | Instr::LiftNum(in_, out, _num) => {
                quote!(const #(self.ident(out)) = #(self.ident(in_));)
            }
            Instr::LowerBool(in_, out) => {
                quote!(const #(self.ident(out)) = #(self.ident(in_)) ? 1 : 0;)
            }
            Instr::LiftBool(in_, out) => quote!(const #(self.ident(out)) = #(self.ident(in_)) > 0;),
            Instr::StrLen(in_, out) | Instr::VecLen(in_, out) => {
                quote!(const #(self.ident(out)) = #(self.ident(in_)).length;)
            }
            Instr::Allocate(ptr, len, size, align) => {
                quote!(const #(self.ident(ptr)) = #api.allocate(#(self.ident(len)) * #(*size), #(*align));)
            }
            Instr::Deallocate(ptr, len, size, align) => quote! {
                if (#(self.ident(len)) > 0) {
                    #api.deallocate(#(self.ident(ptr)), #(self.ident(len)) * #(*size), #(*align));
                }
            },
            Instr::LowerString(in_, ptr, len) => quote! {
                const #(self.ident(in_))_0 =
                    new Uint8Array(#api.instance.exports.memory.buffer, #(self.ident(ptr)), #(self.ident(len)));
                const #(self.ident(in_))_1 = new TextEncoder();
                #(self.ident(in_))_1.encodeInto(#(self.ident(in_)), #(self.ident(in_))_0);
            },
            Instr::LiftString(ptr, len, out) => quote! {
                const #(self.ident(out))_0 =
                    new Uint8Array(#api.instance.exports.memory.buffer, #(self.ident(ptr)), #(self.ident(len)));
                const #(self.ident(out))_1 = new TextDecoder();
                const #(self.ident(out)) = #(self.ident(out))_1.decode(#(self.ident(out))_0);
            },

            Instr::LowerVec(in_, ptr, len, ty) => quote! {
                const #(self.ident(in_))_0 =
                    new #(self.generate_array(*ty))(
                        #api.instance.exports.memory.buffer, #(self.ident(ptr)), #(self.ident(len)));
                #(self.ident(in_))_0.set(#(self.ident(in_)), 0);
            },
            Instr::LiftVec(ptr, len, out, ty) => quote! {
                const #(self.ident(out))_0 =
                    new #(self.generate_array(*ty))(
                        #api.instance.exports.memory.buffer, #(self.ident(ptr)), #(self.ident(len)));
                const #(self.ident(out)) = Array.from(#(self.ident(out))_0);
            },
            Instr::Call(symbol, ret, args) => {
                let invoke = quote!(#api.instance.exports.#symbol(#(for arg in args => #(self.ident(arg)),)););
                if let Some(ret) = ret {
                    quote!(const #(self.ident(ret)) = #invoke)
                } else {
                    invoke
                }
            }
            Instr::ReturnValue(ret) => quote!(return #(self.ident(ret));),
            Instr::ReturnVoid => quote!(return;),
        }
    }

    fn ident(&self, var: &Var) -> js::Tokens {
        quote!(#(format!("tmp{}", var.binding)))
    }

    fn generate_array(&self, ty: NumType) -> js::Tokens {
        match ty {
            NumType::U8 => quote!(Uint8Array),
            NumType::U16 => quote!(Uint16Array),
            NumType::U32 => quote!(Uint32Array),
            NumType::U64 => quote!(BigUint64Array),
            NumType::I8 => quote!(Int8Array),
            NumType::I16 => quote!(Int16Array),
            NumType::I32 => quote!(Int32Array),
            NumType::I64 => quote!(BigInt64Array),
            NumType::F32 => quote!(Float32Array),
            NumType::F64 => quote!(Float64Array),
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
        for obj in iface.objects() {
            for func in &obj.methods {
                if let Some(ret) = Self::generate_return(func.ret.as_ref()) {
                    funcs.push(format!("\"__{}_{} {}\"", &obj.name, &func.name, ret))
                }
            }
        }
        for func in iface.functions() {
            if let Some(ret) = Self::generate_return(func.ret.as_ref()) {
                funcs.push(format!("\"__{} {}\"", &func.name, ret))
            }
        }
        funcs
    }

    fn generate_return(ret: Option<&AbiType>) -> Option<&'static str> {
        if let Some(ret) = ret {
            match ret {
                AbiType::Num(_)
                | AbiType::Isize
                | AbiType::Usize
                | AbiType::Bool
                | AbiType::RefObject(_)
                | AbiType::Object(_) => None,
                AbiType::RefStr | AbiType::RefSlice(_) => Some("i32 i32"),
                AbiType::String | AbiType::Vec(_) => Some("i32 i32 i32"),
                AbiType::Option(_) => todo!(),
                AbiType::Result(_) => todo!(),
                AbiType::Future(_) => todo!(),
                AbiType::Stream(_) => todo!(),
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
        let rust_gen = RustGenerator::new(Abi::Wasm32);
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
                    .expect("Compiling lib")
                    .success();
                assert!(ret);
                //println!("{}", #_(#bin));
                #wasm_multi_value
                let ret = Command::new("node")
                    .arg("--expose-gc")
                    .arg("--unhandled-rejections=strict")
                    .arg(#(quoted(js_file.as_ref().to_str().unwrap())))
                    .status()
                    .expect("Running node")
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

    pub fn compile_pass_ts(iface: &str, ts_tokens: js::Tokens) -> Result<()> {
        let iface = Interface::parse(iface)?;
        let ts_gen = TsGenerator::default();
        let js_tokens = ts_gen.generate(iface);

        assert_eq!(
            js_tokens.to_file_string().unwrap(),
            ts_tokens.to_file_string().unwrap()
        );
        Ok(())
    }
}
