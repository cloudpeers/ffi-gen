use crate::{Abi, AbiFunction, AbiType, Interface, PrimType};
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
        }
    }

    fn generate_function(&self, func: AbiFunction) -> rust::Tokens {
        quote! {
            #[no_mangle]
            pub extern "C" fn #(format!("__{}", &func.name))(
                #(for (name, ty) in &func.args => #(self.generate_arg(name, ty)))
            ) #(self.generate_return(func.ret.as_ref()))
            {
                #(for (name, ty) in &func.args => #(self.generate_lift(name, ty)))
                let ret = #(&func.name)(#(for (name, _) in &func.args => #name,));
                #(self.generate_lower(func.ret.as_ref()))
            }
        }
    }

    fn generate_arg(&self, name: &str, ty: &AbiType) -> rust::Tokens {
        match ty {
            AbiType::Prim(ty) => quote!(#name: #(self.generate_prim_type(*ty)),),
            AbiType::RefStr => quote! {
                #(name)_ptr: #(self.generate_isize()),
                #(name)_len: #(self.generate_usize()),
            },
            AbiType::String => quote! {
                #(name)_ptr: #(self.generate_isize()),
                #(name)_len: #(self.generate_usize()),
                #(name)_cap: #(self.generate_usize()),
            },
            _ => todo!("arg {:?}", ty),
        }
    }

    fn generate_lift(&self, name: &str, ty: &AbiType) -> rust::Tokens {
        match ty {
            AbiType::Prim(_) => quote!(let #(name) = #(name) as _;),
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
            ty => todo!("lift arg {:?}", ty),
        }
    }

    fn generate_return(&self, ret: Option<&AbiType>) -> rust::Tokens {
        if let Some(ret) = ret {
            match ret {
                AbiType::Prim(ty) => quote!(-> #(self.generate_prim_type(*ty))),
                AbiType::RefStr => quote!(-> #(self.generate_slice())),
                AbiType::String => quote!(-> #(self.generate_alloc())),
                ret => todo!("ret {:?}", ret),
            }
        } else {
            quote!()
        }
    }

    fn generate_lower(&self, ret: Option<&AbiType>) -> rust::Tokens {
        if let Some(ret) = ret {
            match ret {
                AbiType::Prim(_) => quote!(ret as _),
                AbiType::RefStr => quote! {
                    let ret: &str = ret;
                    #(self.generate_slice()) {
                        ptr: ret.as_ptr() as _,
                        len: ret.len() as _,
                    }
                },
                AbiType::String => quote! {
                    use core::mem::ManuallyDrop;
                    let ret = ManuallyDrop::new(ret);
                    #(self.generate_alloc()) {
                        ptr: ret.as_ptr() as _,
                        len: ret.len() as _,
                        cap: ret.capacity() as _,
                    }
                },
                _ => todo!("lower ret {:?}", ret),
            }
        } else {
            quote!(drop(ret))
        }
    }

    fn generate_prim_type(&self, ty: PrimType) -> rust::Tokens {
        match ty {
            PrimType::U8 => quote!(u8),
            PrimType::U16 => quote!(u16),
            PrimType::U32 => quote!(u32),
            PrimType::U64 => quote!(u64),
            PrimType::Usize => self.generate_usize(),
            PrimType::I8 => quote!(i8),
            PrimType::I16 => quote!(i16),
            PrimType::I32 => quote!(i32),
            PrimType::I64 => quote!(i64),
            PrimType::Isize => self.generate_isize(),
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
        let test = TestCases::new();
        test.pass(tmp.as_ref());
        Ok(())
    }
}
