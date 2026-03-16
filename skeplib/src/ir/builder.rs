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
        func.params.push(crate::ir::program::IrParam {
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
        func.locals.push(crate::ir::program::IrLocal {
            id,
            name: name.into(),
            ty,
        });
        id
    }

    pub fn push_temp(&mut self, func: &mut IrFunction, ty: IrType) -> TempId {
        let id = TempId(self.next_temp);
        self.next_temp += 1;
        func.temps.push(crate::ir::program::IrTemp { id, ty });
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
        if let Some(target) = self.block_mut(func, block) {
            target.instrs.push(instr);
        }
    }

    pub fn set_terminator(&self, func: &mut IrFunction, block: BlockId, terminator: Terminator) {
        if let Some(target) = self.block_mut(func, block) {
            target.terminator = terminator;
        }
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
