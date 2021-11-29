use crate::parser::{Interface, Type};

#[derive(Clone, Copy, Debug)]
pub enum NumType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

#[derive(Clone, Debug)]
pub enum AbiType {
    Num(NumType),
    Usize,
    Isize,
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

#[derive(Clone, Debug)]
pub enum FunctionType {
    Constructor(String),
    Method(String),
    Function,
}

#[derive(Clone, Debug)]
pub struct AbiFunction {
    pub ty: FunctionType,
    pub name: String,
    pub args: Vec<(String, AbiType)>,
    pub ret: Option<AbiType>,
}

#[derive(Clone, Debug)]
pub struct AbiObject {
    pub name: String,
    pub methods: Vec<AbiFunction>,
    pub destructor: String,
}

#[derive(Clone, Debug)]
pub struct Export {
    pub symbol: String,
    pub args: Vec<Var>,
    pub rets: Vec<Var>,
    pub instr: Vec<Instr>,
}

#[derive(Default)]
struct VarGen {
    counter: u32,
}

impl VarGen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gen_num(&mut self, num: NumType) -> Var {
        self.gen(AbiType::Num(num))
    }

    pub fn gen(&mut self, ty: AbiType) -> Var {
        let binding = self.counter;
        self.counter += 1;
        Var { binding, ty }
    }
}

#[derive(Clone, Debug)]
pub struct Var {
    pub binding: u32,
    pub ty: AbiType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Abi {
    Native32,
    Native64,
    Wasm32,
    Wasm64,
}

impl Abi {
    pub fn native() -> Self {
        #[cfg(target_pointer_width = "32")]
        return Abi::Native32;
        #[cfg(target_pointer_width = "64")]
        return Abi::Native64;
    }

    fn uptr(self) -> NumType {
        match self {
            Self::Native32 | Self::Wasm32 => NumType::U32,
            Self::Native64 | Self::Wasm64 => NumType::U64,
        }
    }

    fn iptr(self) -> NumType {
        match self {
            Self::Native32 | Self::Wasm32 => NumType::I32,
            Self::Native64 | Self::Wasm64 => NumType::I64,
        }
    }

    /// Returns the size and alignment of a primitive type.
    pub fn layout(self, ty: NumType) -> (usize, usize) {
        let size = match ty {
            NumType::U8 | NumType::I8 => 1,
            NumType::U16 | NumType::I16 => 2,
            NumType::U32 | NumType::I32 | NumType::F32 => 4,
            NumType::U64 | NumType::I64 | NumType::F64 => 8,
        };
        let size = match self {
            Self::Native32 | Self::Native64 => size,
            Self::Wasm32 | Self::Wasm64 => core::cmp::max(4, size),
        };
        (size, size)
    }

