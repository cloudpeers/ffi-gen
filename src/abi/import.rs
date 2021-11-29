use crate::{Abi, AbiFunction, AbiType, FunctionType, NumType};

#[derive(Clone, Debug)]
pub struct Import {
    pub symbol: String,
    pub args: Vec<(u32, NumType)>,
    pub ret: Vec<NumType>,
    pub instr: Vec<Instr>,
}

impl Abi {
    fn lower_arg(
        self,
        arg: u32,
        ty: &AbiType,
        ident: &mut u32,
        args: &mut Vec<(u32, NumType)>,
        import: &mut Vec<Instr>,
        import_cleanup: &mut Vec<Instr>,
    ) {
        match ty {
            AbiType::Num(num) => {
                let out = *ident;
                *ident += 1;
                import.push(Instr::LowerNum(arg, out, *num));
                args.push((out, *num));
            }
            AbiType::Isize => {
                let out = *ident;
                *ident += 1;
                import.push(Instr::LowerNum(arg, out, self.iptr()));
                args.push((out, self.iptr()));
            }
            AbiType::Usize => {
                let out = *ident;
                *ident += 1;
                import.push(Instr::LowerNum(arg, out, self.uptr()));
                args.push((out, self.uptr()));
            }
            AbiType::Bool => {
                let out = *ident;
                *ident += 1;
                import.push(Instr::LowerBool(arg, out));
                args.push((out, NumType::U8));
            }
            AbiType::RefStr => {
                let ptr = *ident;
                *ident += 1;
                let len = *ident;
                *ident += 1;
                import.push(Instr::StrLen(arg, len));
                import.push(Instr::Allocate(ptr, len, 1, 1));
                import.push(Instr::LowerString(arg, ptr, len));
                args.push((ptr, self.iptr()));
                args.push((len, self.uptr()));
                import_cleanup.push(Instr::Deallocate(ptr, len, 1, 1));
            }
            AbiType::String => {
                let ptr = *ident;
                *ident += 1;
                let len = *ident;
                *ident += 1;
                import.push(Instr::StrLen(arg, len));
                import.push(Instr::Allocate(ptr, len, 1, 1));
                import.push(Instr::LowerString(arg, ptr, len));
                args.push((ptr, self.iptr()));
                args.push((len, self.uptr()));
                args.push((len, self.uptr()));
            }
            AbiType::RefSlice(ty) => {
                let ptr = *ident;
                *ident += 1;
                let len = *ident;
                *ident += 1;
                import.push(Instr::VecLen(arg, len));
                let (size, align) = self.layout(*ty);
                import.push(Instr::Allocate(ptr, len, size, align));
                import.push(Instr::LowerVec(arg, ptr, len, *ty));
                args.push((ptr, self.iptr()));
                args.push((len, self.uptr()));
                import_cleanup.push(Instr::Deallocate(ptr, len, size, align));
            }
            AbiType::Vec(ty) => {
                let ptr = *ident;
                *ident += 1;
                let len = *ident;
                *ident += 1;
                import.push(Instr::VecLen(arg, len));
                let (size, align) = self.layout(*ty);
                import.push(Instr::Allocate(ptr, len, size, align));
                import.push(Instr::LowerVec(arg, ptr, len, *ty));
                args.push((ptr, self.iptr()));
                args.push((len, self.uptr()));
                args.push((len, self.uptr()));
            }
            AbiType::RefObject(_) => {
                let ptr = *ident;
                *ident += 1;
                import.push(Instr::BorrowObject(arg, ptr));
                args.push((ptr, self.iptr()));
            }
            AbiType::Object(_) => {
                let ptr = *ident;
                *ident += 1;
                import.push(Instr::MoveObject(arg, ptr));
                args.push((ptr, self.iptr()));
            }
            AbiType::Future(_) => {
                let ptr = *ident;
                *ident += 1;
                import.push(Instr::MoveFuture(arg, ptr));
                args.push((ptr, self.iptr()));
            }
            AbiType::Stream(_) => {
                let ptr = *ident;
                *ident += 1;
                import.push(Instr::MoveStream(arg, ptr));
                args.push((ptr, self.iptr()));
            }
            AbiType::Option(_ty) => todo!(),
            AbiType::Result(_ty) => todo!(),
        }
    }

