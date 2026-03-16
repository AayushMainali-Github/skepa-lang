mod block;
mod builder;
mod instr;
mod pretty;
mod program;
mod types;
mod value;
mod verify;

pub use block::{BasicBlock, BlockId, FunctionId, GlobalId, LocalId, ParamId, StructId, TempId};
pub use builder::IrBuilder;
pub use instr::{
    BinaryOp, BranchTerminator, BuiltinCall, CmpOp, FieldRef, Instr, LogicOp, Terminator, UnaryOp,
};
pub use pretty::PrettyIr;
pub use program::{IrFunction, IrGlobal, IrModuleInit, IrProgram, IrStruct, StructField};
pub use types::IrType;
pub use value::{ConstValue, Operand};
pub use verify::{IrVerifier, IrVerifyError};
