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
    fn export_arg(
        self,
        ty: &AbiType,
        gen: &mut VarGen,
        exports: &mut Vec<Instr>,
        def: &mut Vec<Var>,
        args: &mut Vec<Var>,
    ) {
        let out = gen.gen(ty.clone());
        args.push(out.clone());
        match ty {
            AbiType::Num(num) => {
                let arg = gen.gen_num(*num);
                def.push(arg.clone());
                exports.push(Instr::LiftNum(arg, out));
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
            AbiType::RefFuture(ty) => {
                let ptr = gen.gen_num(self.iptr());
                def.push(ptr.clone());
                exports.push(Instr::LiftRefFuture(ptr, out, (&**ty).clone()));
            }
            AbiType::RefStream(ty) => {
                let ptr = gen.gen_num(self.iptr());
                def.push(ptr.clone());
                exports.push(Instr::LiftRefStream(ptr, out, (&**ty).clone()));
            }
            AbiType::Future(_) => todo!(),
            AbiType::Stream(_) => todo!(),
        }
    }

    fn export_return(
        self,
        ret: Var,
        gen: &mut VarGen,
        exports: &mut Vec<Instr>,
        rets: &mut Vec<Var>,
    ) {
        match &ret.ty {
            AbiType::Num(num) => {
                let out = gen.gen_num(*num);
                rets.push(out.clone());
                exports.push(Instr::LowerNum(ret, out));
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
            AbiType::Option(ty) => {
                let var = gen.gen_num(NumType::U8);
                let some = gen.gen((&**ty).clone());
                rets.push(var.clone());
                let mut some_instr = vec![];
                self.export_return(some.clone(), gen, &mut some_instr, rets);
                exports.push(Instr::LowerOption(ret, var, some, some_instr));
            }
            AbiType::Result(ty) => {
                let var = gen.gen_num(NumType::U8);
                let ok = gen.gen((&**ty).clone());
                let err = gen.gen(AbiType::String);
                rets.push(var.clone());
                let mut err_instr = vec![];
                self.export_return(err.clone(), gen, &mut err_instr, rets);
                let mut ok_instr = vec![];
                self.export_return(ok.clone(), gen, &mut ok_instr, rets);
                exports.push(Instr::LowerResult(ret, var, ok, ok_instr, err, err_instr));
            }
            AbiType::RefFuture(_ty) => todo!(),
            AbiType::Future(ty) => {
                let ptr = gen.gen_num(self.iptr());
                rets.push(ptr.clone());
                let ty = (&**ty).clone();
                exports.push(Instr::LowerFuture(ret, ptr, ty));
            }
            AbiType::RefStream(_ty) => todo!(),
            AbiType::Stream(ty) => {
                let ptr = gen.gen_num(self.iptr());
                rets.push(ptr.clone());
                let ty = (&**ty).clone();
                exports.push(Instr::LowerStream(ret, ptr, ty));
            }
        }
    }

    pub fn export(self, func: &AbiFunction) -> Export {
        let mut gen = VarGen::new();
        let mut def = vec![];
        let mut args = vec![];
        let mut rets = vec![];
        let mut exports = vec![];
        let self_ = match &func.ty {
            FunctionType::Method(object) => {
                let out = gen.gen(AbiType::RefObject(object.clone()));
                let ptr = gen.gen_num(self.iptr());
                def.push(ptr.clone());
                exports.push(Instr::LiftRefObject(ptr, out.clone(), object.clone()));
                Some(out)
            }
            FunctionType::PollFuture(_, ty) => {
                let out = gen.gen(AbiType::RefFuture(Box::new(ty.clone())));
                let ptr = gen.gen_num(self.iptr());
                def.push(ptr.clone());
                exports.push(Instr::LiftRefFuture(ptr, out.clone(), ty.clone()));
                Some(out)
            }
            FunctionType::PollStream(_, ty) => {
                let out = gen.gen(AbiType::RefStream(Box::new(ty.clone())));
                let ptr = gen.gen_num(self.iptr());
                def.push(ptr.clone());
                exports.push(Instr::LiftRefStream(ptr, out.clone(), ty.clone()));
                Some(out)
            }
            _ => None,
        };
        for (_, ty) in func.args.iter() {
            self.export_arg(ty, &mut gen, &mut exports, &mut def, &mut args);
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
            let mut instr = vec![];
            self.export_return(ret, &mut gen, &mut instr, &mut rets);
            if !rets.is_empty() {
                exports.push(Instr::DefineRets(rets.clone()));
            }
            exports.extend(instr);
        }
        Export {
            symbol: func.symbol(),
            instr: exports,
            args: def,
            ret: func.ret(rets),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Instr {
    LiftNum(Var, Var),
    LowerNum(Var, Var),
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
    LowerOption(Var, Var, Var, Vec<Instr>),
    LowerResult(Var, Var, Var, Vec<Instr>, Var, Vec<Instr>),
    LowerFuture(Var, Var, AbiType),
    LiftRefFuture(Var, Var, AbiType),
    LowerStream(Var, Var, AbiType),
    LiftRefStream(Var, Var, AbiType),
    CallAbi(FunctionType, Option<Var>, String, Option<Var>, Vec<Var>),
    DefineRets(Vec<Var>),
}
