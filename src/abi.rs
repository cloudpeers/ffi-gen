use crate::parser::{Interface, Type};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FfiType {
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

#[derive(Clone, Copy, Debug)]
pub enum NumType {
    U8,
    U16,
    U32,
    U64,
    Usize,
    I8,
    I16,
    I32,
    I64,
    Isize,
    F32,
    F64,
}

#[derive(Clone, Debug)]
pub enum AbiType {
    Num(NumType),
    Bool,
    RefStr,
    String,
    RefSlice(NumType),
    Vec(NumType),
    RefObject(String),
    Object(String),
    Option(Box<AbiType>),
    Result(Box<AbiType>),
    Future(Box<AbiType>),
    Stream(Box<AbiType>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Abi {
    Native(FfiType),
    Wasm(FfiType),
}

impl Abi {
    pub fn native() -> Self {
        #[cfg(target_pointer_width = "32")]
        return Abi::Native(FfiType::I32);
        #[cfg(target_pointer_width = "64")]
        return Abi::Native(FfiType::I64);
    }

    pub fn ptr(self) -> FfiType {
        match self {
            Self::Native(ptr) => ptr,
            Self::Wasm(ptr) => ptr,
        }
    }

    /// Returns the size and alignment of a primitive type.
    pub fn layout(self, ty: NumType) -> (usize, usize) {
        let size = match ty {
            NumType::U8 | NumType::I8 => 1,
            NumType::U16 | NumType::I16 => 2,
            NumType::U32 | NumType::I32 | NumType::F32 => 4,
            NumType::U64 | NumType::I64 | NumType::F64 => 8,
            NumType::Usize | NumType::Isize if self.ptr() == FfiType::I32 => 4,
            NumType::Usize | NumType::Isize if self.ptr() == FfiType::I64 => 8,
            _ => unimplemented!(),
        };
        let size = match self {
            Self::Native(_) => size,
            Self::Wasm(_) => core::cmp::max(4, size),
        };
        (size, size)
    }

    /*pub fn lower_type(self, ty: AbiType) -> Vec<FfiType> {
        match ty {
            AbiType::Num(NumType::U8) => vec![FfiType::I8],
            AbiType::Num(NumType::U16) => vec![FfiType::I16],
            AbiType::Num(NumType::U32) => vec![FfiType::I32],
            AbiType::Num(NumType::U64) => vec![FfiType::I64],
            AbiType::Num(NumType::Usize) => vec![self.ptr()],
            AbiType::Num(NumType::I8) => vec![FfiType::I8],
            AbiType::Num(NumType::I16) => vec![FfiType::I16],
            AbiType::Num(NumType::I32) => vec![FfiType::I32],
            AbiType::Num(NumType::I64) => vec![FfiType::I64],
            AbiType::Num(NumType::Isize) => vec![self.ptr()],
            AbiType::Num(NumType::F32) => vec![FfiType::F32],
            AbiType::Num(NumType::F64) => vec![FfiType::F64],
            AbiType::Bool => vec![FfiType::I8],
            AbiType::RefStr => vec![self.ptr(), self.ptr()],
            AbiType::String => vec![self.ptr(), self.ptr(), self.ptr()],
            AbiType::RefSlice(_) => vec![self.ptr(), self.ptr()],
            AbiType::Vec(_) => vec![self.ptr(), self.ptr(), self.ptr()],
            AbiType::RefObject(_)
            | AbiType::Object(_)
            | AbiType::Future(_)
            | AbiType::Stream(_) => vec![self.ptr()],
            AbiType::Option(ty) => {
                let mut v = vec![FfiType::I8];
                v.extend(self.lower(*ty));
                v
            }
            AbiType::Result(ty) => {
                let mut v = vec![FfiType::I8, self.ptr(), self.ptr()];
                v.extend(self.lower(*ty));
                v
            }
        }
    }*/

    fn lower_arg(
        self,
        arg: u32,
        ty: &AbiType,
        ident: &mut u32,
        args: &mut Vec<(u32, FfiType)>,
        instr: &mut Vec<Instr>,
        cleanup: &mut Vec<Instr>,
    ) {
        match ty {
            AbiType::Num(NumType::U8) => {
                let out = *ident;
                *ident += 1;
                instr.push(Instr::CastU8I8(arg, out));
                args.push((out, FfiType::I8));
            }
            AbiType::Num(NumType::U16) => {
                let out = *ident;
                *ident += 1;
                instr.push(Instr::CastU16I16(arg, out));
                args.push((out, FfiType::I16));
            }
            AbiType::Num(NumType::U32) => {
                let out = *ident;
                *ident += 1;
                instr.push(Instr::CastU32I32(arg, out));
                args.push((out, FfiType::I32));
            }
            AbiType::Num(NumType::U64) => {
                let out = *ident;
                *ident += 1;
                instr.push(Instr::CastU64I64(arg, out));
                args.push((out, FfiType::I64));
            }
            AbiType::Num(NumType::Usize) => {
                let out = *ident;
                *ident += 1;
                match self.ptr() {
                    FfiType::I32 => instr.push(Instr::CastU32I32(arg, out)),
                    FfiType::I64 => instr.push(Instr::CastU64I64(arg, out)),
                    _ => unimplemented!(),
                }
                args.push((out, self.ptr()));
            }
            AbiType::Num(NumType::I8) => {
                args.push((arg, FfiType::I8));
            }
            AbiType::Num(NumType::I16) => {
                args.push((arg, FfiType::I16));
            }
            AbiType::Num(NumType::I32) => {
                args.push((arg, FfiType::I32));
            }
            AbiType::Num(NumType::I64) => {
                args.push((arg, FfiType::I64));
            }
            AbiType::Num(NumType::Isize) => {
                args.push((arg, self.ptr()));
            }
            AbiType::Num(NumType::F32) => {
                args.push((arg, FfiType::F32));
            }
            AbiType::Num(NumType::F64) => {
                args.push((arg, FfiType::F64));
            }
            AbiType::Bool => {
                let out = *ident;
                *ident += 1;
                instr.push(Instr::CastBoolI8(arg, out));
                args.push((out, FfiType::I8));
            }
            AbiType::RefStr => {
                let ptr = *ident;
                *ident += 1;
                let len = *ident;
                *ident += 1;
                instr.push(Instr::StrLen(arg, len));
                instr.push(Instr::Allocate(ptr, len, 1, 1));
                instr.push(Instr::LowerString(arg, ptr, len));
                args.push((ptr, self.ptr()));
                args.push((len, self.ptr()));
                cleanup.push(Instr::Deallocate(ptr, len, 1, 1));
            }
            AbiType::String => {
                let ptr = *ident;
                *ident += 1;
                let len = *ident;
                *ident += 1;
                instr.push(Instr::StrLen(arg, len));
                instr.push(Instr::Allocate(ptr, len, 1, 1));
                instr.push(Instr::LowerString(arg, ptr, len));
                args.push((ptr, self.ptr()));
                args.push((len, self.ptr()));
                args.push((len, self.ptr()));
            }
            AbiType::RefSlice(ty) => {
                let ptr = *ident;
                *ident += 1;
                let len = *ident;
                *ident += 1;
                instr.push(Instr::VecLen(arg, len));
                let (size, align) = self.layout(*ty);
                instr.push(Instr::Allocate(ptr, len, size, align));
                instr.push(Instr::LowerVec(arg, ptr, len, size));
                args.push((ptr, self.ptr()));
                args.push((len, self.ptr()));
                cleanup.push(Instr::Deallocate(ptr, len, size, align));
            }
            AbiType::Vec(ty) => {
                let ptr = *ident;
                *ident += 1;
                let len = *ident;
                *ident += 1;
                instr.push(Instr::VecLen(arg, len));
                let (size, align) = self.layout(*ty);
                instr.push(Instr::Allocate(ptr, len, size, align));
                instr.push(Instr::LowerVec(arg, ptr, len, size));
                args.push((ptr, self.ptr()));
                args.push((len, self.ptr()));
                args.push((len, self.ptr()));
            }
            AbiType::RefObject(_) => {
                let ptr = *ident;
                *ident += 1;
                instr.push(Instr::BorrowObject(arg, ptr));
                args.push((ptr, self.ptr()));
            }
            AbiType::Object(_) => {
                let ptr = *ident;
                *ident += 1;
                instr.push(Instr::MoveObject(arg, ptr));
                args.push((ptr, self.ptr()));
            }
            AbiType::Future(_) => {
                let ptr = *ident;
                *ident += 1;
                instr.push(Instr::MoveFuture(arg, ptr));
                args.push((ptr, self.ptr()));
            }
            AbiType::Stream(_) => {
                let ptr = *ident;
                *ident += 1;
                instr.push(Instr::MoveStream(arg, ptr));
                args.push((ptr, self.ptr()));
            }
            AbiType::Option(_ty) => todo!(),
            AbiType::Result(_ty) => todo!(),
        }
    }

    pub fn lower_func(self, func: &AbiFunction) -> FfiFunction {
        let symbol = match &func.ty {
            FunctionType::Constructor(obj) | FunctionType::Method(obj) => {
                format!("__{}_{}", obj, &func.name)
            }
            FunctionType::Function => format!("__{}", &func.name),
        };
        let mut ident = 0;
        let mut instr = vec![];
        let mut cleanup = vec![];
        let mut args = vec![];
        let mut rets = vec![];
        let mut return_ = vec![];
        if let FunctionType::Constructor(_) = &func.ty {
            let self_ = ident;
            ident += 1;
            instr.push(Instr::BorrowSelf(self_));
            args.push((self_, self.ptr()));
        }
        for (name, ty) in func.args.iter() {
            let arg = ident;
            ident += 1;
            instr.push(Instr::BindArg(name.clone(), ident));
            self.lower_arg(arg, ty, &mut ident, &mut args, &mut instr, &mut cleanup);
        }
        let ret = ident;
        ident += 1;
        instr.push(Instr::Call(
            symbol.clone(),
            ret,
            args.iter().map(|(ident, _)| *ident).collect(),
        ));
        match func.ret.as_ref() {
            Some(AbiType::Num(NumType::U8)) => {
                let out = ident;
                ident += 1;
                rets.push(FfiType::I8);
                instr.push(Instr::CastI8U8(ret, out));
                return_.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Num(NumType::U16)) => {
                let out = ident;
                ident += 1;
                rets.push(FfiType::I16);
                instr.push(Instr::CastI16U16(ret, out));
                return_.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Num(NumType::U32)) => {
                let out = ident;
                ident += 1;
                rets.push(FfiType::I32);
                instr.push(Instr::CastI32U32(ret, out));
                return_.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Num(NumType::U64)) => {
                let out = ident;
                ident += 1;
                rets.push(FfiType::I64);
                instr.push(Instr::CastI64U64(ret, out));
                return_.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Num(NumType::Usize)) => {
                let out = ident;
                ident += 1;
                rets.push(self.ptr());
                match self.ptr() {
                    FfiType::I32 => instr.push(Instr::CastI32U32(ret, out)),
                    FfiType::I64 => instr.push(Instr::CastI64U64(ret, out)),
                    _ => unimplemented!(),
                }
                return_.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Num(NumType::I8)) => {
                rets.push(FfiType::I8);
                return_.push(Instr::ReturnValue(ret));
            }
            Some(AbiType::Num(NumType::I16)) => {
                rets.push(FfiType::I16);
                return_.push(Instr::ReturnValue(ret));
            }
            Some(AbiType::Num(NumType::I32)) => {
                rets.push(FfiType::I32);
                return_.push(Instr::ReturnValue(ret));
            }
            Some(AbiType::Num(NumType::I64)) => {
                rets.push(FfiType::I64);
                return_.push(Instr::ReturnValue(ret));
            }
            Some(AbiType::Num(NumType::Isize)) => {
                rets.push(self.ptr());
                return_.push(Instr::ReturnValue(ret));
            }
            Some(AbiType::Num(NumType::F32)) => {
                rets.push(FfiType::F32);
                return_.push(Instr::ReturnValue(ret));
            }
            Some(AbiType::Num(NumType::F64)) => {
                rets.push(FfiType::F64);
                return_.push(Instr::ReturnValue(ret));
            }
            Some(AbiType::Bool) => {
                let out = ident;
                ident += 1;
                rets.push(FfiType::I8);
                instr.push(Instr::CastI8Bool(ret, out));
                return_.push(Instr::ReturnValue(out));
            }
            Some(AbiType::RefStr) => {
                let (ptr, len, out) = (ident, ident + 1, ident + 2);
                ident += 3;
                rets.push(self.ptr());
                rets.push(self.ptr());
                instr.push(Instr::BindRet(0, ptr));
                instr.push(Instr::BindRet(1, len));
                instr.push(Instr::LiftString(ptr, len, out));
                return_.push(Instr::ReturnValue(out));
            }
            Some(AbiType::String) => {
                let (ptr, len, cap, out) = (ident, ident + 1, ident + 2, ident + 3);
                ident += 4;
                rets.push(self.ptr());
                rets.push(self.ptr());
                rets.push(self.ptr());
                instr.push(Instr::BindRet(0, ptr));
                instr.push(Instr::BindRet(1, len));
                instr.push(Instr::BindRet(2, cap));
                instr.push(Instr::LiftString(ptr, len, out));
                instr.push(Instr::Deallocate(ptr, cap, 1, 1));
                return_.push(Instr::ReturnValue(out));
            }
            Some(AbiType::RefSlice(ty)) => {
                let (ptr, len, out) = (ident, ident + 1, ident + 2);
                ident += 3;
                rets.push(self.ptr());
                rets.push(self.ptr());
                instr.push(Instr::BindRet(0, ptr));
                instr.push(Instr::BindRet(1, len));
                let (size, _align) = self.layout(*ty);
                instr.push(Instr::LiftVec(ptr, len, out, size));
                return_.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Vec(ty)) => {
                let (ptr, len, cap, out) = (ident, ident + 1, ident + 2, ident + 3);
                ident += 4;
                rets.push(self.ptr());
                rets.push(self.ptr());
                rets.push(self.ptr());
                instr.push(Instr::BindRet(0, ptr));
                instr.push(Instr::BindRet(1, len));
                instr.push(Instr::BindRet(2, cap));
                let (size, align) = self.layout(*ty);
                instr.push(Instr::LiftVec(ptr, len, out, size));
                instr.push(Instr::Deallocate(ptr, cap, size, align));
                return_.push(Instr::ReturnValue(out));
            }
            Some(AbiType::RefObject(_obj)) => todo!(),
            Some(AbiType::Object(obj)) => {
                let out = ident;
                ident += 1;
                rets.push(self.ptr());
                let destructor = format!("drop_object_{}", obj);
                instr.push(Instr::MakeObject(obj.clone(), ret, destructor, out));
                return_.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Future(_ty)) => todo!(),
            Some(AbiType::Stream(_ty)) => todo!(),
            Some(AbiType::Option(_ty)) => todo!(),
            Some(AbiType::Result(_ty)) => todo!(),
            None => {
                return_.push(Instr::ReturnVoid);
            }
        }
        #[allow(clippy::drop_copy)]
        drop(ident);
        instr.extend(cleanup);
        instr.extend(return_);
        FfiFunction {
            symbol,
            args,
            ret: rets,
            instr,
        }
    }
}

#[derive(Clone, Debug)]
pub enum FunctionType {
    Constructor(String),
    Method(String),
    Function,
}

#[derive(Clone, Debug)]
pub struct FfiFunction {
    pub symbol: String,
    pub args: Vec<(u32, FfiType)>,
    pub ret: Vec<FfiType>,
    pub instr: Vec<Instr>,
}

#[derive(Clone, Debug)]
pub struct AbiObject {
    pub name: String,
    pub methods: Vec<AbiFunction>,
}

#[derive(Clone, Debug)]
pub struct AbiFunction {
    pub ty: FunctionType,
    pub name: String,
    pub args: Vec<(String, AbiType)>,
    pub ret: Option<AbiType>,
}

#[derive(Clone, Debug)]
pub enum Instr {
    BorrowSelf(u32),
    BorrowObject(u32, u32),
    MoveObject(u32, u32),
    MoveFuture(u32, u32),
    MoveStream(u32, u32),
    MakeObject(String, u32, String, u32),
    BindArg(String, u32),
    BindRet(usize, u32),
    CastU8I8(u32, u32),
    CastU16I16(u32, u32),
    CastU32I32(u32, u32),
    CastU64I64(u32, u32),
    CastI8U8(u32, u32),
    CastI16U16(u32, u32),
    CastI32U32(u32, u32),
    CastI64U64(u32, u32),
    CastBoolI8(u32, u32),
    CastI8Bool(u32, u32),
    StrLen(u32, u32),
    VecLen(u32, u32),
    Allocate(u32, u32, usize, usize),
    Deallocate(u32, u32, usize, usize),
    LowerString(u32, u32, u32),
    LiftString(u32, u32, u32),
    LowerVec(u32, u32, u32, usize),
    LiftVec(u32, u32, u32, usize),
    IsSome(u32, u32),
    Call(String, u32, Vec<u32>),
    ReturnValue(u32),
    ReturnVoid,
}

impl Interface {
    pub fn objects(&self) -> Vec<AbiObject> {
        let mut objs = vec![];
        for object in &self.objects {
            let mut methods = vec![];
            for method in &object.methods {
                let obj = object.ident.clone();
                let ty = if method.is_static {
                    FunctionType::Constructor(obj)
                } else {
                    FunctionType::Method(obj)
                };
                let func = AbiFunction {
                    ty,
                    name: method.func.ident.clone(),
                    args: method
                        .func
                        .ty
                        .args
                        .iter()
                        .map(|(n, ty)| (n.clone(), self.to_type(ty)))
                        .collect(),
                    ret: method.func.ty.ret.as_ref().map(|ty| self.to_type(ty)),
                };
                methods.push(func);
            }
            objs.push(AbiObject {
                name: object.ident.clone(),
                methods,
            });
        }
        objs
    }

    pub fn functions(&self) -> Vec<AbiFunction> {
        let mut funcs = vec![];
        for func in &self.functions {
            let name = func.ident.clone();
            let args = func
                .ty
                .args
                .iter()
                .map(|(n, ty)| (n.clone(), self.to_type(ty)))
                .collect();
            let ret = func.ty.ret.as_ref().map(|ty| self.to_type(ty));
            let func = AbiFunction {
                ty: FunctionType::Function,
                name,
                args,
                ret,
            };
            funcs.push(func);
        }
        funcs
    }

    pub fn to_type(&self, ty: &Type) -> AbiType {
        match ty {
            Type::U8 => AbiType::Num(NumType::U8),
            Type::U16 => AbiType::Num(NumType::U16),
            Type::U32 => AbiType::Num(NumType::U32),
            Type::U64 => AbiType::Num(NumType::U64),
            Type::Usize => AbiType::Num(NumType::Usize),
            Type::I8 => AbiType::Num(NumType::I8),
            Type::I16 => AbiType::Num(NumType::I16),
            Type::I32 => AbiType::Num(NumType::I32),
            Type::I64 => AbiType::Num(NumType::I64),
            Type::Isize => AbiType::Num(NumType::Isize),
            Type::F32 => AbiType::Num(NumType::F32),
            Type::F64 => AbiType::Num(NumType::F64),
            Type::Bool => AbiType::Bool,
            Type::Ref(inner) => match &**inner {
                Type::String => AbiType::RefStr,
                Type::Slice(inner) => match self.to_type(inner) {
                    AbiType::Num(ty) => AbiType::RefSlice(ty),
                    ty => unimplemented!("&{:?}", ty),
                },
                Type::Ident(ident) => AbiType::RefObject(ident.clone()),
                ty => unimplemented!("&{:?}", ty),
            },
            Type::String => AbiType::String,
            Type::Vec(inner) => match self.to_type(inner) {
                AbiType::Num(ty) => AbiType::Vec(ty),
                ty => unimplemented!("Vec<{:?}>", ty),
            },
            Type::Ident(ident) if self.is_object(ident) => AbiType::Object(ident.clone()),
            Type::Option(ty) => AbiType::Option(Box::new(self.to_type(ty))),
            Type::Result(ty) => AbiType::Result(Box::new(self.to_type(ty))),
            Type::Future(ty) => AbiType::Future(Box::new(self.to_type(ty))),
            Type::Stream(ty) => AbiType::Stream(Box::new(self.to_type(ty))),
            ty => unimplemented!("{:?}", ty),
        }
    }
}
