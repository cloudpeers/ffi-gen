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
                import.push(Instr::StrLen(arg.clone(), len.clone()));
                import.push(Instr::Allocate(ptr.clone(), len.clone(), 1, 1));
                import.push(Instr::LowerString(arg.clone(), ptr.clone(), len.clone()));
                args.extend_from_slice(&[ptr.clone(), len.clone()]);
                import_cleanup.push(Instr::Deallocate(ptr, len, 1, 1));
            }
            AbiType::String => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                import.push(Instr::StrLen(arg.clone(), len.clone()));
                import.push(Instr::Allocate(ptr.clone(), len.clone(), 1, 1));
                import.push(Instr::LowerString(arg.clone(), ptr.clone(), len.clone()));
                args.extend_from_slice(&[ptr, len.clone(), len]);
            }
            AbiType::RefSlice(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                import.push(Instr::VecLen(arg.clone(), len.clone()));
                let (size, align) = self.layout(*ty);
                import.push(Instr::Allocate(ptr.clone(), len.clone(), size, align));
                import.push(Instr::LowerVec(arg.clone(), ptr.clone(), len.clone(), *ty));
                args.extend_from_slice(&[ptr.clone(), len.clone()]);
                import_cleanup.push(Instr::Deallocate(ptr, len, size, align));
            }
            AbiType::Vec(ty) => {
                let ptr = gen.gen_num(self.iptr());
                let len = gen.gen_num(self.uptr());
                import.push(Instr::VecLen(arg.clone(), len.clone()));
                let (size, align) = self.layout(*ty);
                import.push(Instr::Allocate(ptr.clone(), len.clone(), size, align));
                import.push(Instr::LowerVec(arg.clone(), ptr.clone(), len.clone(), *ty));
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
            AbiType::Future(_) => {
                let ptr = gen.gen_num(self.iptr());
                import.push(Instr::MoveFuture(arg.clone(), ptr.clone()));
                args.push(ptr);
            }
            AbiType::Stream(_) => {
                let ptr = gen.gen_num(self.iptr());
                import.push(Instr::MoveStream(arg.clone(), ptr.clone()));
                args.push(ptr);
            }
            AbiType::Option(_ty) => todo!(),
            AbiType::Result(_ty) => todo!(),
        }
    }

    fn import_return(
        self,
        ty: &AbiType,
        out: Var,
        gen: &mut VarGen,
        rets: &mut Vec<Var>,
        import: &mut Vec<Instr>,
    ) {
        match ty {
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
                import.push(Instr::MakeObject(obj.clone(), ptr, destructor, out));
            }
            AbiType::Option(ty) => {
                let var = gen.gen_num(NumType::U8);
                rets.push(var.clone());
                import.push(Instr::HandleNull(var));
                self.import_return(&**ty, out, gen, rets, import);
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
                self.import_return(&**ty, out, gen, rets, import);
            }
            AbiType::Future(_ty) => todo!(),
            AbiType::Stream(_ty) => todo!(),
        }
    }

    pub fn import(self, func: &AbiFunction) -> Import {
        let symbol = match &func.ty {
            FunctionType::Constructor(obj) | FunctionType::Method(obj) => {
                format!("__{}_{}", obj, &func.name)
            }
            FunctionType::Function => format!("__{}", &func.name),
        };
        let mut gen = VarGen::new();
        let mut args = vec![];
        let mut rets = vec![];
        let mut import = vec![];
        let mut import_cleanup = vec![];
        if let FunctionType::Method(_) = &func.ty {
            let self_ = gen.gen_num(self.iptr());
            import.push(Instr::BorrowSelf(self_.clone()));
            args.push(self_);
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
            self.import_return(&ret.ty, out.clone(), &mut gen, &mut rets, &mut instr);
            import.push(Instr::BindRets(ret.clone(), rets.clone()));
            import.extend(instr);
            import.extend(import_cleanup);
            import.push(Instr::ReturnValue(out));
        } else {
            import.extend(import_cleanup);
            import.push(Instr::ReturnVoid);
        }
        let ret = match rets.len() {
            0 => Return::Void,
            1 => Return::Num(rets[0].clone()),
            _ => Return::Struct(rets, format!("{}Return", symbol)),
        };
        Import {
            symbol,
            args,
            instr: import,
            ret,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Instr {
    BorrowSelf(Var),
    BorrowObject(Var, Var),
    MoveObject(Var, Var),
    MoveFuture(Var, Var),
    MoveStream(Var, Var),
    MakeObject(String, Var, String, Var),
    BindArg(String, Var),
    LowerNum(Var, Var, NumType),
    LiftNum(Var, Var, NumType),
    LowerBool(Var, Var),
    LiftBool(Var, Var),
    StrLen(Var, Var),
    VecLen(Var, Var),
    Allocate(Var, Var, usize, usize),
    Deallocate(Var, Var, usize, usize),
    LowerString(Var, Var, Var),
    LiftString(Var, Var, Var),
    LowerVec(Var, Var, Var, NumType),
    LiftVec(Var, Var, Var, NumType),
    Call(String, Option<Var>, Vec<Var>),
    ReturnValue(Var),
    ReturnVoid,
    HandleNull(Var),
    HandleError(Var, Var, Var, Var),
    BindRets(Var, Vec<Var>),
}
