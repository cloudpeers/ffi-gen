use crate::parser::{Interface, Type};

pub mod export;
pub mod import;

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
pub struct AbiStruct {
    pub name: String,
    pub fields: Vec<(String, AbiType)>,
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

    pub fn uptr(self) -> NumType {
        match self {
            Self::Native32 | Self::Wasm32 => NumType::U32,
            Self::Native64 | Self::Wasm64 => NumType::U64,
        }
    }

    pub fn iptr(self) -> NumType {
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

    pub fn exports(&self, abi: &Abi) -> Vec<export::Export> {
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

    pub fn imports(&self, abi: &Abi) -> Vec<import::Import> {
        let mut imports = vec![];
        for function in self.functions() {
            imports.push(abi.import(&function));
        }
        for obj in self.objects() {
            for method in &obj.methods {
                imports.push(abi.import(method));
            }
        }
        imports
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
