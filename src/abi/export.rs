use super::VarGen;
use crate::{Abi, AbiFunction, AbiType, FunctionType, NumType, Return, Var};

#[derive(Clone, Debug)]
pub struct Export {
    pub symbol: String,
    pub args: Vec<Var>,
    pub instr: Vec<Instr>,
    pub ret: Return,
}

impl Abi {
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
        let self_ = if let FunctionType::Method(object) = &func.ty {
            let out = gen.gen(AbiType::RefObject(object.clone()));
            let ptr = gen.gen_num(self.iptr());
            def.push(ptr.clone());
            exports.push(Instr::LiftRefObject(ptr, out.clone(), object.clone()));
            Some(out)
        } else {
            None
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
        let ret = func.ret.as_ref().map(|ret| gen.gen(ret.clone()));
        exports.push(Instr::CallAbi(
            func.ty.clone(),
            self_,
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
        let ret = match rets.len() {
            0 => Return::Void,
            1 => Return::Num(rets[0].clone()),
            _ => Return::Struct(rets, format!("{}Return", symbol)),
        };
        Export {
            symbol,
            instr: exports,
            args: def,
            ret,
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
    CallAbi(FunctionType, Option<Var>, String, Option<Var>, Vec<Var>),
}
