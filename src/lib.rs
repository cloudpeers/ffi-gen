mod abi;
mod dart;
mod js;
mod parser;
mod rust;

use crate::abi::{
    export, import, AbiFunction, AbiFuture, AbiObject, AbiStream, AbiType, FunctionType, NumType,
    Return, Var,
};
use crate::dart::DartGenerator;
use crate::js::{JsGenerator, TsGenerator, WasmMultiValueShim};
use crate::parser::Interface;
use crate::rust::RustGenerator;
use anyhow::Result;
use std::path::Path;
use std::process::Command;

pub use crate::abi::Abi;

pub struct FfiGen {
    iface: Interface,
}

impl FfiGen {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let s = std::fs::read_to_string(path)?;
        let iface = Interface::parse(&s)?;
        Ok(Self { iface })
    }

    pub fn generate_rust(&self, abi: Abi) -> Result<String> {
        let rust = RustGenerator::new(abi);
        let rust = rust.generate(self.iface.clone()).to_file_string()?;
        Ok(rust)
    }

    pub fn wasm_multi_value_shim<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        WasmMultiValueShim::new().run(path, self.iface.clone())
    }

    pub fn generate_dart<P: AsRef<Path>>(&self, path: P, cdylib: &str) -> Result<()> {
        let dart = DartGenerator::new(cdylib.to_string());
        let dart = dart.generate(self.iface.clone()).to_file_string()?;
        std::fs::write(path.as_ref(), &dart)?;
        let status = Command::new("dart")
            .arg("format")
            .arg(path.as_ref())
            .status()?;
        if !status.success() {
            anyhow::bail!("dart format failed");
        }
        Ok(())
    }

    pub fn generate_js<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let js = JsGenerator::default();
        let js = js.generate(self.iface.clone()).to_file_string()?;
        std::fs::write(path.as_ref(), &js)?;
        let status = Command::new("prettier")
            .arg("--write")
            .arg(path.as_ref())
            .status()?;
        if !status.success() {
            anyhow::bail!("prettier failed");
        }
        Ok(())
    }

    pub fn generate_ts<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let ts = TsGenerator::default();
        let ts = ts.generate(self.iface.clone()).to_file_string()?;
        std::fs::write(path.as_ref(), &ts)?;
        let status = Command::new("prettier")
            .arg("--write")
            .arg(path.as_ref())
            .status()?;
        if !status.success() {
            anyhow::bail!("prettier failed");
        }
        Ok(())
    }
}

#[cfg(feature = "test_runner")]
pub mod test_runner {
    pub use crate::dart::test_runner::compile_pass as compile_pass_dart;
    pub use crate::js::test_runner::compile_pass as compile_pass_js;
    pub use crate::js::test_runner::compile_pass_ts;
    pub use crate::rust::test_runner::compile_pass as compile_pass_rust;

    #[macro_export]
    macro_rules! compile_pass {
        ($ident:ident, $iface:expr, ($($api:tt)*), ($($rust:tt)*), ($($dart:tt)*), ($($js:tt)*), ($($ts:tt)*)) => {
            mod $ident {
                #[test]
                fn rust() {
                    $crate::test_runner::compile_pass_rust($iface, genco::quote!($($api)*), genco::quote!($($rust)*)).unwrap();
                }

                #[test]
                fn dart() {
                    $crate::test_runner::compile_pass_dart($iface, genco::quote!($($api)*), genco::quote!($($dart)*)).unwrap();
                }

                #[test]
                fn js() {
                    $crate::test_runner::compile_pass_js($iface, genco::quote!($($api)*), genco::quote!($($js)*)).unwrap();
                }

                /*#[test]
                fn ts() {
                    $crate::test_runner::compile_pass_ts($iface, genco::quote!($($ts)*)).unwrap();
                }*/
            }
        }
    }
}
