use crate::import::{Import, Instr};
use crate::{Abi, AbiFunction, AbiObject, AbiType, FunctionType, Interface, NumType, Return, Var};
use genco::prelude::*;
use genco::tokens::static_literal;
use heck::*;

pub struct DartGenerator {
    abi: Abi,
    cdylib_name: String,
}

impl DartGenerator {
    pub fn new(cdylib_name: String) -> Self {
        Self {
            abi: Abi::native(),
            cdylib_name,
        }
    }

    pub fn generate(&self, iface: Interface) -> dart::Tokens {
        quote! {
            #(static_literal("//")) AUTO GENERATED FILE, DO NOT EDIT.
            #(static_literal("//"))
            #(static_literal("//")) Generated by "ffi-gen".

            import "dart:async";
            import "dart:convert";
            import "dart:ffi" as ffi;
            import "dart:io" show Platform;
            import "dart:isolate";
            import "dart:typed_data";

            ffi.Pointer<ffi.Void> _registerFinalizer(_Box boxed) {
                final dart = ffi.DynamicLibrary.executable();
                final registerPtr = dart.lookup<ffi.NativeFunction<ffi.Pointer<ffi.Void> Function(
                    ffi.Handle, ffi.Pointer<ffi.Void>, ffi.IntPtr, ffi.Pointer<ffi.Void>)>>("Dart_NewFinalizableHandle");
                final register = registerPtr
                    .asFunction<ffi.Pointer<ffi.Void> Function(
                        Object, ffi.Pointer<ffi.Void>, int, ffi.Pointer<ffi.Void>)>();
                return register(boxed, boxed._ptr, 42, boxed._dropPtr.cast());
            }

            void _unregisterFinalizer(_Box boxed) {
                final dart = ffi.DynamicLibrary.executable();
                final unregisterPtr = dart.lookup<ffi.NativeFunction<ffi.Void Function(
                    ffi.Pointer<ffi.Void>, ffi.Handle)>>("Dart_DeleteFinalizableHandle");
                final unregister = unregisterPtr
                    .asFunction<void Function(ffi.Pointer<ffi.Void>, _Box)>();
                unregister(boxed._finalizer, boxed);
            }

            class _Box {
                final Api _api;
                final ffi.Pointer<ffi.Void> _ptr;
                final String _dropSymbol;
                bool _dropped;
                bool _moved;
                ffi.Pointer<ffi.Void> _finalizer = ffi.Pointer.fromAddress(0);

                _Box(this._api, this._ptr, this._dropSymbol) : _dropped = false, _moved = false;

                late final _dropPtr = _api._lookup<
                    ffi.NativeFunction<
                        ffi.Void Function(ffi.Pointer<ffi.Void>, ffi.Pointer<ffi.Void>)>>(_dropSymbol);

                late final _drop = _dropPtr.asFunction<
                    void Function(ffi.Pointer<ffi.Void>, ffi.Pointer<ffi.Void>)>();

                int borrow() {
                    if (_dropped) {
                        throw StateError("use after free");
                    }
                    if (_moved) {
                        throw StateError("use after move");
                    }
                    return _ptr.address;
                }

                int move() {
                    if (_dropped) {
                        throw StateError("use after free");
                    }
                    if (_moved) {
                        throw StateError("can't move value twice");
                    }
                    _moved = true;
                    _unregisterFinalizer(this);
                    return _ptr.address;
                }

                void drop() {
                    if (_dropped) {
                        throw StateError("double free");
                    }
                    if (_moved) {
                        throw StateError("can't drop moved value");
                    }
                    _dropped = true;
                    _unregisterFinalizer(this);
                    _drop(ffi.Pointer.fromAddress(0), _ptr);
                }
            }

            Future<T> _nativeFuture<T>(_Box box, T? Function(int, int, int) nativePoll) {
                final completer = Completer<T>();
                final rx = ReceivePort();
                void poll() {
                    try {
                        final ret = nativePoll(box.borrow(), ffi.NativeApi.postCObject.address, rx.sendPort.nativePort);
                        if (ret == null) {
                            return;
                        }
                        completer.complete(ret);
                    } catch(err) {
                        completer.completeError(err);
                    }
                    rx.close();
                    box.drop();
                }
                rx.listen((dynamic _message) => poll());
                poll();
                return completer.future;
            }

            Stream<T> _nativeStream<T>(_Box box, T? Function(int, int, int, int) nativePoll) {
                final controller = StreamController<T>();
                final rx = ReceivePort();
                final done = ReceivePort();
                void poll() {
                    try {
                        final ret = nativePoll(
                            box.borrow(),
                            ffi.NativeApi.postCObject.address,
                            rx.sendPort.nativePort,
                            done.sendPort.nativePort,
                        );
                        if (ret != null) {
                            controller.add(ret);
                        }
                    } catch(err) {
                        controller.addError(err);
                    }
                }
                rx.listen((dynamic _message) => poll());
                done.listen((dynamic _message) {
                    rx.close();
                    done.close();
                    controller.close();
                    box.drop();
                });
                poll();
                return controller.stream;
            }

            class Api {
                #(static_literal("///")) Holds the symbol lookup function.
                final ffi.Pointer<T> Function<T extends ffi.NativeType>(String symbolName)
                    _lookup;

                #(static_literal("///")) The symbols are looked up in [dynamicLibrary].
                Api(ffi.DynamicLibrary dynamicLibrary)
                    : _lookup = dynamicLibrary.lookup;

                #(static_literal("///")) The symbols are looked up with [lookup].
                Api.fromLookup(
                    ffi.Pointer<T> Function<T extends ffi.NativeType>(String symbolName)
                        lookup)
                    : _lookup = lookup;

                #(static_literal("///")) The library is loaded from the executable.
                factory Api.loadStatic() {
                    return Api(ffi.DynamicLibrary.executable());
                }

                #(static_literal("///")) The library is dynamically loaded.
                factory Api.loadDynamic(String name) {
                    return Api(ffi.DynamicLibrary.open(name));
                }

                #(static_literal("///")) The library is loaded based on platform conventions.
                factory Api.load() {
                    String? name;
                    if (Platform.isLinux) name = #_(#("lib")#(&self.cdylib_name)#(".so"));
                    if (Platform.isAndroid) name = #_(#("lib")#(&self.cdylib_name)#(".so"));
                    if (Platform.isMacOS) name = #_(#("lib")#(&self.cdylib_name)#(".dylib"));
                    if (Platform.isIOS) name = #_("");
                    if (Platform.isWindows) #_(#(&self.cdylib_name)#(".dll"));
                    if (name == null) {
                        throw UnsupportedError(#_("This platform is not supported."));
                    }
                    if (name == "") {
                        return Api.loadStatic();
                    } else {
                        return Api.loadDynamic(name);
                    }
                }

                ffi.Pointer<T> __allocate<T extends ffi.NativeType>(int byteCount, int alignment) {
                    return _allocate(byteCount, alignment).cast();
                }

                void __deallocate<T extends ffi.NativeType>(ffi.Pointer pointer, int byteCount, int alignment) {
                    _deallocate(pointer.cast(), byteCount, alignment);
                }

                #(for func in iface.functions() => #(self.generate_function(&func)))

                late final _allocatePtr = _lookup<
                    ffi.NativeFunction<
                        ffi.Pointer<ffi.Uint8> Function(ffi.IntPtr, ffi.IntPtr)>>("allocate");

                late final _allocate = _allocatePtr.asFunction<
                    ffi.Pointer<ffi.Uint8> Function(int, int)>();

                late final _deallocatePtr = _lookup<
                    ffi.NativeFunction<
                        ffi.Void Function(ffi.Pointer<ffi.Uint8>, ffi.IntPtr, ffi.IntPtr)>>("deallocate");

                late final _deallocate = _deallocatePtr.asFunction<
                    void Function(ffi.Pointer<ffi.Uint8>, int, int)>();

                #(for fut in iface.futures() => #(self.generate_function(&fut.poll())))
                #(for stream in iface.streams() => #(self.generate_function(&stream.poll())))

                #(for func in iface.imports(&self.abi) => #(self.generate_wrapper(func)))
            }

            #(for obj in iface.objects() => #(self.generate_object(obj)))

            #(for func in iface.imports(&self.abi) => #(self.generate_return_struct(&func.ret)))
        }
    }

    fn generate_object(&self, obj: AbiObject) -> dart::Tokens {
        quote! {
            class #(&obj.name) {
                final Api _api;
                final _Box _box;

                #(&obj.name)._(this._api, this._box);

                #(for func in &obj.methods => #(self.generate_function(func)))

                void drop() {
                    _box.drop();
                }
            }
        }
    }

