use crate::{Abi, AbiFunction, AbiType, Interface, PrimType};
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

            #(for func in iface.into_functions() => #(self.generate_function(func)))

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
        if let Some(object) = func.object.as_ref() {
            format!("__{}_{}", object, &func.name)
        } else {
            format!("__{}", &func.name)
        }
    }

    fn generate_invoke(&self, func: &AbiFunction) -> rust::Tokens {
        if let Some(object) = func.object.as_ref() {
            if func.is_static {
                quote!(#(object)::#(&func.name))
            } else {
                quote!(__self.#(&func.name))
            }
        } else {
            quote!(#(&func.name))
        }
    }

    fn generate_self(&self, func: &AbiFunction) -> rust::Tokens {
        if let Some(object) = func.object.as_ref() {
            if !func.is_static {
                return quote!(__self: &#object,);
            }
        }
        quote!()
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
            AbiType::Prim(ty) => quote!(#name: #(self.generate_prim_type(*ty)),),
            AbiType::RefStr | AbiType::RefSlice(_) => quote! {
                #(name)_ptr: #(self.generate_isize()),
                #(name)_len: #(self.generate_usize()),
            },
            AbiType::String | AbiType::Vec(_) => quote! {
                #(name)_ptr: #(self.generate_isize()),
                #(name)_len: #(self.generate_usize()),
                #(name)_cap: #(self.generate_usize()),
            },
            AbiType::Box(ident) => quote!(#name: Box<#ident>,),
            AbiType::Ref(ident) => quote!(#name: &#ident,),
            AbiType::Object(ident) => quote!(#name: Box<#ident>,),
        }
    }

    fn generate_lift(&self, name: &str, ty: &AbiType) -> rust::Tokens {
        match ty {
            AbiType::Prim(_) => quote!(let #(name) = #(name) as _;),
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
            AbiType::Box(_) | AbiType::Ref(_) | AbiType::Object(_) => quote!(),
        }
    }

    fn generate_return(&mut self, ret: Option<&AbiType>) -> rust::Tokens {
        if let Some(ret) = ret {
            match ret {
                AbiType::Prim(ty) => quote!(-> #(self.generate_prim_type(*ty))),
                AbiType::RefStr | AbiType::RefSlice(_) => quote!(-> #(self.generate_slice())),
                AbiType::String | AbiType::Vec(_) => quote!(-> #(self.generate_alloc())),
                AbiType::Box(ident) | AbiType::Object(ident) => {
                    self.destructors.insert(ident.clone());
                    quote!(-> Box<#ident>)
                }
                AbiType::Ref(ident) => panic!("invalid return type `&{}`", ident),
            }
        } else {
            quote!()
        }
    }

    fn generate_lower(&self, ret: Option<&AbiType>) -> rust::Tokens {
        if let Some(ret) = ret {
            match ret {
                AbiType::Prim(_) => quote!(ret as _),
                AbiType::RefStr | AbiType::RefSlice(_) => quote! {
                    #(self.generate_slice()) {
                        ptr: ret.as_ptr() as _,
                        len: ret.len() as _,
                    }
                },
                AbiType::String | AbiType::Vec(_) => quote! {
                    use std::mem::ManuallyDrop;
                    let ret = ManuallyDrop::new(ret);
                    #(self.generate_alloc()) {
                        ptr: ret.as_ptr() as _,
                        len: ret.len() as _,
                        cap: ret.capacity() as _,
                    }
                },
                AbiType::Box(_) | AbiType::Object(_) => quote!(ret),
                AbiType::Ref(_) => unreachable!(),
            }
        } else {
            quote! {
                #[allow(clippy::drop_copy)]
                drop(ret)
            }
        }
    }

    fn generate_prim_type(&self, ty: PrimType) -> rust::Tokens {
        match ty {
            PrimType::U8 => quote!(u8),
            PrimType::U16 => quote!(u16),
            PrimType::U32 => quote!(u32),
            PrimType::U64 => quote!(u64),
            PrimType::Usize => quote!(usize),
            PrimType::I8 => quote!(i8),
            PrimType::I16 => quote!(i16),
            PrimType::I32 => quote!(i32),
            PrimType::I64 => quote!(i64),
            PrimType::Isize => quote!(isize),
            PrimType::Bool => quote!(bool),
            PrimType::F32 => quote!(f32),
            PrimType::F64 => quote!(f64),
        }
    }

    fn generate_usize(&self) -> rust::Tokens {
        match self.abi.ptr_width() {
            32 => quote!(u32),
            64 => quote!(u64),
            _ => unimplemented!(),
        }
    }

    fn generate_isize(&self) -> rust::Tokens {
        match self.abi.ptr_width() {
            32 => quote!(i32),
            64 => quote!(i64),
            _ => unimplemented!(),
        }
    }

    fn generate_slice(&self) -> rust::Tokens {
        match self.abi.ptr_width() {
            32 => quote!(Slice32),
            64 => quote!(Slice64),
            _ => unimplemented!(),
        }
    }

    fn generate_alloc(&self) -> rust::Tokens {
        match self.abi.ptr_width() {
            32 => quote!(Alloc32),
            64 => quote!(Alloc64),
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
