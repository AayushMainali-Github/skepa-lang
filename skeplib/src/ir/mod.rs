mod block;
mod builder;
mod instr;
mod interp;
pub mod lowering;
mod nativeability;
pub mod opt;
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
pub use interp::{IrInterpError, IrInterpreter};
pub use nativeability::{NativeLocalKind, NativeabilityAnalysis};
pub use pretty::PrettyIr;
pub use program::{
    IrFunction, IrGlobal, IrLocal, IrModuleInit, IrParam, IrProgram, IrStruct, IrTemp, StructField,
};
pub use skepart::RtValue as IrValue;
pub use types::IrType;
pub use value::{ConstValue, Operand};
pub use verify::{IrVerifier, IrVerifyError};
