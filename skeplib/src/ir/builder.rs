use crate::ir::{
    BasicBlock, BlockId, FunctionId, Instr, IrFunction, IrProgram, IrType, LocalId, ParamId,
    TempId, Terminator,
};

pub struct IrBuilder {
    next_function: usize,
    next_block: usize,
    next_param: usize,
    next_local: usize,
    next_temp: usize,
}

impl IrBuilder {
    pub fn new() -> Self {
        Self {
            next_function: 0,
            next_block: 0,
            next_param: 0,
            next_local: 0,
            next_temp: 0,
        }
    }

    pub fn begin_program(&mut self) -> IrProgram {
        IrProgram::new()
    }

    pub fn begin_function(&mut self, name: impl Into<String>, ret_ty: IrType) -> IrFunction {
        let id = FunctionId(self.next_function);
        self.next_function += 1;

        let entry = self.alloc_block();
        let mut func = IrFunction {
            id,
            name: name.into(),
            params: Vec::new(),
            locals: Vec::new(),
            temps: Vec::new(),
            ret_ty,
            entry,
            blocks: Vec::new(),
        };
        func.blocks.push(BasicBlock::new(entry, "entry"));
        func
    }

    pub fn push_param(
        &mut self,
        func: &mut IrFunction,
        name: impl Into<String>,
        ty: IrType,
    ) -> ParamId {
        let id = ParamId(self.next_param);
        self.next_param += 1;
        func.params.push(crate::ir::IrParam {
            id,
            name: name.into(),
            ty,
        });
        id
    }

    pub fn push_local(
        &mut self,
        func: &mut IrFunction,
        name: impl Into<String>,
        ty: IrType,
    ) -> LocalId {
        let id = LocalId(self.next_local);
        self.next_local += 1;
        func.locals.push(crate::ir::IrLocal {
            id,
            name: name.into(),
            ty,
        });
        id
    }

    pub fn push_temp(&mut self, func: &mut IrFunction, ty: IrType) -> TempId {
        let id = TempId(self.next_temp);
        self.next_temp += 1;
        func.temps.push(crate::ir::IrTemp { id, ty });
        id
    }

    pub fn push_block(&mut self, func: &mut IrFunction, name: impl Into<String>) -> BlockId {
        let id = self.alloc_block();
        func.blocks.push(BasicBlock::new(id, name));
        id
    }

    pub fn block_mut<'a>(
        &self,
        func: &'a mut IrFunction,
        id: BlockId,
    ) -> Option<&'a mut BasicBlock> {
        func.blocks.iter_mut().find(|block| block.id == id)
    }

    pub fn push_instr(&self, func: &mut IrFunction, block: BlockId, instr: Instr) {
        let target = self
            .block_mut(func, block)
            .unwrap_or_else(|| panic!("attempted to push IR instr to missing block {:?}", block));
        target.instrs.push(instr);
    }

    pub fn set_terminator(&self, func: &mut IrFunction, block: BlockId, terminator: Terminator) {
        let target = self.block_mut(func, block).unwrap_or_else(|| {
            panic!(
                "attempted to set IR terminator on missing block {:?}",
                block
            )
        });
        target.terminator = terminator;
    }

    fn alloc_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }
}

impl Default for IrBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "attempted to push IR instr to missing block")]
    fn builder_panics_when_pushing_instr_to_missing_block() {
        let mut builder = IrBuilder::new();
        let mut func = builder.begin_function("main", IrType::Int);
        let dst = builder.push_temp(&mut func, IrType::Int);
        builder.push_instr(
            &mut func,
            BlockId(999),
            Instr::Const {
                dst,
                ty: IrType::Int,
                value: crate::ir::ConstValue::Int(1),
            },
        );
    }

    #[test]
    #[should_panic(expected = "attempted to set IR terminator on missing block")]
    fn builder_panics_when_setting_terminator_on_missing_block() {
        let mut builder = IrBuilder::new();
        let mut func = builder.begin_function("main", IrType::Int);
        builder.set_terminator(&mut func, BlockId(999), Terminator::Return(None));
    }
}
