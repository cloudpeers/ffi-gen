use anyhow::Result;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct GrammarParser;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Interface {
    pub functions: Vec<Function>,
    pub objects: Vec<Object>,
}

impl Interface {
    pub fn parse(input: &str) -> Result<Self> {
        let root = GrammarParser::parse(Rule::root, input)?;
        let mut functions = vec![];
        let mut objects = vec![];
        for pair in root {
            for pair in pair.into_inner() {
                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::object => {
                            let obj = Object::parse(pair)?;
                            objects.push(obj);
                        }
                        Rule::function => {
                            let fun = Function::parse(pair)?;
                            functions.push(fun);
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(Self { functions, objects })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Object {
    pub ident: String,
    pub methods: Vec<Method>,
}

impl Object {
    pub fn parse(pair: Pair<Rule>) -> Result<Self> {
        let mut ident = None;
        let mut methods = vec![];
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::ident => {
                    ident = Some(pair.as_str().to_string());
                }
                Rule::method => {
                    let method = Method::parse(pair)?;
                    methods.push(method);
                }
                _ => {}
            }
        }
        Ok(Self {
            ident: ident.unwrap(),
            methods,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Method {
    pub is_static: bool,
    pub func: Function,
}

impl Method {
    pub fn parse(pair: Pair<Rule>) -> Result<Self> {
        let mut is_static = false;
        let mut func = None;
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::static_ => {
                    is_static = true;
                }
                Rule::function => {
                    func = Some(Function::parse(pair)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            is_static,
            func: func.unwrap(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    pub ident: String,
    pub ty: FunctionType,
}

impl Function {
    pub fn parse(pair: Pair<Rule>) -> Result<Self> {
        let mut ident = None;
        let mut ty = None;
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::ident => {
                    ident = Some(pair.as_str().to_string());
                }
                Rule::function_type => {
                    ty = Some(FunctionType::parse(pair)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            ident: ident.unwrap(),
            ty: ty.unwrap(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionType {
    pub is_async: bool,
    pub args: Vec<(String, Type)>,
    pub ret: Option<Type>,
}

impl FunctionType {
    pub fn parse(pair: Pair<Rule>) -> Result<Self> {
        let mut is_async = false;
        let mut args = vec![];
        let mut ret = None;
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::async_ => {
                    is_async = true;
                }
                Rule::args => {
                    for pair in pair.into_inner() {
                        match pair.as_rule() {
                            Rule::arg => {
                                let mut ident = None;
                                let mut ty = None;
                                for pair in pair.into_inner() {
                                    match pair.as_rule() {
                                        Rule::ident => {
                                            ident = Some(pair.as_str().to_string());
                                        }
                                        Rule::type_ => {
                                            ty = Some(Type::parse(pair)?);
                                        }
                                        _ => {}
                                    }
                                }
                                args.push((ident.unwrap(), ty.unwrap()));
                            }
                            _ => {}
                        }
                    }
                }
                Rule::type_ => {
                    ret = Some(Type::parse(pair)?);
                }
                _ => {}
            }
        }
        Ok(Self {
            is_async,
            args,
            ret,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
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
    Bool,
    F32,
    F64,
    String,
    Ident(String),
    Box(Box<Type>),
    Slice(Box<Type>),
    Vec(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>),
    Ref(Box<Type>),
    Mut(Box<Type>),
    Fn(Box<FunctionType>),
}

impl Type {
    pub fn parse(pair: Pair<Rule>) -> Result<Self> {
        let pair = pair.into_inner().next().unwrap();
        Ok(match pair.as_rule() {
            Rule::primitive => match pair.as_str() {
                "u8" => Type::U8,
                "u16" => Type::U16,
                "u32" => Type::U32,
                "u64" => Type::U64,
                "usize" => Type::Usize,
                "i8" => Type::I8,
                "i16" => Type::I16,
                "i32" => Type::I32,
                "i64" => Type::I64,
                "isize" => Type::Isize,
                "bool" => Type::Bool,
                "f32" => Type::F32,
                "f64" => Type::F64,
                "string" => Type::String,
                _ => unreachable!(),
            },
            Rule::ident => Type::Ident(pair.as_str().to_string()),
            Rule::box_
            | Rule::slice
            | Rule::vec
            | Rule::opt
            | Rule::res
            | Rule::ref_
            | Rule::mut_ => {
                let first = pair.as_str().chars().nth(0).unwrap();
                let second = pair.as_str().chars().nth(1).unwrap();
                let mut inner = None;
                for pair in pair.into_inner() {
                    if pair.as_rule() == Rule::type_ {
                        inner = Some(Box::new(Type::parse(pair)?));
                    }
                }
                let inner = inner.unwrap();
                match first {
                    'B' => Type::Box(inner),
                    '[' => Type::Slice(inner),
                    'V' => Type::Vec(inner),
                    'O' => Type::Option(inner),
                    'R' => Type::Result(inner),
                    '&' => {
                        if second == 'm' {
                            Type::Mut(inner)
                        } else {
                            Type::Ref(inner)
                        }
                    }
                    _ => unreachable!(),
                }
            }
            Rule::function_type => Type::Fn(Box::new(FunctionType::parse(pair)?)),
            r => unreachable!("{:?}", r),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() -> Result<()> {
        let res = Interface::parse("")?;
        assert_eq!(
            res,
            Interface {
                objects: vec![],
                functions: vec![]
            }
        );
        let res = Interface::parse("hello fn();")?;
        assert_eq!(
            res,
            Interface {
                objects: vec![],
                functions: vec![Function {
                    ident: "hello".to_string(),
                    ty: FunctionType {
                        is_async: false,
                        args: vec![],
                        ret: None,
                    }
                }]
            }
        );
        let res = Interface::parse("hello fn(a: u8);")?;
        assert_eq!(
            res,
            Interface {
                objects: vec![],
                functions: vec![Function {
                    ident: "hello".to_string(),
                    ty: FunctionType {
                        is_async: false,
                        args: vec![("a".to_string(), Type::U8)],
                        ret: None,
                    }
                }]
            }
        );
        let res = Interface::parse("hello fn() -> u8;")?;
        assert_eq!(
            res,
            Interface {
                objects: vec![],
                functions: vec![Function {
                    ident: "hello".to_string(),
                    ty: FunctionType {
                        is_async: false,
                        args: vec![],
                        ret: Some(Type::U8),
                    }
                }]
            }
        );
        let res = Interface::parse("hello fn(a: &string);")?;
        assert_eq!(
            res,
            Interface {
                objects: vec![],
                functions: vec![Function {
                    ident: "hello".to_string(),
                    ty: FunctionType {
                        is_async: false,
                        args: vec![("a".to_string(), Type::Ref(Box::new(Type::String)))],
                        ret: None,
                    }
                }]
            }
        );
        let res = Interface::parse("hello fn(a: &[u8]) -> Vec<i64>;")?;
        assert_eq!(
            res,
            Interface {
                objects: vec![],
                functions: vec![Function {
                    ident: "hello".to_string(),
                    ty: FunctionType {
                        is_async: false,
                        args: vec![(
                            "a".to_string(),
                            Type::Ref(Box::new(Type::Slice(Box::new(Type::U8))))
                        )],
                        ret: Some(Type::Vec(Box::new(Type::I64))),
                    }
                }]
            }
        );
        let res = Interface::parse("hello async fn();")?;
        assert_eq!(
            res,
            Interface {
                objects: vec![],
                functions: vec![Function {
                    ident: "hello".to_string(),
                    ty: FunctionType {
                        is_async: true,
                        args: vec![],
                        ret: None,
                    }
                }]
            }
        );
        let res = Interface::parse(
            "object Greeter { static new fn() -> Box<Greeter>; greet fn() -> string; }",
        )?;
        assert_eq!(
            res,
            Interface {
                functions: vec![],
                objects: vec![Object {
                    ident: "Greeter".to_string(),
                    methods: vec![
                        Method {
                            is_static: true,
                            func: Function {
                                ident: "new".to_string(),
                                ty: FunctionType {
                                    is_async: false,
                                    args: vec![],
                                    ret: Some(Type::Box(Box::new(Type::Ident(
                                        "Greeter".to_string()
                                    )))),
                                }
                            }
                        },
                        Method {
                            is_static: false,
                            func: Function {
                                ident: "greet".to_string(),
                                ty: FunctionType {
                                    is_async: false,
                                    args: vec![],
                                    ret: Some(Type::String),
                                }
                            }
                        }
                    ]
                }]
            }
        );
        Ok(())
    }
}
