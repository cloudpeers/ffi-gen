use crate::export::Instr;
use crate::{Abi, AbiFunction, AbiObject, AbiType, FunctionType, Interface, NumType, Return, Var};
use genco::prelude::*;

pub struct RustGenerator {
    abi: Abi,
}

impl RustGenerator {
    pub fn new(abi: Abi) -> Self {
        Self { abi }
    }

    pub fn generate(&self, iface: Interface) -> rust::Tokens {
        quote! {
            /// Try to execute some function, catching any panics and aborting to make sure Rust
            /// doesn't unwind across the FFI boundary.
            pub fn panic_abort<R>(func: impl FnOnce() -> R + std::panic::UnwindSafe) -> R {
                match std::panic::catch_unwind(func) {
                    Ok(res) => res,
                    Err(_) => {
                        std::process::abort();
                    }
                }
            }

            #[no_mangle]
            pub unsafe extern "C" fn allocate(size: usize, align: usize) -> *mut u8 {
                let layout = std::alloc::Layout::from_size_align_unchecked(size, align);
                let ptr = std::alloc::alloc(layout);
                if ptr.is_null() {
                    std::alloc::handle_alloc_error(layout);
                }
                ptr
            }

            #[no_mangle]
            pub unsafe extern "C" fn deallocate(ptr: *mut u8, size: usize, align: usize) {
                let layout = std::alloc::Layout::from_size_align_unchecked(size, align);
                std::alloc::dealloc(ptr, layout);
            }

            #(for func in iface.functions() => #(self.generate_function(&func)))
            #(for func in iface.functions() => #(self.generate_return_struct(&func)))
            #(for obj in iface.objects() => #(for method in obj.methods => #(self.generate_function(&method))))
            #(for obj in iface.objects() => #(for method in obj.methods => #(self.generate_return_struct(&method))))
            #(for obj in iface.objects() => #(self.generate_destructor(&obj)))
        }
    }

    fn generate_function(&self, func: &AbiFunction) -> rust::Tokens {
        let ffi = self.abi.export(func);
        let args = quote!(#(for var in &ffi.args => #(self.var(var)): #(self.ty(&var.ty)),));
        let ret = match &ffi.ret {
            Return::Void => quote!(),
            Return::Num(var) => quote!(-> #(self.ty(&var.ty))),
            Return::Struct(_, name) => quote!(-> #name),
        };
        let return_ = match &ffi.ret {
            Return::Void => quote!(),
            Return::Num(var) => self.var(var),
            Return::Struct(vars, name) => quote! {
                #name {
                    #(for (i, var) in vars.iter().enumerate() => #(format!("ret{}", i)): #(self.var(var)),)
                }
            },
        };
        quote! {
            #[no_mangle]
            pub extern "C" fn #(&ffi.symbol)(#args) #ret {
                panic_abort(move || {
                    #(for instr in &ffi.instr => #(self.instr(instr)))
                    #return_
                })
            }
        }
    }

    fn generate_destructor(&self, obj: &AbiObject) -> rust::Tokens {
        // make destructor compatible with dart by adding an unused `isolate_callback_data` as
        // the first argument.
        let name = format!("drop_box_{}", &obj.name);
        quote! {
            #[no_mangle]
            pub extern "C" fn #name(_: #(self.num_type(self.abi.iptr())), boxed: #(self.num_type(self.abi.iptr()))) {
                panic_abort(move || {
                    unsafe { Box::<#(&obj.name)>::from_raw(boxed as *mut _) };
                });
            }
        }
    }

    fn generate_return_struct(&self, func: &AbiFunction) -> rust::Tokens {
        let ffi = self.abi.export(func);
        if let Return::Struct(vars, name) = &ffi.ret {
            quote! {
                #[repr(C)]
                pub struct #name {
                    #(for (i, var) in vars.iter().enumerate() => #(format!("ret{}", i)): #(self.ty(&var.ty)),)
                }
            }
        } else {
            quote!()
        }
    }

