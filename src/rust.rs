use crate::{Abi, AbiFunction, AbiType, FfiType, FunctionType, Interface, NumType};
use genco::prelude::*;
use std::collections::HashSet;

pub struct RustGenerator {
    abi: Abi,
    destructors: HashSet<String>,
}

impl RustGenerator {
    pub fn new(abi: Abi) -> Self {
        Self {
            abi,
            destructors: Default::default(),
        }
    }

    pub fn generate(&mut self, iface: Interface) -> rust::Tokens {
        quote! {
            #[repr(C)]
            pub struct Slice32 {
                pub ptr: i32,
                pub len: u32,
            }

            #[repr(C)]
            pub struct Slice64 {
                pub ptr: i64,
                pub len: u64,
            }

            #[repr(C)]
            pub struct Alloc32 {
                pub ptr: i32,
                pub len: u32,
                pub cap: u32,
            }

            #[repr(C)]
            pub struct Alloc64 {
                pub ptr: i64,
                pub len: u64,
                pub cap: u64,
            }

            #[repr(C)]
            pub enum FfiOption<T> {
                Some(T),
                None,
            }

            #[repr(C)]
            pub enum FfiResult<T, E> {
                Ok(T),
                Err(E),
            }

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

            #(for func in iface.functions() => #(self.generate_function(func)))

            #(for dest in &self.destructors => #(self.generate_destructor(dest)))
        }
    }

