use super::VarGen;
use crate::{Abi, AbiFunction, AbiType, FunctionType, NumType, Return, Var};

#[derive(Clone, Debug)]
pub struct Import {
    pub symbol: String,
    pub args: Vec<Var>,
    pub instr: Vec<Instr>,
    pub ret: Return,
}

impl Abi {
    fn import_arg(
        self,
        arg: Var,
        gen: &mut VarGen,
        args: &mut Vec<Var>,
        import: &mut Vec<Instr>,
        import_cleanup: &mut Vec<Instr>,
    ) {
        match &arg.ty {
            AbiType::Num(num) => {
                let out = gen.gen_num(*num);
                import.push(Instr::LowerNum(arg.clone(), out.clone(), *num));
                args.push(out);
            }
            AbiType::Isize => {
                let out = gen.gen_num(self.iptr());
                import.push(Instr::LowerNum(arg, out.clone(), self.iptr()));
                args.push(out);
            }
            AbiType::Usize => {
                let out = gen.gen_num(self.uptr());
                import.push(Instr::LowerNum(arg, out.clone(), self.uptr()));
                args.push(out);
            }
            AbiType::Bool => {
                let out = gen.gen_num(NumType::U8);
                import.push(Instr::LowerBool(arg, out.clone()));
                args.push(out);
            }
            AbiType::RefStr => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                import.push(Instr::LowerString(
                    arg.clone(),
                    ptr.clone(),
                    len.clone(),
                    1,
                    1,
                ));
                args.extend_from_slice(&[ptr.clone(), len.clone()]);
                import_cleanup.push(Instr::Deallocate(ptr, len, 1, 1));
            }
            AbiType::String => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                import.push(Instr::LowerString(
                    arg.clone(),
                    ptr.clone(),
                    len.clone(),
                    1,
                    1,
                ));
                args.extend_from_slice(&[ptr, len.clone(), len]);
            }
            AbiType::RefSlice(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let (size, align) = self.layout(*ty);
                import.push(Instr::LowerVec(
                    arg.clone(),
                    ptr.clone(),
                    len.clone(),
                    *ty,
                    size,
                    align,
                ));
                args.extend_from_slice(&[ptr.clone(), len.clone()]);
                import_cleanup.push(Instr::Deallocate(ptr, len, size, align));
            }
            AbiType::Vec(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let (size, align) = self.layout(*ty);
                import.push(Instr::LowerVec(
                    arg.clone(),
                    ptr.clone(),
                    len.clone(),
                    *ty,
                    size,
                    align,
                ));
                args.extend_from_slice(&[ptr, len.clone(), len]);
            }
            AbiType::RefObject(_) => {
                let ptr = gen.gen_num(self.iptr());
                import.push(Instr::BorrowObject(arg.clone(), ptr.clone()));
                args.push(ptr);
            }
            AbiType::Object(_) => {
                let ptr = gen.gen_num(self.iptr());
                import.push(Instr::MoveObject(arg.clone(), ptr.clone()));
                args.push(ptr);
            }
            AbiType::RefFuture(_ty) => todo!(),
            AbiType::Future(_) => {
                let ptr = gen.gen_num(self.iptr());
                import.push(Instr::MoveFuture(arg.clone(), ptr.clone()));
                args.push(ptr);
            }
            AbiType::RefStream(_ty) => todo!(),
            AbiType::Stream(_) => {
                let ptr = gen.gen_num(self.iptr());
                import.push(Instr::MoveStream(arg.clone(), ptr.clone()));
                args.push(ptr);
            }
            AbiType::Option(_ty) => todo!(),
            AbiType::Result(_ty) => todo!(),
            AbiType::Tuple(_ty) => todo!(),
        }
    }

    fn import_return(
        self,
        symbol: &str,
        ty: &AbiType,
        out: Var,
        gen: &mut VarGen,
        rets: &mut Vec<Var>,
        import: &mut Vec<Instr>,
    ) {
        match ty {
            AbiType::Num(num)
                if matches!((self, num), (Abi::Wasm32, NumType::U64 | NumType::I64)) =>
            {
                let low = gen.gen_num(NumType::U32);
                let high = gen.gen_num(NumType::U32);
                rets.extend_from_slice(&[low.clone(), high.clone()]);
                import.push(Instr::LiftNumFromU32Tuple(low, high, out, *num));
            }
            AbiType::Num(num) => {
                let var = gen.gen_num(*num);
                rets.push(var.clone());
                import.push(Instr::LiftNum(var, out, *num));
            }
            AbiType::Isize => {
                let var = gen.gen_num(self.iptr());
                rets.push(var.clone());
                import.push(Instr::LiftNum(var, out, self.iptr()));
            }
            AbiType::Usize => {
                let var = gen.gen_num(self.uptr());
                rets.push(var.clone());
                import.push(Instr::LiftNum(var, out, self.uptr()));
            }
            AbiType::Bool => {
                let var = gen.gen_num(NumType::U8);
                rets.push(var.clone());
                import.push(Instr::LiftBool(var, out));
            }
            AbiType::RefStr => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                rets.push(ptr.clone());
                rets.push(len.clone());
                import.push(Instr::LiftString(ptr, len, out));
            }
            AbiType::String => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                rets.push(ptr.clone());
                rets.push(len.clone());
                rets.push(cap.clone());
                import.push(Instr::LiftString(ptr.clone(), len, out));
                import.push(Instr::Deallocate(ptr, cap, 1, 1));
            }
            AbiType::RefSlice(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                rets.push(ptr.clone());
                rets.push(len.clone());
                import.push(Instr::LiftVec(ptr, len, out, *ty));
            }
            AbiType::Vec(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                rets.push(ptr.clone());
                rets.push(len.clone());
                rets.push(cap.clone());
                let (size, align) = self.layout(*ty);
                import.push(Instr::LiftVec(ptr.clone(), len, out, *ty));
                import.push(Instr::Deallocate(ptr, cap, size, align));
            }
            AbiType::RefObject(_obj) => todo!(),
            AbiType::Object(obj) => {
                let ptr = gen.gen_num(self.iptr());
                rets.push(ptr.clone());
                let destructor = format!("drop_box_{}", obj);
                import.push(Instr::LiftObject(obj.clone(), ptr, destructor, out));
            }
            AbiType::Option(ty) => {
                let var = gen.gen_num(NumType::U8);
                rets.push(var.clone());
                import.push(Instr::HandleNull(var));
                self.import_return(symbol, &**ty, out, gen, rets, import);
            }
            AbiType::Result(ty) => {
                let var = gen.gen_num(NumType::U8);
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                let cap = gen.gen_num(self.uptr());
                rets.push(var.clone());
                rets.push(ptr.clone());
                rets.push(len.clone());
                rets.push(cap.clone());
                import.push(Instr::HandleError(var, ptr, len, cap));
                self.import_return(symbol, &**ty, out, gen, rets, import);
            }
            AbiType::RefFuture(_ty) => todo!(),
            AbiType::Future(_ty) => {
                let ptr = gen.gen_num(self.iptr());
                rets.push(ptr.clone());
                let poll = format!("{}_future_poll", symbol);
                let destructor = format!("{}_future_drop", symbol);
                import.push(Instr::LiftFuture(ptr, poll, destructor, out));
            }
            AbiType::RefStream(_ty) => todo!(),
            AbiType::Stream(_ty) => {
                let ptr = gen.gen_num(self.iptr());
                rets.push(ptr.clone());
                let poll = format!("{}_stream_poll", symbol);
                let destructor = format!("{}_stream_drop", symbol);
                import.push(Instr::LiftStream(ptr, poll, destructor, out));
            }
            AbiType::Tuple(_ty) => todo!(),
        }
    }

    pub fn import(self, func: &AbiFunction) -> Import {
        let symbol = func.symbol();
        let mut gen = VarGen::new();
        let mut args = vec![];
        let mut rets = vec![];
        let mut import = vec![];
        let mut import_cleanup = vec![];
        match &func.ty {
            FunctionType::Method(_) => {
                let self_ = gen.gen_num(self.iptr());
                import.push(Instr::BorrowSelf(self_.clone()));
                args.push(self_);
            }
            FunctionType::PollFuture(_, _) => {
                let arg = gen.gen_num(self.iptr());
                import.push(Instr::BindArg("boxed".to_string(), arg.clone()));
                self.import_arg(arg, &mut gen, &mut args, &mut import, &mut import_cleanup);
            }
            FunctionType::PollStream(_, _) => {
                let arg = gen.gen_num(self.iptr());
                import.push(Instr::BindArg("boxed".to_string(), arg.clone()));
                self.import_arg(arg, &mut gen, &mut args, &mut import, &mut import_cleanup);
            }
            _ => {}
        }
        for (name, ty) in func.args.iter() {
            let arg = gen.gen(ty.clone());
            import.push(Instr::BindArg(name.clone(), arg.clone()));
            self.import_arg(arg, &mut gen, &mut args, &mut import, &mut import_cleanup);
        }
        let ret = func.ret.as_ref().map(|ty| gen.gen(ty.clone()));
        import.push(Instr::Call(symbol.clone(), ret.clone(), args.clone()));
        if let Some(ret) = ret {
            let out = gen.gen(ret.ty.clone());
            let mut instr = vec![];
            self.import_return(
                &symbol,
                &ret.ty,
                out.clone(),
                &mut gen,
                &mut rets,
                &mut instr,
            );
            import.push(Instr::BindRets(ret.clone(), rets.clone()));
            import.extend(instr);
            import.extend(import_cleanup);
            import.push(Instr::ReturnValue(out));
        } else {
            import.extend(import_cleanup);
            import.push(Instr::ReturnVoid);
        }
        Import {
            symbol,
            args,
            instr: import,
            ret: func.ret(rets),
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
    LowerString(Var, Var, Var, usize, usize),
    LiftVec(Var, Var, Var, NumType),
    LowerVec(Var, Var, Var, NumType, usize, usize),
    HandleNull(Var),
    HandleError(Var, Var, Var, Var),
    BorrowSelf(Var),
    BorrowObject(Var, Var),
    MoveObject(Var, Var),
    LiftObject(String, Var, String, Var),
    MoveFuture(Var, Var),
    LiftFuture(Var, String, String, Var),
    MoveStream(Var, Var),
    LiftStream(Var, String, String, Var),
    Call(String, Option<Var>, Vec<Var>),
    BindRets(Var, Vec<Var>),
    ReturnValue(Var),
    ReturnVoid,
}
