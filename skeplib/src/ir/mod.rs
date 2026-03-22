mod block;
mod builder;
mod instr;
mod interp;
pub mod lowering;
mod native_aggregates;
mod native_calls;
mod native_strings;
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
pub use native_aggregates::{NativeAggregatePlan, NativeArrayPlan, NativeStructPlan};
pub use native_calls::NativeCallPlan;
pub use native_strings::{
    NativeStringBuiltinLowering, NativeStringPlan, NativeStringValue,
    collect_program_string_constants,
};
pub use nativeability::{NativeLocalKind, NativeabilityAnalysis};
pub use pretty::PrettyIr;
pub use program::{
    IrFunction, IrGlobal, IrLocal, IrModuleInit, IrParam, IrProgram, IrStruct, IrTemp, StructField,
};
pub use skepart::RtValue as IrValue;
pub use types::IrType;
pub use value::{ConstValue, Operand};
pub use verify::{IrVerifier, IrVerifyError};
