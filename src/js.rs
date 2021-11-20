use crate::{AbiFunction, Interface};
use genco::prelude::*;

pub struct JsGenerator {}

impl JsGenerator {
    pub fn new() -> Self {
        Self {}
    }

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

            class Api {
                async fetch(url, imports) {
                    this.instance = await fetchAndInstantiate(url, imports);
                }

                #(for func in iface.into_functions() => #(self.generate_function(func)))
            }

            module.exports = {
                Api: Api,
            }
        }
    }

    pub fn generate_function(&self, func: AbiFunction) -> js::Tokens {
        quote! {
            #(&func.name)() {
                this.instance.exports.#(format!("__{}", &func.name))();
            }
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
        let rust_gen = RustGenerator::new(Abi::Native);
        let rust_tokens = rust_gen.generate(iface.clone());
        let mut js_file = NamedTempFile::new()?;
        let js_gen = JsGenerator::new();
        let js_tokens = js_gen.generate(iface);

        let library_tokens = quote! {
            #rust_tokens
            #rust
        };

        let library_file = NamedTempFile::new()?;
        let bin_tokens = quote! {
            #js_tokens

            async function main() {
                const api = new Api();
                await api.fetch(#_(#(library_file.as_ref().to_str().unwrap())));
                #js
            }
            main();
        };

        let library = library_tokens.to_file_string()?;
        rust_file.write_all(library.as_bytes())?;
        let bin = bin_tokens.to_file_string()?;
        js_file.write_all(bin.as_bytes())?;

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
                let ret = Command::new("node")
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