    fn instr(&self, instr: &Instr) -> rust::Tokens {
        match instr {
            Instr::LiftNum(in_, out) | Instr::LiftIsize(in_, out) | Instr::LiftUsize(in_, out) => {
                quote!(let #(self.var(out)) = #(self.var(in_)) as _;)
            }
            Instr::LowerNum(in_, out)
            | Instr::LowerIsize(in_, out)
            | Instr::LowerUsize(in_, out) => quote!(#(self.var(out)) = #(self.var(in_)) as _;),
            Instr::LiftBool(in_, out) => quote!(let #(self.var(out)) = #(self.var(in_)) > 0;),
            Instr::LowerBool(in_, out) => {
                quote!(#(self.var(out)) = if #(self.var(in_)) { 1 } else { 0 };)
            }
            Instr::LiftStr(ptr, len, out) => quote! {
                let #(self.var(out))_0: &[u8] =
                    unsafe { core::slice::from_raw_parts(#(self.var(ptr)) as _, #(self.var(len)) as _) };
                let #(self.var(out)): &str = unsafe { std::str::from_utf8_unchecked(#(self.var(out))_0) };
            },
            Instr::LowerStr(in_, ptr, len) => quote! {
                #(self.var(ptr)) = #(self.var(in_)).as_ptr() as _;
                #(self.var(len)) = #(self.var(in_)).len() as _;
            },
            Instr::LiftString(ptr, len, cap, out) => quote! {
                let #(self.var(out)) = unsafe {
                    String::from_raw_parts(
                        #(self.var(ptr)) as _,
                        #(self.var(len)) as _,
                        #(self.var(cap)) as _,
                    )
                };
            },
            Instr::LowerString(in_, ptr, len, cap) | Instr::LowerVec(in_, ptr, len, cap, _) => {
                quote! {
                    let #(self.var(in_))_0 = std::mem::ManuallyDrop::new(#(self.var(in_)));
                    #(self.var(ptr)) = #(self.var(in_))_0.as_ptr() as _;
                    #(self.var(len)) = #(self.var(in_))_0.len() as _;
                    #(self.var(cap)) = #(self.var(in_))_0.capacity() as _;
                }
            }
            Instr::LiftSlice(ptr, len, out, ty) => quote! {
                let #(self.var(out)): &[#(self.num_type(*ty))] =
                    unsafe { core::slice::from_raw_parts(#(self.var(ptr)) as _, #(self.var(len)) as _) };
            },
            Instr::LowerSlice(in_, ptr, len, _ty) => quote! {
                #(self.var(ptr)) = #(self.var(in_)).as_ptr() as _;
                #(self.var(len)) = #(self.var(in_)).len() as _;
            },
            Instr::LiftVec(ptr, len, cap, out, ty) => quote! {
                let #(self.var(out)) = unsafe {
                    Vec::<#(self.num_type(*ty))>::from_raw_parts(
                        #(self.var(ptr)) as _,
                        #(self.var(len)) as _,
                        #(self.var(cap)) as _,
                    )
                };
            },
            Instr::LiftRefObject(in_, out, object) => quote! {
                let #(self.var(out)) = unsafe { &*(#(self.var(in_)) as *const #object) };
            },
            Instr::LowerRefObject(in_, out) => quote! {
                #(self.var(out)) = #(self.var(in_)) as *const _ as _;
            },
            Instr::LiftObject(in_, out, object) => quote! {
                let #(self.var(out)): Box<#object> = unsafe { Box::from_raw(#(self.var(in_))) };
            },
            Instr::LowerObject(in_, out) => quote! {
                #(self.var(out)) = Box::into_raw(#(self.var(in_))) as _;
            },
            Instr::LowerOption(in_, var, some, some_instr) => quote! {
                #(self.var(var)) = if let Some(#(self.var(some))) = #(self.var(in_)) {
                    #(for instr in some_instr => #(self.instr(instr)))
                    0
                } else {
                    1
                };
            },
            Instr::LowerResult(in_, var, ok, ok_instr, err, err_instr) => quote! {
                #(self.var(var)) = match #(self.var(in_)) {
                    Ok(#(self.var(ok))) => {
                        #(for instr in ok_instr => #(self.instr(instr)))
                        0
                    }
                    Err(#(self.var(err))_0) => {
                        let #(self.var(err)) = #(self.var(err))_0.to_string();
                        #(for instr in err_instr => #(self.instr(instr)))
                        1
                    }
                };
            },
            Instr::CallAbi(ty, self_, name, ret, args) => {
                let invoke = match ty {
                    FunctionType::Constructor(object) => {
                        quote!(#object::#name)
                    }
                    FunctionType::Method(_) => {
                        quote!(#(self.var(self_.as_ref().unwrap())).#name)
                    }
                    FunctionType::Function => {
                        quote!(#name)
                    }
                };
                let args = quote!(#(for arg in args => #(self.var(arg))));
                if let Some(ret) = ret {
                    quote!(let #(self.var(ret)) = #invoke(#args);)
                } else {
                    quote!(#invoke(#args);)
                }
            }
            Instr::DefineRets(vars) => quote! {
                #(for var in vars => #[allow(unused_assignments)] let mut #(self.var(var)) = Default::default();)
            },
        }
    }

    fn var(&self, var: &Var) -> rust::Tokens {
        quote!(#(format!("tmp{}", var.binding)))
    }

    fn ty(&self, ty: &AbiType) -> rust::Tokens {
        match ty {
            AbiType::Num(num) => self.num_type(*num),
            _ => unreachable!(),
        }
    }

    fn num_type(&self, ty: NumType) -> rust::Tokens {
        match ty {
            NumType::U8 => quote!(u8),
            NumType::U16 => quote!(u16),
            NumType::U32 => quote!(u32),
            NumType::U64 => quote!(u64),
            NumType::I8 => quote!(i8),
            NumType::I16 => quote!(i16),
            NumType::I32 => quote!(i32),
            NumType::I64 => quote!(i64),
            NumType::F32 => quote!(f32),
            NumType::F64 => quote!(f64),
        }
    }
}

#[cfg(feature = "test_runner")]
pub mod test_runner {
    use super::*;
    use anyhow::Result;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use trybuild::TestCases;

    pub fn compile_pass(iface: &str, api: rust::Tokens, test: rust::Tokens) -> Result<()> {
        let iface = Interface::parse(iface)?;
        let gen = RustGenerator::new(Abi::native());
        let gen_tokens = gen.generate(iface);
        let tokens = quote! {
            #gen_tokens
            #api
            fn main() {
                #test
            }
        };
        let res = tokens.to_file_string()?;
        let mut tmp = NamedTempFile::new()?;
        tmp.write_all(res.as_bytes())?;
        //println!("{}", res);
        let test = TestCases::new();
        test.pass(tmp.as_ref());
        Ok(())
    }
}
