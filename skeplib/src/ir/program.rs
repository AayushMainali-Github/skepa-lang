use crate::ir::{BasicBlock, BlockId, FunctionId, IrType, StructId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructField {
    pub name: String,
    pub ty: IrType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrStruct {
    pub id: StructId,
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrGlobal {
    pub id: crate::ir::GlobalId,
    pub name: String,
    pub ty: IrType,
    pub init: Option<crate::ir::Operand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrParam {
    pub id: crate::ir::ParamId,
    pub name: String,
    pub ty: IrType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrLocal {
    pub id: crate::ir::LocalId,
    pub name: String,
    pub ty: IrType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrTemp {
    pub id: crate::ir::TempId,
    pub ty: IrType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrFunction {
    pub id: FunctionId,
    pub name: String,
    pub params: Vec<IrParam>,
    pub locals: Vec<IrLocal>,
    pub temps: Vec<IrTemp>,
    pub ret_ty: IrType,
    pub entry: BlockId,
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrModuleInit {
    pub function: FunctionId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrProgram {
    pub structs: Vec<IrStruct>,
    pub globals: Vec<IrGlobal>,
    pub functions: Vec<IrFunction>,
    pub module_init: Option<IrModuleInit>,
}

impl IrProgram {
    pub fn new() -> Self {
        Self {
            structs: Vec::new(),
            globals: Vec::new(),
            functions: Vec::new(),
            module_init: None,
        }
    }
}

impl Default for IrProgram {
    fn default() -> Self {
        Self::new()
    }
}
