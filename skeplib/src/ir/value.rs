use crate::ir::{GlobalId, LocalId, TempId};

#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Const(ConstValue),
    Temp(TempId),
    Local(LocalId),
    Global(GlobalId),
}