    fn generate_function(&self, func: &AbiFunction) -> dart::Tokens {
        let ffi = self.abi.import(func);
        let api = match &func.ty {
            FunctionType::Constructor(_) => "api",
            FunctionType::Method(_) => "_api",
            FunctionType::Function
            | FunctionType::PollFuture(_, _)
            | FunctionType::PollStream(_, _) => "this",
        };
        let boxed = match &func.ty {
            FunctionType::PollFuture(_, _) | FunctionType::PollStream(_, _) => quote!(int boxed,),
            _ => quote!(),
        };
        let name = match &func.ty {
            FunctionType::PollFuture(_, _) | FunctionType::PollStream(_, _) => {
                format!("__{}", self.ident(&ffi.symbol))
            }
            _ => self.ident(&func.name),
        };
        let args = quote!(#(for (name, ty) in &func.args => #(self.generate_type(ty)) #(self.ident(name)),));
        let body = quote!(#(for instr in &ffi.instr => #(self.generate_instr(api, instr))));
        let ret = if let Some(ret) = func.ret.as_ref() {
            self.generate_type(ret)
        } else {
            quote!(void)
        };
        match &func.ty {
            FunctionType::Constructor(_object) => quote! {
                static #ret #name(Api api, #args) {
                    #body
                }
            },
            _ => {
                quote! {
                    #ret #name(#(boxed)#args) {
                        #body
                    }
                }
            }
        }
    }