    fn generate_function(&mut self, func: AbiFunction) -> rust::Tokens {
        quote! {
            #[no_mangle]
            pub extern "C" fn #(self.generate_func_name(&func))(
                #(self.generate_self(&func))
                #(for (name, ty) in &func.args => #(self.generate_arg(name, ty)))
            ) #(self.generate_return(func.ret.as_ref()))
            {
                panic_abort(move || {
                    #(for (name, ty) in &func.args => #(self.generate_lift(name, ty)))
                    let ret = #(self.generate_invoke(&func))(#(for (name, _) in &func.args => #name,));
                    #(self.generate_lower(func.ret.as_ref()))
                })
            }
        }
    }

    fn generate_func_name(&self, func: &AbiFunction) -> String {
        match &func.ty {
            FunctionType::Constructor(object) | FunctionType::Method(object) => {
                format!("__{}_{}", object, &func.name)
            }
            FunctionType::Function => format!("__{}", &func.name),
        }
    }

    fn generate_invoke(&self, func: &AbiFunction) -> rust::Tokens {
        match &func.ty {
            FunctionType::Constructor(object) => {
                quote!(#(object)::#(&func.name))
            }
            FunctionType::Method(_) => {
                quote!(__self.#(&func.name))
            }
            FunctionType::Function => {
                quote!(#(&func.name))
            }
        }
    }

    fn generate_self(&self, func: &AbiFunction) -> rust::Tokens {
        if let FunctionType::Method(object) = &func.ty {
            quote!(__self: &#object,)
        } else {
            quote!()
        }
    }

    fn generate_destructor(&self, boxed: &str) -> rust::Tokens {
        // make destructor compatible with dart by adding an unused `isolate_callback_data` as
        // the first argument.
        let name = format!("drop_box_{}", boxed);
        quote! {
            #[no_mangle]
            pub extern "C" fn #name(_: *const core::ffi::c_void, boxed: Box<#boxed>) {
                panic_abort(move || {
                    drop(boxed);
                });
            }
        }
    }

    fn generate_arg(&self, name: &str, ty: &AbiType) -> rust::Tokens {
        match ty {
            AbiType::Num(ty) => quote!(#name: #(self.generate_prim_type(*ty)),),
            AbiType::Bool => quote!(#name: bool,),
            AbiType::RefStr | AbiType::RefSlice(_) => quote! {
                #(name)_ptr: #(self.generate_isize()),
                #(name)_len: #(self.generate_usize()),
            },
            AbiType::String | AbiType::Vec(_) => quote! {
                #(name)_ptr: #(self.generate_isize()),
                #(name)_len: #(self.generate_usize()),
                #(name)_cap: #(self.generate_usize()),
            },
            AbiType::RefObject(ident) => quote!(#name: &#ident,),
            AbiType::Object(ident) => quote!(#name: Box<#ident>,),
            AbiType::Option(ty) => {
                let inner = self.generate_arg(name, ty);
                quote!(#(name)_variant: i32, #inner)
            }
            AbiType::Result(ty) => panic!("invalid argument type Result<{:?}>.", ty),
            AbiType::Future(ty) => panic!("invalid argument type Future<{:?}>.", ty),
            AbiType::Stream(ty) => panic!("invalid argument type Stream<{:?}>.", ty),
        }
    }

    fn generate_lift(&self, name: &str, ty: &AbiType) -> rust::Tokens {
        match ty {
            AbiType::Num(_) | AbiType::Bool => quote!(let #(name) = #(name) as _;),
            AbiType::RefSlice(ty) => quote! {
                let #name: &[#(self.generate_prim_type(*ty))] =
                    unsafe { core::slice::from_raw_parts(#(name)_ptr as _, #(name)_len as _) };
            },
            AbiType::RefStr => quote! {
                let #name: &[u8] = unsafe { core::slice::from_raw_parts(#(name)_ptr as _, #(name)_len as _) };
                let #name: &str = unsafe { std::str::from_utf8_unchecked(#name) };
            },
            AbiType::String => quote! {
                let #name = unsafe {
                    String::from_raw_parts(
                        #(name)_ptr as _,
                        #(name)_len as _,
                        #(name)_cap as _,
                    )
                };
            },
            AbiType::Vec(ty) => quote! {
                let #name = unsafe {
                    Vec::<#(self.generate_prim_type(*ty))>::from_raw_parts(
                        #(name)_ptr as _,
                        #(name)_len as _,
                        #(name)_cap as _,
                    )
                };
            },
            AbiType::RefObject(_) | AbiType::Object(_) => quote!(),
            AbiType::Option(ty) => {
                let inner = self.generate_lift(name, ty);
                quote! {
                    let #name = if #(name)_variant == 0 {
                        None
                    } else {
                        #inner
                        Some(#name)
                    };
                }
            }
            AbiType::Result(_) => unreachable!(),
            AbiType::Future(_) => unreachable!(),
            AbiType::Stream(_) => unreachable!(),
        }
    }

    fn generate_return(&mut self, ret: Option<&AbiType>) -> rust::Tokens {
        if let Some(ret) = ret {
            if let AbiType::Object(ident) = ret {
                self.destructors.insert(ident.clone());
            }
            quote!(-> #(self.generate_return_type(ret)))
        } else {
            quote!()
        }
    }

    fn generate_return_type(&self, ret: &AbiType) -> rust::Tokens {
        match ret {
            AbiType::Num(ty) => self.generate_prim_type(*ty),
            AbiType::Bool => quote!(bool),
            AbiType::RefStr | AbiType::RefSlice(_) => self.generate_slice(),
            AbiType::String | AbiType::Vec(_) => self.generate_alloc(),
            AbiType::Object(ident) => quote!(Box<#ident>),
            AbiType::RefObject(ident) => panic!("invalid return type `&{}`", ident),
            AbiType::Option(ty) => quote!(FfiOption<#(self.generate_return_type(ty))>),
            AbiType::Result(ty) => {
                quote!(FfiResult<#(self.generate_return_type(ty)), #(self.generate_alloc())>)
            }
            AbiType::Future(ty) => {
                quote!(Box<dyn Future<Output = #(self.generate_return_type(ty))> + Send + Unpin + 'static>)
            }
            AbiType::Stream(ty) => {
                quote!(Box<dyn Stream<Item = #(self.generate_return_type(ty))> + Send + Unpin + 'static>)
            }
        }
    }

    fn generate_lower(&self, ret: Option<&AbiType>) -> rust::Tokens {
        if let Some(ret) = ret {
            match ret {
                AbiType::Num(_) | AbiType::Bool => quote!(ret as _),
                AbiType::RefStr | AbiType::RefSlice(_) => quote! {
                    #(self.generate_slice()) {
                        ptr: ret.as_ptr() as _,
                        len: ret.len() as _,
                    }
                },
                AbiType::String | AbiType::Vec(_) => quote! {
                    let ret = std::mem::ManuallyDrop::new(ret);
                    #(self.generate_alloc()) {
                        ptr: ret.as_ptr() as _,
                        len: ret.len() as _,
                        cap: ret.capacity() as _,
                    }
                },
                AbiType::Object(_) => quote!(ret),
                AbiType::RefObject(_) => unreachable!(),
                AbiType::Option(_) => quote! {
                    match ret {
                        Some(res) => FfiOption::Some(res),
                        None => FfiOption::None,
                    }
                },
                AbiType::Result(_) => quote! {
                    match ret {
                        Ok(res) => FfiResult::Ok(res),
                        Err(err) => {
                            let ret = std::mem::ManuallyDrop::new(err.to_string());
                            let ret = #(self.generate_alloc()) {
                                ptr: ret.as_ptr() as _,
                                len: ret.len() as _,
                                cap: ret.capacity() as _,
                            };
                            FfiResult::Err(ret)
                        }
                    }
                },
                AbiType::Future(_) => quote!(Box::new(ret)),
                AbiType::Stream(_) => quote!(Box::new(ret)),
            }
        } else {
            quote! {
                #[allow(clippy::drop_copy)]
                drop(ret)
            }
        }
    }

    fn generate_prim_type(&self, ty: NumType) -> rust::Tokens {
        match ty {
            NumType::U8 => quote!(u8),
            NumType::U16 => quote!(u16),
            NumType::U32 => quote!(u32),
            NumType::U64 => quote!(u64),
            NumType::Usize => quote!(usize),
            NumType::I8 => quote!(i8),
            NumType::I16 => quote!(i16),
            NumType::I32 => quote!(i32),
            NumType::I64 => quote!(i64),
            NumType::Isize => quote!(isize),
            NumType::F32 => quote!(f32),
            NumType::F64 => quote!(f64),
        }
    }

    fn generate_usize(&self) -> rust::Tokens {
        match self.abi.ptr() {
            FfiType::I32 => quote!(u32),
            FfiType::I64 => quote!(u64),
            _ => unimplemented!(),
        }
    }

    fn generate_isize(&self) -> rust::Tokens {
        match self.abi.ptr() {
            FfiType::I32 => quote!(i32),
            FfiType::I64 => quote!(i64),
            _ => unimplemented!(),
        }
    }

    fn generate_slice(&self) -> rust::Tokens {
        match self.abi.ptr() {
            FfiType::I32 => quote!(Slice32),
            FfiType::I64 => quote!(Slice64),
            _ => unimplemented!(),
        }
    }

    fn generate_alloc(&self) -> rust::Tokens {
        match self.abi.ptr() {
            FfiType::I32 => quote!(Alloc32),
            FfiType::I64 => quote!(Alloc64),
            _ => unimplemented!(),
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
        let mut gen = RustGenerator::new(Abi::native());
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
