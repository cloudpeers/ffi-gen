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
    RefFuture(Box<AbiType>),
    Future(Box<AbiType>),
    RefStream(Box<AbiType>),
    Stream(Box<AbiType>),
}

impl AbiType {
    pub fn num(&self) -> NumType {
        if let Self::Num(num) = self {
            *num
        } else {
            panic!()
        }
    }
}

#[derive(Clone, Debug)]
pub enum FunctionType {
    Constructor(String),
    Method(String),
    Function,
    PollFuture(String, AbiType),
    PollStream(String, AbiType),
}

#[derive(Clone, Debug)]
pub struct AbiFunction {
    pub doc: Vec<String>,
    pub ty: FunctionType,
    pub name: String,
    pub args: Vec<(String, AbiType)>,
    pub ret: Option<AbiType>,
}

impl AbiFunction {
    pub fn symbol(&self) -> String {
        match &self.ty {
            FunctionType::Constructor(object) | FunctionType::Method(object) => {
                format!("__{}_{}", object, &self.name)
            }
            FunctionType::Function => format!("__{}", &self.name),
            FunctionType::PollFuture(symbol, _) => format!("{}_future_{}", symbol, &self.name),
            FunctionType::PollStream(symbol, _) => format!("{}_stream_{}", symbol, &self.name),
        }
    }

    pub fn ret(&self, rets: Vec<Var>) -> Return {
        match rets.len() {
            0 => Return::Void,
            1 => Return::Num(rets[0].clone()),
            _ => Return::Struct(rets, format!("{}Return", self.symbol())),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AbiObject {
    pub doc: Vec<String>,
    pub name: String,
    pub methods: Vec<AbiFunction>,
    pub destructor: String,
}

#[derive(Clone, Debug)]
pub struct AbiFuture {
    pub ty: AbiType,
    pub symbol: String,
}

impl AbiFuture {
    pub fn poll(&self) -> AbiFunction {
        AbiFunction {
            ty: FunctionType::PollFuture(self.symbol.clone(), self.ty.clone()),
            doc: vec![],
            name: "poll".to_string(),
            args: vec![
                ("post_cobject".to_string(), AbiType::Isize),
                ("port".to_string(), AbiType::Num(NumType::I64)),
            ],
            ret: Some(AbiType::Option(Box::new(self.ty.clone()))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AbiStream {
    pub ty: AbiType,
    pub symbol: String,
}

impl AbiStream {
    pub fn poll(&self) -> AbiFunction {
        AbiFunction {
            ty: FunctionType::PollStream(self.symbol.clone(), self.ty.clone()),
            doc: vec![],
            name: "poll".to_string(),
            args: vec![
                ("post_cobject".to_string(), AbiType::Isize),
                ("port".to_string(), AbiType::Num(NumType::I64)),
                ("done".to_string(), AbiType::Num(NumType::I64)),
            ],
            ret: Some(AbiType::Option(Box::new(self.ty.clone()))),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Return {
    Void,
    Num(Var),
    Struct(Vec<Var>, String),
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
                let func = AbiFunction {
                    doc: method.doc.clone(),
                    name: method.ident.clone(),
                    ty: if method.is_static {
                        FunctionType::Constructor(obj)
                    } else {
                        FunctionType::Method(obj)
                    },
                    args: method
                        .args
                        .iter()
                        .map(|(n, ty)| (n.clone(), self.to_type(ty)))
                        .collect(),
                    ret: method.ret.as_ref().map(|ty| self.to_type(ty)),
                };
                methods.push(func);
            }
            objs.push(AbiObject {
                doc: object.doc.clone(),
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
            assert!(!func.is_static);
            let args = func
                .args
                .iter()
                .map(|(n, ty)| (n.clone(), self.to_type(ty)))
                .collect();
            let ret = func.ret.as_ref().map(|ty| self.to_type(ty));
            let func = AbiFunction {
                doc: func.doc.clone(),
                name: func.ident.clone(),
                ty: FunctionType::Function,
                args,
                ret,
            };
            funcs.push(func);
        }
        funcs
    }

    pub fn futures(&self) -> Vec<AbiFuture> {
        let mut futures = vec![];
        let mut functions = self.functions();
        for obj in self.objects() {
            functions.extend(obj.methods);
        }
        for func in functions {
            if let Some(ty) = func.ret.as_ref() {
                let mut p = ty;
                loop {
                    match p {
                        AbiType::Option(ty) | AbiType::Result(ty) => p = &**ty,
                        AbiType::Future(ty) => {
                            let symbol = func.symbol();
                            futures.push(AbiFuture {
                                ty: (&**ty).clone(),
                                symbol,
                            });
                            break;
                        }
                        _ => break,
                    }
                }
            }
        }
        futures
    }

    pub fn streams(&self) -> Vec<AbiStream> {
        let mut streams = vec![];
        let mut functions = self.functions();
        for obj in self.objects() {
            functions.extend(obj.methods);
        }
        for func in functions {
            if let Some(ty) = func.ret.as_ref() {
                let mut p = ty;
                loop {
                    match p {
                        AbiType::Option(ty) | AbiType::Result(ty) => p = &**ty,
                        AbiType::Stream(ty) => {
                            let symbol = func.symbol();
                            streams.push(AbiStream {
                                ty: (&**ty).clone(),
                                symbol,
                            });
                            break;
                        }
                        _ => break,
                    }
                }
            }
        }
        streams
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
        for fut in self.futures() {
            imports.push(abi.import(&fut.poll()));
        }
        for stream in self.streams() {
            imports.push(abi.import(&stream.poll()));
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
