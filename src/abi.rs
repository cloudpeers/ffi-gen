use crate::{Interface, Type};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Abi {
    Native,
    Wasm32,
}

pub struct AbiFunction {
    pub is_static: bool,
    pub is_async: bool,
    pub object: Option<String>,
    pub name: String,
    pub args: Vec<(String, Type)>,
    pub ret: Option<Type>,
}

impl AbiFunction {
    pub fn exported_name(&self) -> String {
        if let Some(object) = self.object.as_ref() {
            format!("__{}_{}", object, &self.name)
        } else {
            format!("__{}", &self.name)
        }
    }
}

impl Interface {
    pub fn into_functions(self) -> Vec<AbiFunction> {
        let mut funcs = vec![];
        for object in self.objects {
            for method in object.methods {
                let func = AbiFunction {
                    is_static: method.is_static,
                    is_async: method.func.ty.is_async,
                    object: Some(object.ident.clone()),
                    name: method.func.ident,
                    args: method.func.ty.args,
                    ret: method.func.ty.ret,
                };
                funcs.push(func);
            }
        }
        for func in self.functions {
            let func = AbiFunction {
                is_static: false,
                is_async: func.ty.is_async,
                object: None,
                name: func.ident,
                args: func.ty.args,
                ret: func.ty.ret,
            };
            funcs.push(func);
        }
        funcs
    }
}
