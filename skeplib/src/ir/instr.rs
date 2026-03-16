use crate::ir::{BlockId, FunctionId, IrType, Operand, StructId, TempId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicOp {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRef {
    pub index: usize,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinCall {
    pub package: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    Const {
        dst: TempId,
        ty: IrType,
        value: crate::ir::ConstValue,
    },
    Copy {
        dst: TempId,
        ty: IrType,
        src: Operand,
    },
    Unary {
        dst: TempId,
        ty: IrType,
        op: UnaryOp,
        operand: Operand,
    },
    Binary {
        dst: TempId,
        ty: IrType,
        op: BinaryOp,
        left: Operand,
        right: Operand,
    },
    Compare {
        dst: TempId,
        op: CmpOp,
        left: Operand,
        right: Operand,
    },
    Logic {
        dst: TempId,
        op: LogicOp,
        left: Operand,
        right: Operand,
    },
    LoadGlobal {
        dst: TempId,
        ty: IrType,
        global: crate::ir::GlobalId,
    },
    StoreGlobal {
        global: crate::ir::GlobalId,
        ty: IrType,
        value: Operand,
    },
    LoadLocal {
        dst: TempId,
        ty: IrType,
        local: crate::ir::LocalId,
    },
    StoreLocal {
        local: crate::ir::LocalId,
        ty: IrType,
        value: Operand,
    },
    MakeArray {
        dst: TempId,
        elem_ty: IrType,
        items: Vec<Operand>,
    },
    MakeArrayRepeat {
        dst: TempId,
        elem_ty: IrType,
        value: Operand,
        size: usize,
    },
    ArrayGet {
        dst: TempId,
        elem_ty: IrType,
        array: Operand,
        index: Operand,
    },
    ArraySet {
        elem_ty: IrType,
        array: Operand,
        index: Operand,
        value: Operand,
    },
    VecPush {
        vec: Operand,
        value: Operand,
    },
    VecGet {
        dst: TempId,
        elem_ty: IrType,
        vec: Operand,
        index: Operand,
    },
    VecSet {
        elem_ty: IrType,
        vec: Operand,
        index: Operand,
        value: Operand,
    },
    MakeStruct {
        dst: TempId,
        struct_id: StructId,
        fields: Vec<Operand>,
    },
    StructGet {
        dst: TempId,
        ty: IrType,
        base: Operand,
        field: FieldRef,
    },
    StructSet {
        base: Operand,
        field: FieldRef,
        value: Operand,
        ty: IrType,
    },
    MakeClosure {
        dst: TempId,
        function: FunctionId,
    },
    CallDirect {
        dst: Option<TempId>,
        ret_ty: IrType,
        function: FunctionId,
        args: Vec<Operand>,
    },
    CallIndirect {
        dst: Option<TempId>,
        ret_ty: IrType,
        callee: Operand,
        args: Vec<Operand>,
    },
    CallBuiltin {
        dst: Option<TempId>,
        ret_ty: IrType,
        builtin: BuiltinCall,
        args: Vec<Operand>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct BranchTerminator {
    pub cond: Operand,
    pub then_block: BlockId,
    pub else_block: BlockId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    Jump(BlockId),
    Branch(BranchTerminator),
    Return(Option<Operand>),
    Panic { message: String },
    Unreachable,
}
