use crate::{Abi, AbiFunction, Interface, Type};
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
            pub struct Slice {
                pub ptr: isize,
                pub len: usize,
            }

            #[repr(C)]
            pub struct Slice32 {
                pub ptr: i32,
                pub len: u32,
            }

            #[repr(C)]
            pub struct Alloc {
                pub ptr: isize,
                pub len: usize,
                pub cap: usize,
            }

            #[repr(C)]
            pub struct Alloc32 {
                pub ptr: i32,
                pub len: u32,
                pub cap: u32,
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
                #(for (name, ty) in &func.args => #(self.generate_lift_after_call(name, ty)))
                #(self.generate_lower(func.ret.as_ref()))
            }
        }
    }

    fn generate_arg(&self, name: &str, ty: &Type) -> rust::Tokens {
        match ty {
            Type::U8 => quote!(#name: u8,),
            Type::U16 => quote!(#name: u16,),
            Type::U32 => quote!(#name: u32,),
            Type::U64 => quote!(#name: u64,),
            Type::Usize => quote!(#name: #(self.generate_usize()),),
            Type::I8 => quote!(#name: i8,),
            Type::I16 => quote!(#name: i16,),
            Type::I32 => quote!(#name: i32,),
            Type::I64 => quote!(#name: i64,),
            Type::Isize => quote!(#name: #(self.generate_isize()),),
            Type::Bool => quote!(#name: bool,),
            Type::F32 => quote!(#name: f32,),
            Type::F64 => quote!(#name: f64,),
            Type::String => quote! {
                #(name)_ptr: #(self.generate_isize()),
                #(name)_len: #(self.generate_usize()),
                #(name)_cap: #(self.generate_usize()),
            },
            Type::Ref(inner) => match &**inner {
                Type::String => quote! {
                    #(name)_ptr: #(self.generate_isize()),
                    #(name)_len: #(self.generate_usize()),
                },
                ty => todo!("arg &{:?}", ty),
            },
            Type::Mut(inner) => match &**inner {
                Type::String => {
                    quote!(#(format!("alloc_{}", name)): &mut #(self.generate_alloc()),)
                }
                ty => todo!("arg &mut {:?}", ty),
            },
            _ => todo!("arg {:?}", ty),
        }
    }

    fn generate_lift(&self, name: &str, ty: &Type) -> rust::Tokens {
        match ty {
            Type::U8 => quote!(let #name: u8 = #name;),
            Type::U16 => quote!(let #name: u16 = #name;),
            Type::U32 => quote!(let #name: u32 = #name;),
            Type::U64 => quote!(let #name: u64 = #name;),
            Type::Usize => quote!(let #name = #name;),
            Type::I8 => quote!(let #name: i8 = #name;),
            Type::I16 => quote!(let #name: i16 = #name;),
            Type::I32 => quote!(let #name: i32 = #name;),
            Type::I64 => quote!(let #name: i64 = #name;),
            Type::Isize => quote!(let #name = #name;),
            Type::Bool => quote!(let #name: bool = #name;),
            Type::F32 => quote!(let #name: f32 = #name;),
            Type::F64 => quote!(let #name: f64 = #name;),
            Type::String => quote! {
                let #name = unsafe {
                    String::from_raw_parts(
                        #(name)_ptr as _,
                        #(name)_len as _,
                        #(name)_cap as _,
                    )
                };
            },
            Type::Ref(inner) => match &**inner {
                Type::String => quote! {
                    let #name: &[u8] = unsafe { core::slice::from_raw_parts(#(name)_ptr as _, #(name)_len) };
                    let #name: &str = unsafe { std::str::from_utf8_unchecked(#name) };
                },
                ty => todo!("lift arg &{:?}", ty),
            },
            Type::Mut(inner) => match &**inner {
                Type::String => quote! {
                    let mut #name = unsafe {
                        use core::mem::ManuallyDrop;
                        ManuallyDrop::new(String::from_raw_parts(
                            #(format!("alloc_{}", name)).ptr as _,
                            #(format!("alloc_{}", name)).len as _,
                            #(format!("alloc_{}", name)).cap as _,
                        ))
                    };
                    let #name: &mut String = &mut #name;
                },
                ty => todo!("lift arg &mut {:?}", ty),
            },
            ty => todo!("lift arg {:?}", ty),
        }
    }

    fn generate_lift_after_call(&self, name: &str, ty: &Type) -> rust::Tokens {
        match ty {
            Type::Mut(inner) => match &**inner {
                Type::String => quote! {
                    #(format!("alloc_{}", name)).ptr = #name.as_ptr() as _;
                    #(format!("alloc_{}", name)).len = #name.len() as _;
                    #(format!("alloc_{}", name)).cap = #name.capacity() as _;
                },
                _ => quote!(),
            },
            _ => quote!(),
        }
    }

    fn generate_return(&self, ret: Option<&Type>) -> rust::Tokens {
        if let Some(ret) = ret {
            match ret {
                Type::U8 => quote!(-> u8),
                Type::U16 => quote!(-> u16),
                Type::U32 => quote!(-> u32),
                Type::U64 => quote!(-> u64),
                Type::Usize => quote!(-> #(self.generate_usize())),
                Type::I8 => quote!(-> i8),
                Type::I16 => quote!(-> i16),
                Type::I32 => quote!(-> i32),
                Type::I64 => quote!(-> i64),
                Type::Isize => quote!(-> #(self.generate_isize())),
                Type::Bool => quote!(-> bool),
                Type::F32 => quote!(-> f32),
                Type::F64 => quote!(-> f64),
                Type::String => quote!(-> #(self.generate_alloc())),
                Type::Ref(inner) => match &**inner {
                    Type::String => quote!(-> #(self.generate_slice())),
                    ty => todo!("ret &{:?}", ty),
                },
                Type::Mut(_) => panic!("can't return mutable references."),
                ret => todo!("ret {:?}", ret),
            }
        } else {
            quote!()
        }
    }

    fn generate_lower(&self, ret: Option<&Type>) -> rust::Tokens {
        if let Some(ret) = ret {
            match ret {
                Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::Bool
                | Type::F32
                | Type::F64 => quote!(ret),
                Type::Usize | Type::Isize => quote!(ret as _),
                Type::String => {
                    quote! {
                        use core::mem::ManuallyDrop;
                        let ret = ManuallyDrop::new(ret);
                        #(self.generate_alloc()) {
                            ptr: ret.as_ptr() as _,
                            len: ret.len() as _,
                            cap: ret.capacity() as _,
                        }
                    }
                }
                Type::Ref(inner) => match &**inner {
                    Type::String => quote! {
                        let ret: &str = ret;
                        #(self.generate_slice()) {
                            ptr: ret.as_ptr() as _,
                            len: ret.len() as _,
                        }
                    },
                    ty => todo!("lower ret &{:?}", ty),
                },
                Type::Mut(_) => panic!("can't return mutable references."),
                _ => todo!("lower ret {:?}", ret),
            }
        } else {
            quote!(drop(ret))
        }
    }

    fn generate_usize(&self) -> rust::Tokens {
        match self.abi {
            Abi::Native => quote!(usize),
            Abi::Wasm32 => quote!(u32),
        }
    }

    fn generate_isize(&self) -> rust::Tokens {
        match self.abi {
            Abi::Native => quote!(isize),
            Abi::Wasm32 => quote!(i32),
        }
    }

    fn generate_slice(&self) -> rust::Tokens {
        match self.abi {
            Abi::Native => quote!(Slice),
            Abi::Wasm32 => quote!(Slice32),
        }
    }

    fn generate_alloc(&self) -> rust::Tokens {
        match self.abi {
            Abi::Native => quote!(Alloc),
            Abi::Wasm32 => quote!(Alloc32),
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
        let gen = RustGenerator::new(Abi::Native);
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
