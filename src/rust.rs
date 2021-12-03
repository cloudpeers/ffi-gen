use crate::export::Instr;
use crate::{
    Abi, AbiFunction, AbiFuture, AbiObject, AbiStream, AbiType, FunctionType, Interface, NumType,
    Return, Var,
};
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
            use core::future::Future;
            use core::mem::ManuallyDrop;
            use core::pin::Pin;
            use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
            use std::sync::Arc;

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

            #[inline(always)]
            pub fn assert_send_static<T: Send + 'static>(t: T) -> T {
                t
            }

            pub type Result<T, E = String> = core::result::Result<T, E>;

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

            /// Converts a closure into a [`Waker`].
            ///
            /// The closure gets called every time the waker is woken.
            pub fn waker_fn<F: Fn() + Send + Sync + 'static>(f: F) -> Waker {
                let raw = Arc::into_raw(Arc::new(f)) as *const ();
                let vtable = &Helper::<F>::VTABLE;
                unsafe { Waker::from_raw(RawWaker::new(raw, vtable)) }
            }

            struct Helper<F>(F);

            impl<F: Fn() + Send + Sync + 'static> Helper<F> {
                const VTABLE: RawWakerVTable = RawWakerVTable::new(
                    Self::clone_waker,
                    Self::wake,
                    Self::wake_by_ref,
                    Self::drop_waker,
                );

                unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
                    let arc = ManuallyDrop::new(Arc::from_raw(ptr as *const F));
                    core::mem::forget(arc.clone());
                    RawWaker::new(ptr, &Self::VTABLE)
                }

                unsafe fn wake(ptr: *const ()) {
                    let arc = Arc::from_raw(ptr as *const F);
                    (arc)();
                }

                unsafe fn wake_by_ref(ptr: *const ()) {
                    let arc = ManuallyDrop::new(Arc::from_raw(ptr as *const F));
                    (arc)();
                }

                unsafe fn drop_waker(ptr: *const ()) {
                    drop(Arc::from_raw(ptr as *const F));
                }
            }

            fn ffi_waker(_post_cobject: isize, port: i64) -> Waker {
                waker_fn(move || unsafe {
                    if cfg!(target_family = "wasm") {
                        extern "C" {
                            fn __notifier_callback(idx: i32);
                        }
                        __notifier_callback(port as _);
                    } else {
                        let post_cobject: extern "C" fn(i64, *const core::ffi::c_void) =
                            core::mem::transmute(_post_cobject);
                        let obj: i32 = 0;
                        post_cobject(port, &obj as *const _ as *const _);
                    }
                })
            }

            #[repr(transparent)]
            pub struct FfiFuture<T: Send + 'static>(Pin<Box<dyn Future<Output = T> + Send + 'static>>);

            impl<T: Send + 'static> FfiFuture<T> {
                pub fn new(f: impl Future<Output = T> + Send + 'static) -> Self {
                    Self(Box::pin(f))
                }

                pub fn poll(&mut self, post_cobject: isize, port: i64) -> Option<T> {
                    let waker = ffi_waker(post_cobject, port);
                    let mut ctx = Context::from_waker(&waker);
                    match Pin::new(&mut self.0).poll(&mut ctx) {
                        Poll::Ready(res) => Some(res),
                        Poll::Pending => None,
                    }
                }
            }

            pub trait Stream {
                type Item;

                fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>>;
            }

            impl<T> Stream for Pin<T>
            where
                T: core::ops::DerefMut + Unpin,
                T::Target: Stream,
            {
                type Item = <T::Target as Stream>::Item;

                fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
                    self.get_mut().as_mut().poll_next(cx)
                }
            }

            #[cfg(feature = "futures")]
            impl<T: futures::Stream> Stream for T {
                type Item = T::Item;

                fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
                    futures::Stream::poll_next(self, cx)
                }
            }

            #[repr(transparent)]
            pub struct FfiStream<T: Send + 'static>(Pin<Box<dyn Stream<Item = T> + Send + 'static>>);

            impl<T: Send + 'static> FfiStream<T> {
                pub fn new(f: impl Stream<Item = T> + Send + 'static) -> Self {
                    Self(Box::pin(f))
                }

                pub fn poll(&mut self, post_cobject: isize, port: i64, done: i64) -> Option<T> {
                    let waker = ffi_waker(post_cobject, port);
                    let mut ctx = Context::from_waker(&waker);
                    match Pin::new(&mut self.0).poll_next(&mut ctx) {
                        Poll::Ready(Some(res)) => {
                            ffi_waker(post_cobject, port).wake();
                            Some(res)
                        }
                        Poll::Ready(None) => {
                            ffi_waker(post_cobject, done).wake();
                            None
                        }
                        Poll::Pending => None,
                    }
                }
            }

            #(for func in iface.functions() => #(self.generate_function(&func)))
            #(for obj in iface.objects() => #(self.generate_object(&obj)))
            #(for fut in iface.futures() => #(self.generate_future(&fut)))
            #(for fut in iface.streams() => #(self.generate_stream(&fut)))
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
        let return_struct = if let Return::Struct(_, _) = &ffi.ret {
            self.generate_return_struct(func)
        } else {
            quote!()
        };
        quote! {
            #[no_mangle]
            pub extern "C" fn #(&ffi.symbol)(#args) #ret {
                panic_abort(move || {
                    #(for instr in &ffi.instr => #(self.instr(instr)))
                    #return_
                })
            }
            #return_struct
        }
    }

    fn generate_object(&self, obj: &AbiObject) -> rust::Tokens {
        let destructor_name = format!("drop_box_{}", &obj.name);
        let destructor_type = quote!(#(&obj.name));
        quote! {
            #(for method in &obj.methods => #(self.generate_function(method)))
            #(self.generate_destructor(&destructor_name, destructor_type))
        }
    }

    fn generate_destructor(&self, name: &str, ty: rust::Tokens) -> rust::Tokens {
        // make destructor compatible with dart by adding an unused `isolate_callback_data` as
        // the first argument.
        quote! {
            #[no_mangle]
            pub extern "C" fn #name(_: #(self.num_type(self.abi.iptr())), boxed: #(self.num_type(self.abi.iptr()))) {
                panic_abort(move || {
                    unsafe { Box::<#ty>::from_raw(boxed as *mut _) };
                });
            }
        }
    }

    fn generate_future(&self, fut: &AbiFuture) -> rust::Tokens {
        let destructor_name = format!("{}_future_drop", &fut.symbol);
        let destructor_type = quote!(FfiFuture<#(self.ty(&fut.ty))>);
        quote! {
            #(self.generate_function(&fut.poll()))
            #(self.generate_destructor(&destructor_name, destructor_type))
        }
    }

    fn generate_stream(&self, stream: &AbiStream) -> rust::Tokens {
        let destructor_name = format!("{}_stream_drop", &stream.symbol);
        let destructor_type = quote!(FfiStream<#(self.ty(&stream.ty))>);
        quote! {
            #(self.generate_function(&stream.poll()))
            #(self.generate_destructor(&destructor_name, destructor_type))
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
            Instr::LowerNumAsU32Tuple(r#in, low, high, _num_type) => {
                quote! {
                    #(self.var(low)) = #(self.var(r#in)) as u32;
                    #(self.var(high)) = (#(self.var(r#in)) >> 32) as u32;
                }
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
                    let #(self.var(in_))_0 = ManuallyDrop::new(#(self.var(in_)));
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
                let #(self.var(in_))_0 = assert_send_static(#(self.var(in_)));
                #(self.var(out)) = Box::into_raw(Box::new(#(self.var(in_))_0)) as _;
            },
            Instr::LiftRefFuture(in_, out, ty) => quote! {
                let #(self.var(out)) = unsafe { &mut *(#(self.var(in_)) as *mut FfiFuture<#(self.ty(ty))>) };
            },
            Instr::LiftRefStream(in_, out, ty) => quote! {
                let #(self.var(out)) = unsafe { &mut *(#(self.var(in_)) as *mut FfiStream<#(self.ty(ty))>) };
            },
            Instr::LowerFuture(in_, out, ty) => {
                let future = if let AbiType::Result(_) = ty {
                    quote!(async move { Ok(#(self.var(in_)).await.map_err(|err| err.to_string())?) })
                } else {
                    quote!(#(self.var(in_)))
                };
                quote! {
                    let #(self.var(out))_0 = #future;
                    let #(self.var(out))_1: FfiFuture<#(self.ty(ty))> = FfiFuture::new(#(self.var(out))_0);
                    #(self.var(out)) = Box::into_raw(Box::new(#(self.var(out))_1)) as _;
                }
            }
            Instr::LowerStream(in_, out, ty) => {
                let map_err = if let AbiType::Result(_) = ty {
                    quote!(.map_err(|err| err.to_string()))
                } else {
                    quote!()
                };
                quote! {
                    let #(self.var(out))_0: FfiStream<#(self.ty(ty))> = FfiStream::new(#(self.var(in_))#map_err);
                    #(self.var(out)) = Box::into_raw(Box::new(#(self.var(out))_0)) as _;
                }
            }
            Instr::LowerOption(in_, var, some, some_instr) => quote! {
                if let Some(#(self.var(some))) = #(self.var(in_)) {
                    #(self.var(var)) = 1;
                    #(for instr in some_instr => #(self.instr(instr)))
                } else {
                    #(self.var(var)) = 0;
                }
            },
            Instr::LowerResult(in_, var, ok, ok_instr, err, err_instr) => quote! {
                match #(self.var(in_)) {
                    Ok(#(self.var(ok))) => {
                        #(self.var(var)) = 1;
                        #(for instr in ok_instr => #(self.instr(instr)))
                    }
                    Err(#(self.var(err))_0) => {
                        #(self.var(var)) = 0;
                        let #(self.var(err)) = #(self.var(err))_0.to_string();
                        #(for instr in err_instr => #(self.instr(instr)))
                    }
                };
            },
            Instr::CallAbi(ty, self_, name, ret, args) => {
                let invoke = match ty {
                    FunctionType::Constructor(object) => {
                        quote!(#object::#name)
                    }
                    FunctionType::Method(_)
                    | FunctionType::PollFuture(_, _)
                    | FunctionType::PollStream(_, _) => {
                        quote!(#(self.var(self_.as_ref().unwrap())).#name)
                    }
                    FunctionType::Function => {
                        quote!(#name)
                    }
                };
                let args = quote!(#(for arg in args => #(self.var(arg)),));
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
            AbiType::Isize => quote!(isize),
            AbiType::Usize => quote!(usize),
            AbiType::Bool => quote!(bool),
            AbiType::RefStr => quote!(&str),
            AbiType::String => quote!(String),
            AbiType::RefSlice(ty) => quote!(&[#(self.num_type(*ty))]),
            AbiType::Vec(ty) => quote!(Vec<#(self.num_type(*ty))>),
            AbiType::Option(ty) => quote!(Option<#(self.ty(ty))>),
            AbiType::Result(ty) => quote!(Result<#(self.ty(ty))>),
            AbiType::RefObject(_)
            | AbiType::Object(_)
            | AbiType::RefFuture(_)
            | AbiType::Future(_)
            | AbiType::RefStream(_)
            | AbiType::Stream(_) => unimplemented!(),
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