    pub fn export(self, func: &AbiFunction) -> Export {
        let mut gen = VarGen::new();
        let mut def = vec![];
        let mut args = vec![];
        let mut rets = vec![];
        let mut exports = vec![];
        let symbol = match &func.ty {
            FunctionType::Constructor(object) | FunctionType::Method(object) => {
                format!("__{}_{}", object, &func.name)
            }
            FunctionType::Function => format!("__{}", &func.name),
        };
        for (_, ty) in func.args.iter() {
            let out = gen.gen(ty.clone());
            args.push(out.clone());
            match ty {
                AbiType::Num(_num) => {
                    def.push(out);
                }
                AbiType::Isize => {
                    let arg = gen.gen_num(self.iptr());
                    def.push(arg.clone());
                    exports.push(Instr::LiftIsize(arg, out));
                }
                AbiType::Usize => {
                    let arg = gen.gen_num(self.uptr());
                    def.push(arg.clone());
                    exports.push(Instr::LiftUsize(arg, out));
                }
                AbiType::Bool => {
                    let arg = gen.gen_num(NumType::U8);
                    def.push(arg.clone());
                    exports.push(Instr::LiftBool(arg, out));
                }
                AbiType::RefStr => {
                    let ptr = gen.gen_num(self.iptr());
                    let len = gen.gen_num(self.uptr());
                    def.extend_from_slice(&[ptr.clone(), len.clone()]);
                    exports.push(Instr::LiftStr(ptr, len, out));
                }
                AbiType::String => {
                    let ptr = gen.gen_num(self.iptr());
                    let len = gen.gen_num(self.uptr());
                    let cap = gen.gen_num(self.uptr());
                    def.extend_from_slice(&[ptr.clone(), len.clone(), cap.clone()]);
                    exports.push(Instr::LiftString(ptr, len, cap, out));
                }
                AbiType::RefSlice(ty) => {
                    let ptr = gen.gen_num(self.iptr());
                    let len = gen.gen_num(self.uptr());
                    def.extend_from_slice(&[ptr.clone(), len.clone()]);
                    exports.push(Instr::LiftSlice(ptr, len, out, *ty));
                }
                AbiType::Vec(ty) => {
                    let ptr = gen.gen_num(self.iptr());
                    let len = gen.gen_num(self.uptr());
                    let cap = gen.gen_num(self.uptr());
                    def.extend_from_slice(&[ptr.clone(), len.clone(), cap.clone()]);
                    exports.push(Instr::LiftVec(ptr, len, cap, out, *ty));
                }
                AbiType::RefObject(object) => {
                    let ptr = gen.gen_num(self.iptr());
                    def.push(ptr.clone());
                    exports.push(Instr::LiftRefObject(ptr, out, object.clone()));
                }
                AbiType::Object(object) => {
                    let ptr = gen.gen_num(self.iptr());
                    def.push(ptr.clone());
                    exports.push(Instr::LiftObject(ptr, out, object.clone()));
                }
                AbiType::Option(_) => todo!(),
                AbiType::Result(_) => todo!(),
                AbiType::Future(_) => todo!(),
                AbiType::Stream(_) => todo!(),
            }
        }
        let ret = if let Some(ret) = func.ret.as_ref() {
            Some(gen.gen(ret.clone()))
        } else {
            None
        };
        exports.push(Instr::CallAbi(
            func.ty.clone(),
            func.name.clone(),
            ret.clone(),
            args,
        ));
        if let Some(ret) = ret {
            match &ret.ty {
                AbiType::Num(_num) => {
                    rets.push(ret);
                }
                AbiType::Isize => {
                    let out = gen.gen_num(self.iptr());
                    rets.push(out.clone());
                    exports.push(Instr::LowerIsize(ret, out));
                }
                AbiType::Usize => {
                    let out = gen.gen_num(self.uptr());
                    rets.push(out.clone());
                    exports.push(Instr::LowerUsize(ret, out));
                }
                AbiType::Bool => {
                    let out = gen.gen_num(NumType::U8);
                    rets.push(out.clone());
                    exports.push(Instr::LowerBool(ret, out));
                }
                AbiType::RefStr => {
                    let ptr = gen.gen_num(self.iptr());
                    let len = gen.gen_num(self.uptr());
                    rets.extend_from_slice(&[ptr.clone(), len.clone()]);
                    exports.push(Instr::LowerStr(ret, ptr, len));
                }
                AbiType::String => {
                    let ptr = gen.gen_num(self.iptr());
                    let len = gen.gen_num(self.uptr());
                    let cap = gen.gen_num(self.uptr());
                    rets.extend_from_slice(&[ptr.clone(), len.clone(), cap.clone()]);
                    exports.push(Instr::LowerString(ret, ptr, len, cap));
                }
                AbiType::RefSlice(ty) => {
                    let ptr = gen.gen_num(self.iptr());
                    let len = gen.gen_num(self.uptr());
                    let ty = *ty;
                    rets.extend_from_slice(&[ptr.clone(), len.clone()]);
                    exports.push(Instr::LowerSlice(ret, ptr, len, ty));
                }
                AbiType::Vec(ty) => {
                    let ptr = gen.gen_num(self.iptr());
                    let len = gen.gen_num(self.uptr());
                    let cap = gen.gen_num(self.uptr());
                    let ty = *ty;
                    rets.extend_from_slice(&[ptr.clone(), len.clone(), cap.clone()]);
                    exports.push(Instr::LowerVec(ret, ptr, len, cap, ty));
                }
                AbiType::RefObject(_object) => {
                    let ptr = gen.gen_num(self.iptr());
                    rets.push(ptr.clone());
                    exports.push(Instr::LowerRefObject(ret, ptr));
                }
                AbiType::Object(_object) => {
                    let ptr = gen.gen_num(self.iptr());
                    rets.push(ptr.clone());
                    exports.push(Instr::LowerObject(ret, ptr));
                }
                AbiType::Option(_) => todo!(),
                AbiType::Result(_) => todo!(),
                AbiType::Future(_) => todo!(),
                AbiType::Stream(_) => todo!(),
            }
        }
        Export {
            symbol,
            instr: exports,
            args: def,
            rets,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Instr {
    LiftIsize(Var, Var),
    LowerIsize(Var, Var),
    LiftUsize(Var, Var),
    LowerUsize(Var, Var),
    LiftBool(Var, Var),
    LowerBool(Var, Var),
    LiftStr(Var, Var, Var),
    LowerStr(Var, Var, Var),
    LiftString(Var, Var, Var, Var),
    LowerString(Var, Var, Var, Var),
    LiftSlice(Var, Var, Var, NumType),
    LowerSlice(Var, Var, Var, NumType),
    LiftVec(Var, Var, Var, Var, NumType),
    LowerVec(Var, Var, Var, Var, NumType),
    LiftRefObject(Var, Var, String),
    LowerRefObject(Var, Var),
    LiftObject(Var, Var, String),
    LowerObject(Var, Var),
    CallAbi(FunctionType, String, Option<Var>, Vec<Var>),
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
                destructor: format!("drop_box_{}", &object.ident),
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

    pub fn exports(&self, abi: &Abi) -> Vec<Export> {
        let mut exports = vec![];
        for function in self.functions() {
            exports.push(abi.export(&function));
        }
        for obj in self.objects() {
            for method in &obj.methods {
                exports.push(abi.export(method));
            }
        }
        exports
    }

    pub fn to_type(&self, ty: &Type) -> AbiType {
        match ty {
            Type::U8 => AbiType::Num(NumType::U8),
            Type::U16 => AbiType::Num(NumType::U16),
            Type::U32 => AbiType::Num(NumType::U32),
            Type::U64 => AbiType::Num(NumType::U64),
            Type::Usize => AbiType::Usize,
            Type::I8 => AbiType::Num(NumType::I8),
            Type::I16 => AbiType::Num(NumType::I16),
            Type::I32 => AbiType::Num(NumType::I32),
            Type::I64 => AbiType::Num(NumType::I64),
            Type::Isize => AbiType::Isize,
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
