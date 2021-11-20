mod abi;
mod dart;
mod js;
mod parser;
mod rust;

pub use abi::{Abi, AbiFunction};
pub use dart::DartGenerator;
pub use js::JsGenerator;
pub use parser::{Function, FunctionType, Interface, Method, Object, Type};
pub use rust::RustGenerator;

#[cfg(feature = "test_runner")]
pub mod test_runner {
    pub use crate::dart::test_runner::compile_pass as compile_pass_dart;
    pub use crate::js::test_runner::compile_pass as compile_pass_js;
    pub use crate::rust::test_runner::compile_pass as compile_pass_rust;

    #[macro_export]
    macro_rules! compile_pass {
        ($ident:ident, $iface:expr, ($($api:tt)*), ($($rust:tt)*), ($($dart:tt)*), ($($js:tt)*),) => {
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
            }
        }
    }
}