    fn lower_ret(
        self,
        ret: u32,
        ty: Option<&AbiType>,
        ident: &mut u32,
        rets: &mut Vec<NumType>,
        import: &mut Vec<Instr>,
        import_return: &mut Vec<Instr>,
    ) {
        match ty {
            Some(AbiType::Num(num)) => {
                let out = *ident;
                *ident += 1;
                rets.push(*num);
                import.push(Instr::LiftNum(ret, out, *num));
                import_return.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Isize) => {
                let out = *ident;
                *ident += 1;
                rets.push(self.iptr());
                import.push(Instr::LiftNum(ret, out, self.iptr()));
                import_return.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Usize) => {
                let out = *ident;
                *ident += 1;
                rets.push(self.uptr());
                import.push(Instr::LiftNum(ret, out, self.uptr()));
                import_return.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Bool) => {
                let out = *ident;
                *ident += 1;
                rets.push(NumType::U8);
                import.push(Instr::LiftBool(ret, out));
                import_return.push(Instr::ReturnValue(out));
            }
            Some(AbiType::RefStr) => {
                let (ptr, len, out) = (*ident, *ident + 1, *ident + 2);
                *ident += 3;
                rets.push(self.iptr());
                rets.push(self.uptr());
                import.push(Instr::BindRet(ret, 0, ptr));
                import.push(Instr::BindRet(ret, 1, len));
                import.push(Instr::LiftString(ptr, len, out));
                import_return.push(Instr::ReturnValue(out));
            }
            Some(AbiType::String) => {
                let (ptr, len, cap, out) = (*ident, *ident + 1, *ident + 2, *ident + 3);
                *ident += 4;
                rets.push(self.iptr());
                rets.push(self.uptr());
                rets.push(self.uptr());
                import.push(Instr::BindRet(ret, 0, ptr));
                import.push(Instr::BindRet(ret, 1, len));
                import.push(Instr::BindRet(ret, 2, cap));
                import.push(Instr::LiftString(ptr, len, out));
                import.push(Instr::Deallocate(ptr, cap, 1, 1));
                import_return.push(Instr::ReturnValue(out));
            }
            Some(AbiType::RefSlice(ty)) => {
                let (ptr, len, out) = (*ident, *ident + 1, *ident + 2);
                *ident += 3;
                rets.push(self.iptr());
                rets.push(self.uptr());
                import.push(Instr::BindRet(ret, 0, ptr));
                import.push(Instr::BindRet(ret, 1, len));
                import.push(Instr::LiftVec(ptr, len, out, *ty));
                import_return.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Vec(ty)) => {
                let (ptr, len, cap, out) = (*ident, *ident + 1, *ident + 2, *ident + 3);
                *ident += 4;
                rets.push(self.iptr());
                rets.push(self.uptr());
                rets.push(self.uptr());
                import.push(Instr::BindRet(ret, 0, ptr));
                import.push(Instr::BindRet(ret, 1, len));
                import.push(Instr::BindRet(ret, 2, cap));
                let (size, align) = self.layout(*ty);
                import.push(Instr::LiftVec(ptr, len, out, *ty));
                import.push(Instr::Deallocate(ptr, cap, size, align));
                import_return.push(Instr::ReturnValue(out));
            }
            Some(AbiType::RefObject(_obj)) => todo!(),
            Some(AbiType::Object(obj)) => {
                let out = *ident;
                *ident += 1;
                rets.push(self.iptr());
                let destructor = format!("drop_box_{}", obj);
                import.push(Instr::MakeObject(obj.clone(), ret, destructor, out));
                import_return.push(Instr::ReturnValue(out));
            }
            Some(AbiType::Future(_ty)) => todo!(),
            Some(AbiType::Stream(_ty)) => todo!(),
            Some(AbiType::Option(_ty)) => todo!(),
            Some(AbiType::Result(_ty)) => todo!(),
            None => {
                import_return.push(Instr::ReturnVoid);
            }
        }
    }

    pub fn import(self, func: &AbiFunction) -> Import {
        let symbol = match &func.ty {
            FunctionType::Constructor(obj) | FunctionType::Method(obj) => {
                format!("__{}_{}", obj, &func.name)
            }
            FunctionType::Function => format!("__{}", &func.name),
        };
        let mut ident = 0;
        let mut args = vec![];
        let mut rets = vec![];
        let mut import = vec![];
        let mut import_cleanup = vec![];
        let mut import_return = vec![];
        if let FunctionType::Method(_) = &func.ty {
            let self_ = ident;
            ident += 1;
            import.push(Instr::BorrowSelf(self_));
            args.push((self_, self.iptr()));
        }
        for (name, ty) in func.args.iter() {
            let arg = ident;
            ident += 1;
            import.push(Instr::BindArg(name.clone(), arg));
            self.lower_arg(
                arg,
                ty,
                &mut ident,
                &mut args,
                &mut import,
                &mut import_cleanup,
            );
        }
        let ret = ident;
        ident += 1;
        import.push(Instr::Call(
            symbol.clone(),
            ret,
            args.iter().map(|(ident, _)| *ident).collect(),
        ));
        self.lower_ret(
            ret,
            func.ret.as_ref(),
            &mut ident,
            &mut rets,
            &mut import,
            &mut import_return,
        );
        #[allow(clippy::drop_copy)]
        drop(ident);
        import.extend(import_cleanup);
        import.extend(import_return);
        Import {
            symbol,
            args,
            ret: rets,
            instr: import,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Instr {
    BorrowSelf(u32),
    BorrowObject(u32, u32),
    MoveObject(u32, u32),
    MoveFuture(u32, u32),
    MoveStream(u32, u32),
    MakeObject(String, u32, String, u32),
    BindArg(String, u32),
    BindRet(u32, usize, u32),
    LowerNum(u32, u32, NumType),
    LiftNum(u32, u32, NumType),
    LowerBool(u32, u32),
    LiftBool(u32, u32),
    StrLen(u32, u32),
    VecLen(u32, u32),
    Allocate(u32, u32, usize, usize),
    Deallocate(u32, u32, usize, usize),
    LowerString(u32, u32, u32),
    LiftString(u32, u32, u32),
    LowerVec(u32, u32, u32, NumType),
    LiftVec(u32, u32, u32, NumType),
    Call(String, u32, Vec<u32>),
    ReturnValue(u32),
    ReturnVoid,
}
