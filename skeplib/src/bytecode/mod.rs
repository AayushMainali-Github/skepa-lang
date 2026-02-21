use std::collections::HashMap;

mod codec;
mod disasm;
mod lowering;

pub use lowering::{compile_project_entry, compile_source};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
    Function(String),
    Struct {
        name: String,
        fields: Vec<(String, Value)>,
    },
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    LoadConst(Value),
    LoadLocal(usize),
    StoreLocal(usize),
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
    Call {
        name: String,
        argc: usize,
    },
    CallValue {
        argc: usize,
    },
    CallMethod {
        name: String,
        argc: usize,
    },
    CallBuiltin {
        package: String,
        name: String,
        argc: usize,
    },
    MakeArray(usize),
    MakeArrayRepeat(usize),
    ArrayGet,
    ArraySet,
    ArraySetChain(usize),
    ArrayLen,
    MakeStruct {
        name: String,
        fields: Vec<String>,
    },
    StructGet(String),
    StructSetPath(Vec<String>),
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
}
