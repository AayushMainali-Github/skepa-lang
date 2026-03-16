use std::fmt::{self, Display, Formatter};

use crate::ir::{BasicBlock, IrFunction, IrProgram, Terminator};

pub struct PrettyIr<'a> {
    pub program: &'a IrProgram,
}

impl<'a> PrettyIr<'a> {
    pub fn new(program: &'a IrProgram) -> Self {
        Self { program }
    }
}

impl Display for PrettyIr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for function in &self.program.functions {
            fmt_function(f, function)?;
        }
        Ok(())
    }
}

fn fmt_function(f: &mut Formatter<'_>, function: &IrFunction) -> fmt::Result {
    writeln!(f, "fn {} -> {:?} {{", function.name, function.ret_ty)?;
    for block in &function.blocks {
        fmt_block(f, block)?;
    }
    writeln!(f, "}}")
}

fn fmt_block(f: &mut Formatter<'_>, block: &BasicBlock) -> fmt::Result {
    writeln!(f, "  {}:", block.name)?;
    for instr in &block.instrs {
        writeln!(f, "    {:?}", instr)?;
    }
    match &block.terminator {
        Terminator::Unreachable => writeln!(f, "    unreachable"),
        other => writeln!(f, "    {:?}", other),
    }
}
