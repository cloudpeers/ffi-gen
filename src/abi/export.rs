use super::VarGen;
use crate::{Abi, AbiFunction, AbiType, FunctionType, NumType, Return, Var};

#[derive(Clone, Debug)]
pub struct Export {
    pub symbol: String,
    pub ffi_args: Vec<Var>,
    pub instr: Vec<Instr>,
    pub ffi_ret: Return,
}

impl Abi {
    fn export_arg(
        self,
        out: Var,
        gen: &mut VarGen,
        instr: &mut Vec<Instr>,
        ffi_args: &mut Vec<Var>,
    ) {
        match &out.ty {
            AbiType::Num(num) => {
                let arg = gen.gen_num(*num);
                ffi_args.push(arg.clone());
                instr.push(Instr::LiftNum(arg, out));
            }
            AbiType::Isize => {
                let arg = gen.gen_num(self.iptr());
                ffi_args.push(arg.clone());
                instr.push(Instr::LiftIsize(arg, out));
            }
            AbiType::Usize => {
                let arg = gen.gen_num(self.uptr());
                ffi_args.push(arg.clone());
                instr.push(Instr::LiftUsize(arg, out));
            }
            AbiType::Bool => {
                let arg = gen.gen_num(NumType::U8);
                ffi_args.push(arg.clone());
                instr.push(Instr::LiftBool(arg, out));
            }
            AbiType::RefStr => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                ffi_args.extend_from_slice(&[ptr.clone(), len.clone()]);
                instr.push(Instr::LiftStr(ptr, len, out));
            }
            AbiType::String => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                ffi_args.extend_from_slice(&[ptr.clone(), len.clone(), cap.clone()]);
                instr.push(Instr::LiftString(ptr, len, cap, out));
            }
            AbiType::RefSlice(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                ffi_args.extend_from_slice(&[ptr.clone(), len.clone()]);
                let ty = *ty;
                instr.push(Instr::LiftSlice(ptr, len, out, ty));
            }
            AbiType::Vec(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                ffi_args.extend_from_slice(&[ptr.clone(), len.clone(), cap.clone()]);
                let ty = *ty;
                instr.push(Instr::LiftVec(ptr, len, cap, out, ty));
            }
            AbiType::RefObject(object) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                let object = object.clone();
                instr.push(Instr::LiftRefObject(ptr, out, object));
            }
            AbiType::Object(object) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                let object = object.clone();
                instr.push(Instr::LiftObject(ptr, out, object));
            }
            AbiType::Option(ty) => {
                let opt = gen.gen_num(NumType::U8);
                ffi_args.push(opt.clone());
                let some = gen.gen((&**ty).clone());
                let mut some_instr = vec![];
                self.export_arg(some.clone(), gen, &mut some_instr, ffi_args);
                instr.push(Instr::LiftOption(opt, out, some, some_instr));
            }
            AbiType::Result(_) => todo!(),
            AbiType::RefIter(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LiftRefIter(ptr, out, ty));
            }
            AbiType::Iter(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LiftIter(ptr, out, ty));
            }
            AbiType::RefFuture(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LiftRefFuture(ptr, out, ty));
            }
            AbiType::Future(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LiftFuture(ptr, out, ty));
            }
            AbiType::RefStream(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LiftRefStream(ptr, out, ty));
            }
            AbiType::Stream(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LiftStream(ptr, out, ty));
            }
            AbiType::Tuple(tys) => {
                let mut vars = vec![];
                for ty in tys {
                    let arg = gen.gen(ty.clone());
                    vars.push(arg.clone());
                    self.export_arg(arg, gen, instr, ffi_args);
                }
                instr.push(Instr::LiftTuple(vars, out));
            }
        }
    }

    fn export_return(
        self,
        ret: Var,
        gen: &mut VarGen,
        instr: &mut Vec<Instr>,
        ffi_rets: &mut Vec<Var>,
    ) {
        match &ret.ty {
            AbiType::Num(num)
                if matches!((self, num), (Abi::Wasm32, NumType::U64 | NumType::I64)) =>
            {
                let low = gen.gen_num(self.uptr());
                let high = gen.gen_num(self.uptr());
                ffi_rets.extend_from_slice(&[low.clone(), high.clone()]);
                let num_type = *num;
                instr.push(Instr::LowerNumAsU32Tuple(ret, low, high, num_type));
            }
            AbiType::Num(num) => {
                let out = gen.gen_num(*num);
                ffi_rets.push(out.clone());
                instr.push(Instr::LowerNum(ret, out));
            }
            AbiType::Isize => {
                let out = gen.gen_num(self.iptr());
                ffi_rets.push(out.clone());
                instr.push(Instr::LowerIsize(ret, out));
            }
            AbiType::Usize => {
                let out = gen.gen_num(self.uptr());
                ffi_rets.push(out.clone());
                instr.push(Instr::LowerUsize(ret, out));
            }
            AbiType::Bool => {
                let out = gen.gen_num(NumType::U8);
                ffi_rets.push(out.clone());
                instr.push(Instr::LowerBool(ret, out));
            }
            AbiType::RefStr => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                ffi_rets.extend_from_slice(&[ptr.clone(), len.clone()]);
                instr.push(Instr::LowerStr(ret, ptr, len));
            }
            AbiType::String => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                ffi_rets.extend_from_slice(&[ptr.clone(), len.clone(), cap.clone()]);
                instr.push(Instr::LowerString(ret, ptr, len, cap));
            }
            AbiType::RefSlice(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let ty = *ty;
                ffi_rets.extend_from_slice(&[ptr.clone(), len.clone()]);
                instr.push(Instr::LowerSlice(ret, ptr, len, ty));
            }
            AbiType::Vec(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                let ty = *ty;
                ffi_rets.extend_from_slice(&[ptr.clone(), len.clone(), cap.clone()]);
                instr.push(Instr::LowerVec(ret, ptr, len, cap, ty));
            }
            AbiType::RefObject(_object) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                instr.push(Instr::LowerRefObject(ret, ptr));
            }
            AbiType::Object(_object) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                instr.push(Instr::LowerObject(ret, ptr));
            }
            AbiType::Option(ty) => {
                let var = gen.gen_num(NumType::U8);
                let some = gen.gen((&**ty).clone());
                ffi_rets.push(var.clone());
                let mut some_instr = vec![];
                self.export_return(some.clone(), gen, &mut some_instr, ffi_rets);
                instr.push(Instr::LowerOption(ret, var, some, some_instr));
            }
            AbiType::Result(ty) => {
                let var = gen.gen_num(NumType::U8);
                let ok = gen.gen((&**ty).clone());
                let err = gen.gen(AbiType::String);
                ffi_rets.push(var.clone());
                let mut err_instr = vec![];
                self.export_return(err.clone(), gen, &mut err_instr, ffi_rets);
                let mut ok_instr = vec![];
                self.export_return(ok.clone(), gen, &mut ok_instr, ffi_rets);
                instr.push(Instr::LowerResult(ret, var, ok, ok_instr, err, err_instr));
            }
            AbiType::RefIter(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LowerRefIter(ret, ptr, ty));
            }
            AbiType::Iter(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LowerIter(ret, ptr, ty));
            }
            AbiType::RefFuture(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LowerRefFuture(ret, ptr, ty));
            }
            AbiType::Future(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LowerFuture(ret, ptr, ty));
            }
            AbiType::RefStream(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LowerRefStream(ret, ptr, ty));
            }
            AbiType::Stream(ty) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                let ty = (&**ty).clone();
                instr.push(Instr::LowerStream(ret, ptr, ty));
            }
            AbiType::Tuple(tys) => {
                let mut vars = vec![];
                let mut instr_inner = vec![];
                for ty in tys {
                    let ret = gen.gen(ty.clone());
                    vars.push(ret.clone());
                    self.export_return(ret, gen, &mut instr_inner, ffi_rets);
                }
                instr.push(Instr::LowerTuple(ret, vars));
                instr.extend(instr_inner);
            }
        }
    }

    pub fn export(self, func: &AbiFunction) -> Export {
        let mut gen = VarGen::new();
        let mut ffi_args = vec![];
        let mut ffi_ffi_rets = vec![];
        let mut abi_args = vec![];
        let mut instr = vec![];
        let self_ = match &func.ty {
            FunctionType::Method(object) => {
                let out = gen.gen(AbiType::RefObject(object.clone()));
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                instr.push(Instr::LiftRefObject(ptr, out.clone(), object.clone()));
                Some(out)
            }
            FunctionType::NextIter(_, ty) => {
                let out = gen.gen(AbiType::RefIter(Box::new(ty.clone())));
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                instr.push(Instr::LiftRefIter(ptr, out.clone(), ty.clone()));
                Some(out)
            }
            FunctionType::PollFuture(_, ty) => {
                let out = gen.gen(AbiType::RefFuture(Box::new(ty.clone())));
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                instr.push(Instr::LiftRefFuture(ptr, out.clone(), ty.clone()));
                Some(out)
            }
            FunctionType::PollStream(_, ty) => {
                let out = gen.gen(AbiType::RefStream(Box::new(ty.clone())));
                let ptr = gen.gen_num(self.iptr());
                ffi_args.push(ptr.clone());
                instr.push(Instr::LiftRefStream(ptr, out.clone(), ty.clone()));
                Some(out)
            }
            _ => None,
        };
        for (_, ty) in func.args.iter() {
            let out = gen.gen(ty.clone());
            abi_args.push(out.clone());
            self.export_arg(out, &mut gen, &mut instr, &mut ffi_args);
        }
        let abi_ret = func.ret.as_ref().map(|ret| gen.gen(ret.clone()));
        instr.push(Instr::CallAbi(
            func.ty.clone(),
            self_,
            func.name.clone(),
            abi_ret.clone(),
            abi_args,
        ));
        if let Some(abi_ret) = abi_ret {
            let mut instr_ret = vec![];
            self.export_return(abi_ret, &mut gen, &mut instr_ret, &mut ffi_ffi_rets);
            if !ffi_ffi_rets.is_empty() {
                instr.push(Instr::DefineRets(ffi_ffi_rets.clone()));
            }
            instr.extend(instr_ret);
        }
        Export {
            symbol: func.symbol(),
            instr,
            ffi_args: ffi_args,
            ffi_ret: func.ret(ffi_ffi_rets),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Instr {
    LiftNum(Var, Var),
    LowerNum(Var, Var),
    LowerNumAsU32Tuple(Var, Var, Var, NumType),
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
    LiftOption(Var, Var, Var, Vec<Instr>),
    LowerOption(Var, Var, Var, Vec<Instr>),
    LowerResult(Var, Var, Var, Vec<Instr>, Var, Vec<Instr>),
    LiftIter(Var, Var, AbiType),
    LowerIter(Var, Var, AbiType),
    LiftRefIter(Var, Var, AbiType),
    LowerRefIter(Var, Var, AbiType),
    LiftFuture(Var, Var, AbiType),
    LowerFuture(Var, Var, AbiType),
    LiftRefFuture(Var, Var, AbiType),
    LowerRefFuture(Var, Var, AbiType),
    LiftStream(Var, Var, AbiType),
    LowerStream(Var, Var, AbiType),
    LiftRefStream(Var, Var, AbiType),
    LowerRefStream(Var, Var, AbiType),
    LiftTuple(Vec<Var>, Var),
    LowerTuple(Var, Vec<Var>),
    CallAbi(FunctionType, Option<Var>, String, Option<Var>, Vec<Var>),
    DefineRets(Vec<Var>),
}