    fn generate_instr(&self, api: &str, instr: &Instr) -> dart::Tokens {
        match instr {
            Instr::BorrowSelf(out) => quote!(final #(self.var(out)) = _box.borrow();),
            Instr::BorrowObject(in_, out) => {
                quote!(final #(self.var(out)) = #(self.var(in_))._box.borrow();)
            }
            Instr::MoveObject(in_, out)
            | Instr::MoveFuture(in_, out)
            | Instr::MoveStream(in_, out) => {
                quote!(final #(self.var(out)) = #(self.var(in_))._box.move();)
            }
            Instr::LiftObject(obj, box_, drop, out) => quote! {
                final ffi.Pointer<ffi.Void> #(self.var(box_))_0 = ffi.Pointer.fromAddress(#(self.var(box_)));
                final #(self.var(box_))_1 = _Box(#api, #(self.var(box_))_0, #_(#drop));
                #(self.var(box_))_1._finalizer = _registerFinalizer(#(self.var(box_))_1);
                final #(self.var(out)) = #obj._(#api, #(self.var(box_))_1);
            },
            Instr::BindArg(arg, out) => quote!(final #(self.var(out)) = #(self.ident(arg));),
            Instr::BindRets(ret, vars) => {
                if vars.len() > 1 {
                    quote! {
                        #(for (idx, var) in vars.iter().enumerate() =>
                            final #(self.var(var)) = #(self.var(ret)).#(format!("arg{}", idx));)
                    }
                } else {
                    quote!(final #(self.var(&vars[0])) = #(self.var(ret));)
                }
            }
            Instr::LowerNum(in_, out, _num) | Instr::LiftNum(in_, out, _num) => {
                quote!(final #(self.var(out)) = #(self.var(in_));)
            }
            Instr::LowerBool(in_, out) => {
                quote!(final #(self.var(out)) = #(self.var(in_)) ? 1 : 0;)
            }
            Instr::LiftBool(in_, out) => {
                quote!(final #(self.var(out)) = #(self.var(in_)) > 0;)
            }
            Instr::Deallocate(ptr, len, size, align) => quote! {
                if (#(self.var(len)) > 0) {
                    final ffi.Pointer<ffi.Void> #(self.var(ptr))_0;
                    #(self.var(ptr))_0 = ffi.Pointer.fromAddress(#(self.var(ptr)));
                    #api.__deallocate(#(self.var(ptr))_0, #(self.var(len)) * #(*size), #(*align));
                }
            },
            Instr::LowerString(in_, ptr, len, size, align) => quote! {
                final #(self.var(in_))_0 = utf8.encode(#(self.var(in_)));
                final #(self.var(len)) = #(self.var(in_))_0.length;
                final ffi.Pointer<ffi.Uint8> #(self.var(ptr))_0 =
                    #api.__allocate(#(self.var(len)) * #(*size), #(*align));
                final Uint8List #(self.var(ptr))_1 = #(self.var(ptr))_0.asTypedList(#(self.var(len)));
                #(self.var(ptr))_1.setAll(0, #(self.var(in_))_0);
                final #(self.var(ptr)) = #(self.var(ptr))_0.address;
            },
            Instr::LiftString(ptr, len, out) => quote! {
                final ffi.Pointer<ffi.Uint8> #(self.var(ptr))_0 = ffi.Pointer.fromAddress(#(self.var(ptr)));
                final #(self.var(out)) = utf8.decode(#(self.var(ptr))_0.asTypedList(#(self.var(len))));
            },
            Instr::LowerVec(in_, ptr, len, ty, size, align) => quote! {
                final #(self.var(len)) = #(self.var(in_)).length;
                final ffi.Pointer<#(self.generate_native_num_type(*ty))> #(self.var(ptr))_0 =
                    #api.__allocate(#(self.var(len)) * #(*size), #(*align));
                final #(self.var(ptr))_1 = #(self.var(ptr))_0.asTypedList(#(self.var(len)));
                #(self.var(ptr))_1.setAll(0, #(self.var(in_)));
                final #(self.var(ptr)) = #(self.var(ptr))_0.address;
            },
            Instr::LiftVec(ptr, len, out, ty) => quote! {
                final ffi.Pointer<#(self.generate_native_num_type(*ty))> #(self.var(ptr))_0 =
                    ffi.Pointer.fromAddress(#(self.var(ptr)));
                final #(self.var(out)) = #(self.var(ptr))_0.asTypedList(#(self.var(len))).toList();
            },
            Instr::Call(symbol, ret, args) => {
                let api = if api == "this" {
                    quote!()
                } else {
                    quote!(#api.)
                };
                let invoke = quote!(#(api)#(format!("_{}", self.ident(symbol)))(#(for arg in args => #(self.var(arg)),)););
                if let Some(ret) = ret {
                    quote!(final #(self.var(ret)) = #invoke)
                } else {
                    invoke
                }
            }
            Instr::ReturnValue(ret) => quote!(return #(self.var(ret));),
            Instr::ReturnVoid => quote!(return;),
            Instr::HandleNull(var) => quote! {
                if (#(self.var(var)) == 0) {
                    return null;
                }
            },
            Instr::HandleError(var, ptr, len, cap) => quote! {
                if (#(self.var(var)) == 0) {
                    final ffi.Pointer<ffi.Uint8> #(self.var(ptr))_0 = ffi.Pointer.fromAddress(#(self.var(ptr)));
                    final #(self.var(var))_0 = utf8.decode(#(self.var(ptr))_0.asTypedList(#(self.var(len))));
                    if (#(self.var(len)) > 0) {
                        final ffi.Pointer<ffi.Void> #(self.var(ptr))_0;
                        #(self.var(ptr))_0 = ffi.Pointer.fromAddress(#(self.var(ptr)));
                        #api.__deallocate(#(self.var(ptr))_0, #(self.var(cap)), 1);
                    }
                    throw #(self.var(var))_0;
                }
            },
            Instr::LiftFuture(box_, poll, drop, out) => quote! {
                final ffi.Pointer<ffi.Void> #(self.var(box_))_0 = ffi.Pointer.fromAddress(#(self.var(box_)));
                final #(self.var(box_))_1 = _Box(#api, #(self.var(box_))_0, #_(#drop));
                #(self.var(box_))_1._finalizer = _registerFinalizer(#(self.var(box_))_1);
                final #(self.var(out)) = _nativeFuture(#(self.var(box_))_1, #api.#(format!("__{}", self.ident(poll))));
            },
            Instr::LiftStream(box_, poll, drop, out) => quote! {
                final ffi.Pointer<ffi.Void> #(self.var(box_))_0 = ffi.Pointer.fromAddress(#(self.var(box_)));
                final #(self.var(box_))_1 = _Box(#api, #(self.var(box_))_0, #_(#drop));
                #(self.var(box_))_1._finalizer = _registerFinalizer(#(self.var(box_))_1);
                final #(self.var(out)) = _nativeStream(#(self.var(box_))_1, #api.#(format!("__{}", self.ident(poll))));
            },
        }
    }

    fn var(&self, var: &Var) -> dart::Tokens {
        quote!(#(format!("tmp{}", var.binding)))
    }

    fn generate_wrapper(&self, func: Import) -> dart::Tokens {
        let native_args =
            quote!(#(for var in &func.args => #(self.generate_native_num_type(var.ty.num())),));
        let wrapped_args =
            quote!(#(for var in &func.args => #(self.generate_wrapped_num_type(var.ty.num())),));
        let native_ret = self.generate_native_return_type(&func.ret);
        let wrapped_ret = self.generate_wrapped_return_type(&func.ret);
        let symbol_ptr = format!("_{}Ptr", self.ident(&func.symbol));
        quote! {
            late final #(&symbol_ptr) =
                _lookup<ffi.NativeFunction<#native_ret Function(#native_args)>>(#_(#(&func.symbol)));

            late final #(format!("_{}", self.ident(&func.symbol))) =
                #symbol_ptr.asFunction<#wrapped_ret Function(#wrapped_args)>();
        }
    }

    fn generate_type(&self, ty: &AbiType) -> dart::Tokens {
        match ty {
            AbiType::Num(ty) => self.generate_wrapped_num_type(*ty),
            AbiType::Isize | AbiType::Usize => quote!(int),
            AbiType::Bool => quote!(bool),
            AbiType::RefStr | AbiType::String => quote!(String),
            AbiType::RefSlice(ty) | AbiType::Vec(ty) => {
                quote!(List<#(self.generate_wrapped_num_type(*ty))>)
            }
            AbiType::Object(ty) | AbiType::RefObject(ty) => quote!(#ty),
            AbiType::Option(ty) => quote!(#(self.generate_type(&**ty))?),
            AbiType::Result(ty) => self.generate_type(&**ty),
            AbiType::RefFuture(_) => todo!(),
            AbiType::Future(_) => quote!(Future),
            AbiType::RefStream(_) => todo!(),
            AbiType::Stream(_) => quote!(Stream),
        }
    }

    fn generate_wrapped_num_type(&self, ty: NumType) -> dart::Tokens {
        match ty {
            NumType::F32 | NumType::F64 => quote!(double),
            _ => quote!(int),
        }
    }

    fn generate_native_num_type(&self, ty: NumType) -> dart::Tokens {
        match ty {
            NumType::I8 => quote!(ffi.Int8),
            NumType::I16 => quote!(ffi.Int16),
            NumType::I32 => quote!(ffi.Int32),
            NumType::I64 => quote!(ffi.Int64),
            NumType::U8 => quote!(ffi.Uint8),
            NumType::U16 => quote!(ffi.Uint16),
            NumType::U32 => quote!(ffi.Uint32),
            NumType::U64 => quote!(ffi.Uint64),
            NumType::F32 => quote!(ffi.Float),
            NumType::F64 => quote!(ffi.Double),
        }
    }

    fn generate_native_return_type(&self, ret: &Return) -> dart::Tokens {
        match ret {
            Return::Void => quote!(ffi.Void),
            Return::Num(var) => self.generate_native_num_type(var.ty.num()),
            Return::Struct(_, s) => quote!(#(format!("_{}", self.type_ident(s)))),
        }
    }

    fn generate_wrapped_return_type(&self, ret: &Return) -> dart::Tokens {
        match ret {
            Return::Void => quote!(void),
            Return::Num(var) => self.generate_wrapped_num_type(var.ty.num()),
            Return::Struct(_, s) => quote!(#(format!("_{}", self.type_ident(s)))),
        }
    }

    fn generate_return_struct(&self, ret: &Return) -> dart::Tokens {
        if let Return::Struct(vars, name) = ret {
            quote! {
                class #(format!("_{}", self.type_ident(name))) extends ffi.Struct {
                    #(for (i, var) in vars.iter().enumerate() => #(self.generate_return_struct_field(i, var.ty.num())))
                }
            }
        } else {
            quote!()
        }
    }

    fn generate_return_struct_field(&self, i: usize, ty: NumType) -> dart::Tokens {
        quote! {
            @#(self.generate_native_num_type(ty))()
            external #(self.generate_wrapped_num_type(ty)) #(format!("arg{}", i));
        }
    }

    fn type_ident(&self, s: &str) -> String {
        s.to_camel_case()
    }

    fn ident(&self, s: &str) -> String {
        s.to_mixed_case()
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

    pub fn compile_pass(iface: &str, rust: rust::Tokens, dart: dart::Tokens) -> Result<()> {
        let iface = Interface::parse(iface)?;
        let mut rust_file = NamedTempFile::new()?;
        let rust_gen = RustGenerator::new(Abi::native());
        let rust_tokens = rust_gen.generate(iface.clone());
        let mut dart_file = NamedTempFile::new()?;
        let dart_gen = DartGenerator::new("compile_pass".to_string());
        let dart_tokens = dart_gen.generate(iface);

        let library_tokens = quote! {
            #rust_tokens
            #rust
        };

        let bin_tokens = quote! {
            #dart_tokens

            extension on List {
                bool equals(List list) {
                    if (this.length != list.length) return false;
                    for (int i = 0; i < this.length; i++) {
                        if (this[i] != list[i]) {
                            return false;
                        }
                    }
                    return true;
                }
            }

            void main() async {
                final api = Api.load();
                #dart
            }
        };

        let library = library_tokens.to_file_string()?;
        rust_file.write_all(library.as_bytes())?;
        let bin = bin_tokens.to_file_string()?;
        dart_file.write_all(bin.as_bytes())?;

        let library_dir = tempfile::tempdir()?;
        let library_file = library_dir.as_ref().join("libcompile_pass.so");
        let runner_tokens: rust::Tokens = quote! {
            fn main() {
                use std::process::Command;
                let ret = Command::new("rustc")
                    .arg("--edition")
                    .arg("2021")
                    .arg("--crate-name")
                    .arg("compile_pass")
                    .arg("--crate-type")
                    .arg("cdylib")
                    .arg("-o")
                    .arg(#(quoted(library_file.as_path().to_str().unwrap())))
                    .arg(#(quoted(rust_file.as_ref().to_str().unwrap())))
                    .status()
                    .unwrap()
                    .success();
                assert!(ret);
                //println!("{}", #_(#bin));
                let ret = Command::new("dart")
                    .env("LD_LIBRARY_PATH", #(quoted(library_dir.as_ref().to_str().unwrap())))
                    .arg("--enable-asserts")
                    //.arg("--observe")
                    //.arg("--write-service-info=service.json")
                    .arg(#(quoted(dart_file.as_ref().to_str().unwrap())))
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
}
