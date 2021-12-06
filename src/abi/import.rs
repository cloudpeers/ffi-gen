use super::VarGen;
use crate::{Abi, AbiFunction, AbiType, FunctionType, NumType, Return, Var};

#[derive(Clone, Debug)]
pub struct Import {
    pub symbol: String,
    pub abi_args: Vec<(String, AbiType)>,
    pub ffi_args: Vec<Var>,
    pub instr: Vec<Instr>,
    pub ffi_ret: Return,
    pub abi_ret: Option<AbiType>,
}

impl Abi {
    fn import_arg(
        self,
        arg: Var,
        gen: &mut VarGen,
        ffi_args: &mut Vec<Var>,
        instr: &mut Vec<Instr>,
        instr_cleanup: &mut Vec<Instr>,
    ) {
        match &arg.ty {
            AbiType::Num(num) => {
                let out = gen.gen_num(*num);
                instr.push(Instr::LowerNum(arg.clone(), out.clone(), *num));
                ffi_args.push(out);
            }
            AbiType::Isize => {
                let out = gen.gen_num(self.iptr());
                instr.push(Instr::LowerNum(arg, out.clone(), self.iptr()));
                ffi_args.push(out);
            }
            AbiType::Usize => {
                let out = gen.gen_num(self.uptr());
                instr.push(Instr::LowerNum(arg, out.clone(), self.uptr()));
                ffi_args.push(out);
            }
            AbiType::Bool => {
                let out = gen.gen_num(NumType::U8);
                instr.push(Instr::LowerBool(arg, out.clone()));
                ffi_args.push(out);
            }
            AbiType::RefStr => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                instr.push(Instr::DefineArgs(vec![cap.clone()]));
                instr.push(Instr::LowerString(
                    arg.clone(),
                    ptr.clone(),
                    len.clone(),
                    cap.clone(),
                    1,
                    1,
                ));
                ffi_args.extend_from_slice(&[ptr.clone(), len]);
                instr_cleanup.push(Instr::Deallocate(ptr, cap, 1, 1));
            }
            AbiType::String => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                instr.push(Instr::LowerString(
                    arg.clone(),
                    ptr.clone(),
                    len.clone(),
                    cap.clone(),
                    1,
                    1,
                ));
                ffi_args.extend_from_slice(&[ptr, len, cap]);
            }
            AbiType::RefSlice(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                let (size, align) = self.layout(*ty);
                instr.push(Instr::DefineArgs(vec![cap.clone()]));
                instr.push(Instr::LowerVec(
                    arg.clone(),
                    ptr.clone(),
                    len.clone(),
                    cap.clone(),
                    *ty,
                    size,
                    align,
                ));
                ffi_args.extend_from_slice(&[ptr.clone(), len]);
                instr_cleanup.push(Instr::Deallocate(ptr, cap, size, align));
            }
            AbiType::Vec(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                let (size, align) = self.layout(*ty);
                instr.push(Instr::LowerVec(
                    arg.clone(),
                    ptr.clone(),
                    len.clone(),
                    cap.clone(),
                    *ty,
                    size,
                    align,
                ));
                ffi_args.extend_from_slice(&[ptr, len, cap]);
            }
            AbiType::RefObject(_) => {
                let ptr = gen.gen_num(self.iptr());
                instr.push(Instr::BorrowObject(arg.clone(), ptr.clone()));
                ffi_args.push(ptr);
            }
            AbiType::Object(_) => {
                let ptr = gen.gen_num(self.iptr());
                instr.push(Instr::MoveObject(arg.clone(), ptr.clone()));
                ffi_args.push(ptr);
            }
            AbiType::RefIter(_) => {
                let ptr = gen.gen_num(self.iptr());
                instr.push(Instr::BorrowIter(arg.clone(), ptr.clone()));
                ffi_args.push(ptr);
            }
            AbiType::Iter(_) => {
                let ptr = gen.gen_num(self.iptr());
                instr.push(Instr::MoveIter(arg.clone(), ptr.clone()));
                ffi_args.push(ptr);
            }
            AbiType::RefFuture(_) => {
                let ptr = gen.gen_num(self.iptr());
                instr.push(Instr::BorrowFuture(arg.clone(), ptr.clone()));
                ffi_args.push(ptr);
            }
            AbiType::Future(_) => {
                let ptr = gen.gen_num(self.iptr());
                instr.push(Instr::MoveFuture(arg.clone(), ptr.clone()));
                ffi_args.push(ptr);
            }
            AbiType::RefStream(_) => {
                let ptr = gen.gen_num(self.iptr());
                instr.push(Instr::BorrowStream(arg.clone(), ptr.clone()));
                ffi_args.push(ptr);
            }
            AbiType::Stream(_) => {
                let ptr = gen.gen_num(self.iptr());
                instr.push(Instr::MoveStream(arg.clone(), ptr.clone()));
                ffi_args.push(ptr);
            }
            AbiType::Option(ty) => {
                let var = gen.gen_num(NumType::U8);
                let some = gen.gen((&**ty).clone());
                ffi_args.push(var.clone());
                let mut some_instr = vec![];
                self.import_arg(some.clone(), gen, ffi_args, &mut some_instr, instr_cleanup);
                instr.push(Instr::LowerOption(arg, var, some, some_instr));
            }
            AbiType::Tuple(_) => unreachable!(),
            AbiType::Result(_) => todo!(),
        }
    }

    fn import_return(
        self,
        symbol: &str,
        ty: &AbiType,
        out: Var,
        gen: &mut VarGen,
        ffi_rets: &mut Vec<Var>,
        instr: &mut Vec<Instr>,
    ) {
        match ty {
            AbiType::Num(num)
                if matches!((self, num), (Abi::Wasm32, NumType::U64 | NumType::I64)) =>
            {
                let low = gen.gen_num(NumType::U32);
                let high = gen.gen_num(NumType::U32);
                ffi_rets.extend_from_slice(&[low.clone(), high.clone()]);
                instr.push(Instr::LiftNumFromU32Tuple(low, high, out, *num));
            }
            AbiType::Num(num) => {
                let var = gen.gen_num(*num);
                ffi_rets.push(var.clone());
                instr.push(Instr::LiftNum(var, out, *num));
            }
            AbiType::Isize => {
                let var = gen.gen_num(self.iptr());
                ffi_rets.push(var.clone());
                instr.push(Instr::LiftNum(var, out, self.iptr()));
            }
            AbiType::Usize => {
                let var = gen.gen_num(self.uptr());
                ffi_rets.push(var.clone());
                instr.push(Instr::LiftNum(var, out, self.uptr()));
            }
            AbiType::Bool => {
                let var = gen.gen_num(NumType::U8);
                ffi_rets.push(var.clone());
                instr.push(Instr::LiftBool(var, out));
            }
            AbiType::RefStr => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                ffi_rets.extend_from_slice(&[ptr.clone(), len.clone()]);
                instr.push(Instr::LiftString(ptr, len, out));
            }
            AbiType::String => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                ffi_rets.extend_from_slice(&[ptr.clone(), len.clone(), cap.clone()]);
                instr.push(Instr::LiftString(ptr.clone(), len, out));
                instr.push(Instr::Deallocate(ptr, cap, 1, 1));
            }
            AbiType::RefSlice(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                ffi_rets.extend_from_slice(&[ptr.clone(), len.clone()]);
                instr.push(Instr::LiftVec(ptr, len, out, *ty));
            }
            AbiType::Vec(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                ffi_rets.extend_from_slice(&[ptr.clone(), len.clone(), cap.clone()]);
                let (size, align) = self.layout(*ty);
                instr.push(Instr::LiftVec(ptr.clone(), len, out, *ty));
                instr.push(Instr::Deallocate(ptr, cap, size, align));
            }
            AbiType::RefObject(_) => todo!(),
            AbiType::Object(obj) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                let destructor = format!("drop_box_{}", obj);
                instr.push(Instr::LiftObject(obj.clone(), ptr, destructor, out));
            }
            AbiType::Option(ty) => {
                let var = gen.gen_num(NumType::U8);
                ffi_rets.push(var.clone());
                instr.push(Instr::HandleNull(var));
                self.import_return(symbol, &**ty, out, gen, ffi_rets, instr);
            }
            AbiType::Result(ty) => {
                let var = gen.gen_num(NumType::U8);
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                ffi_rets.extend_from_slice(&[var.clone(), ptr.clone(), len.clone(), cap.clone()]);
                instr.push(Instr::HandleError(var, ptr, len, cap));
                self.import_return(symbol, &**ty, out, gen, ffi_rets, instr);
            }
            AbiType::RefIter(_) => todo!(),
            AbiType::Iter(_) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                let next = format!("{}_iter_next", symbol);
                let destructor = format!("{}_iter_drop", symbol);
                instr.push(Instr::LiftIter(ptr, next, destructor, out));
            }
            AbiType::RefFuture(_) => todo!(),
            AbiType::Future(_) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                let poll = format!("{}_future_poll", symbol);
                let destructor = format!("{}_future_drop", symbol);
                instr.push(Instr::LiftFuture(ptr, poll, destructor, out));
            }
            AbiType::RefStream(_) => todo!(),
            AbiType::Stream(_) => {
                let ptr = gen.gen_num(self.iptr());
                ffi_rets.push(ptr.clone());
                let poll = format!("{}_stream_poll", symbol);
                let destructor = format!("{}_stream_drop", symbol);
                instr.push(Instr::LiftStream(ptr, poll, destructor, out));
            }
            AbiType::Tuple(tys) => {
                let mut vars = vec![];
                for ty in tys {
                    let out = gen.gen(ty.clone());
                    vars.push(out.clone());
                    self.import_return(symbol, ty, out, gen, ffi_rets, instr);
                }
                instr.push(Instr::LiftTuple(vars, out));
            }
        }
    }

    pub fn import(self, func: &AbiFunction) -> Import {
        let symbol = func.symbol();
        let mut gen = VarGen::new();
        let mut ffi_args = vec![];
        let mut ffi_rets = vec![];
        let mut abi_args = vec![];
        let mut instr = vec![];
        let mut instr_cleanup = vec![];
        let mut instr_arg = vec![];
        match &func.ty {
            FunctionType::Method(_) => {
                let self_ = gen.gen_num(self.iptr());
                instr_arg.push(Instr::BorrowSelf(self_.clone()));
                ffi_args.push(self_);
            }
            FunctionType::NextIter(_, _)
            | FunctionType::PollFuture(_, _)
            | FunctionType::PollStream(_, _) => {
                abi_args.push(("boxed".to_string(), AbiType::Isize));
            }
            _ => {}
        }
        for (name, ty) in func.args.iter() {
            match ty {
                AbiType::Tuple(tys) => {
                    for (i, ty) in tys.iter().enumerate() {
                        abi_args.push((format!("{}{}", name, i), ty.clone()));
                    }
                }
                _ => abi_args.push((name.clone(), ty.clone())),
            }
        }
        for (name, ty) in abi_args.iter() {
            let arg = gen.gen(ty.clone());
            instr.push(Instr::BindArg(name.clone(), arg.clone()));
            self.import_arg(
                arg,
                &mut gen,
                &mut ffi_args,
                &mut instr_arg,
                &mut instr_cleanup,
            );
        }
        if !ffi_args.is_empty() {
            instr.push(Instr::DefineArgs(ffi_args.clone()));
        }
        instr.extend(instr_arg);
        let abi_ret = if let Some(AbiType::Tuple(tuple)) = func.ret.as_ref() {
            if tuple.is_empty() {
                None
            } else {
                func.ret.clone()
            }
        } else {
            func.ret.clone()
        };
        let ret = abi_ret.as_ref().map(|ty| gen.gen(ty.clone()));
        instr.push(Instr::Call(symbol.clone(), ret.clone(), ffi_args.clone()));
        if let Some(ret) = ret {
            let out = gen.gen(ret.ty.clone());
            let mut instr_ret = vec![];
            self.import_return(
                &symbol,
                &ret.ty,
                out.clone(),
                &mut gen,
                &mut ffi_rets,
                &mut instr_ret,
            );
            instr.push(Instr::BindRets(ret.clone(), ffi_rets.clone()));
            instr.extend(instr_ret);
            instr.extend(instr_cleanup);
            instr.push(Instr::ReturnValue(out));
        } else {
            instr.extend(instr_cleanup);
            instr.push(Instr::ReturnVoid);
        }
        Import {
            symbol,
            abi_args,
            ffi_args,
            instr,
            ffi_ret: func.ret(ffi_rets),
            abi_ret,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Instr {
    BindArg(String, Var),
    Deallocate(Var, Var, usize, usize),
    LiftNum(Var, Var, NumType),
    LowerNum(Var, Var, NumType),
    LiftNumFromU32Tuple(Var, Var, Var, NumType),
    LiftBool(Var, Var),
    LowerBool(Var, Var),
    LiftString(Var, Var, Var),
    LowerString(Var, Var, Var, Var, usize, usize),
    LiftVec(Var, Var, Var, NumType),
    LowerVec(Var, Var, Var, Var, NumType, usize, usize),
    HandleNull(Var),
    LowerOption(Var, Var, Var, Vec<Instr>),
    HandleError(Var, Var, Var, Var),
    BorrowSelf(Var),
    BorrowObject(Var, Var),
    MoveObject(Var, Var),
    LiftObject(String, Var, String, Var),
    BorrowIter(Var, Var),
    MoveIter(Var, Var),
    LiftIter(Var, String, String, Var),
    BorrowFuture(Var, Var),
    MoveFuture(Var, Var),
    LiftFuture(Var, String, String, Var),
    BorrowStream(Var, Var),
    MoveStream(Var, Var),
    LiftStream(Var, String, String, Var),
    LiftTuple(Vec<Var>, Var),
    DefineArgs(Vec<Var>),
    Call(String, Option<Var>, Vec<Var>),
    BindRets(Var, Vec<Var>),
    ReturnValue(Var),
    ReturnVoid,
}
