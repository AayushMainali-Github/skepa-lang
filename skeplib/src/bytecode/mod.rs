use std::collections::HashMap;
use std::rc::Rc;

mod codec;
mod disasm;
mod lowering;

pub use lowering::{compile_project_entry, compile_project_graph, compile_source};

#[derive(Debug, Clone, PartialEq)]
pub struct StructShape {
    pub name: String,
    pub field_names: Rc<[String]>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(Rc<str>),
    Array(Rc<[Value]>),
    VecHandle(u64),
    Function(Rc<str>),
    FunctionIdx(usize),
    Struct {
        shape: Rc<StructShape>,
        fields: Rc<[Value]>,
    },
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntLocalConstOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    LoadConst(Value),
    LoadLocal(usize),
    StoreLocal(usize),
    AddLocalToLocal {
        dst: usize,
        src: usize,
    },
    AddConstToLocal {
        slot: usize,
        rhs: i64,
    },
    IntLocalLocalOp {
        lhs: usize,
        rhs: usize,
        op: IntLocalConstOp,
    },
    IntLocalConstOp {
        slot: usize,
        op: IntLocalConstOp,
        rhs: i64,
    },
    IntLocalConstOpToLocal {
        src: usize,
        dst: usize,
        op: IntLocalConstOp,
        rhs: i64,
    },
    IntStackOpToLocal {
        slot: usize,
        op: IntLocalConstOp,
    },
    IntStackConstOp {
        op: IntLocalConstOp,
        rhs: i64,
    },
    IntLocalLocalOpToLocal {
        lhs: usize,
        rhs: usize,
        dst: usize,
        op: IntLocalConstOp,
    },
    LoadGlobal(usize),
    StoreGlobal(usize),
    Pop,
    NegInt,
    NotBool,
    Add,
    SubInt,
    MulInt,
    DivInt,
    ModInt,
    Eq,
    Neq,
    LtInt,
    LteInt,
    GtInt,
    GteInt,
    AndBool,
    OrBool,
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    JumpIfLocalLtConst {
        slot: usize,
        rhs: i64,
        target: usize,
    },
    Call {
        name: String,
        argc: usize,
    },
    CallIdx {
        idx: usize,
        argc: usize,
    },
    CallIdxAddConst(i64),
    CallIdxStructFieldAdd(usize),
    CallValue {
        argc: usize,
    },
    CallMethod {
        name: String,
        argc: usize,
    },
    CallMethodId {
        id: usize,
        argc: usize,
    },
    CallBuiltin {
        package: String,
        name: String,
        argc: usize,
    },
    CallBuiltinId {
        id: u16,
        argc: usize,
    },
    StrLen,
    StrLenLocal(usize),
    StrIndexOfConst(Rc<str>),
    StrIndexOfLocalConst {
        slot: usize,
        needle: Rc<str>,
    },
    StrSliceConst {
        start: i64,
        end: i64,
    },
    StrSliceLocalConst {
        slot: usize,
        start: i64,
        end: i64,
    },
    StrContainsConst(Rc<str>),
    StrContainsLocalConst {
        slot: usize,
        needle: Rc<str>,
    },
    MakeArray(usize),
    MakeArrayRepeat(usize),
    ArrayGet,
    ArrayGetLocal(usize),
    ArraySet,
    ArraySetLocal(usize),
    ArrayIncLocal(usize),
    ArraySetChain(usize),
    ArrayLen,
    MakeStruct {
        name: String,
        fields: Vec<String>,
    },
    MakeStructId {
        id: usize,
    },
    StructGet(String),
    StructGetLocalSlot {
        slot: usize,
        field_slot: usize,
    },
    StructGetLocalSlotAddToLocal {
        struct_slot: usize,
        field_slot: usize,
        dst: usize,
    },
    StructGetSlot(usize),
    StructSetPath(Vec<String>),
    StructSetPathSlots(Vec<usize>),
    Return,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FunctionChunk {
    pub name: String,
    pub code: Vec<Instr>,
    pub locals_count: usize,
    pub param_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BytecodeModule {
    pub functions: HashMap<String, FunctionChunk>,
    pub method_names: Vec<String>,
    pub struct_shapes: Vec<StructShape>,
}
