use std::fmt;
use std::rc::Rc;

use crate::ir::{BranchTerminator, FunctionId, IrProgram, IrType, Terminator};
use skepart::{NoopHost, RtError, RtErrorKind, RtHost, RtStructLayout, RtValue};

mod exec;
mod frame;

use frame::Frame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrInterpError {
    MissingMain,
    MissingFunction(FunctionId),
    MissingBlock(crate::ir::BlockId),
    UnsupportedBuiltin(String),
    TypeMismatch(&'static str),
    DivisionByZero,
    InvalidOperand(&'static str),
    InvalidField(String),
    IndexOutOfBounds,
}

impl fmt::Display for IrInterpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMain => write!(f, "IR program has no main function"),
            Self::MissingFunction(id) => write!(f, "IR program is missing function {:?}", id),
            Self::MissingBlock(id) => write!(f, "IR function is missing block {:?}", id),
            Self::UnsupportedBuiltin(name) => {
                write!(f, "IR interpreter does not support builtin `{name}`")
            }
            Self::TypeMismatch(msg) => write!(f, "IR type mismatch: {msg}"),
            Self::DivisionByZero => write!(f, "IR division by zero"),
            Self::InvalidOperand(msg) => write!(f, "IR invalid operand: {msg}"),
            Self::InvalidField(name) => write!(f, "IR invalid field `{name}`"),
            Self::IndexOutOfBounds => write!(f, "IR index out of bounds"),
        }
    }
}

impl IrInterpError {
    fn from_runtime(err: RtError) -> Self {
        match err.kind {
            RtErrorKind::DivisionByZero => Self::DivisionByZero,
            RtErrorKind::IndexOutOfBounds => Self::IndexOutOfBounds,
            RtErrorKind::TypeMismatch => {
                Self::TypeMismatch(Box::leak(err.message.into_boxed_str()))
            }
            RtErrorKind::MissingField => Self::InvalidField(err.message),
            RtErrorKind::InvalidArgument => {
                Self::InvalidOperand(Box::leak(err.message.into_boxed_str()))
            }
            RtErrorKind::Io | RtErrorKind::Process => {
                Self::InvalidOperand(Box::leak(err.message.into_boxed_str()))
            }
            RtErrorKind::UnsupportedBuiltin => Self::UnsupportedBuiltin(err.message),
        }
    }
}

pub struct IrInterpreter<'a> {
    program: &'a IrProgram,
    globals: Vec<RtValue>,
    struct_layouts: Vec<Rc<RtStructLayout>>,
    host: Box<dyn RtHost>,
}

impl<'a> IrInterpreter<'a> {
    pub fn new(program: &'a IrProgram) -> Self {
        Self::with_host(program, Box::new(NoopHost::default()))
    }

    pub fn with_host(program: &'a IrProgram, host: Box<dyn RtHost>) -> Self {
        Self {
            program,
            globals: vec![RtValue::Unit; program.globals.len()],
            struct_layouts: program
                .structs
                .iter()
                .map(|strukt| {
                    Rc::new(RtStructLayout {
                        name: strukt.name.clone(),
                        field_names: strukt
                            .fields
                            .iter()
                            .map(|field| field.name.clone())
                            .collect(),
                        field_types: strukt
                            .fields
                            .iter()
                            .map(|field| Some(runtime_type_name(&field.ty)))
                            .collect(),
                    })
                })
                .collect(),
            host,
        }
    }

    pub fn run_main(mut self) -> Result<RtValue, IrInterpError> {
        if let Some(init) = &self.program.module_init {
            let _ = self.run_function(init.function, Vec::new())?;
        }
        let main = self
            .program
            .functions
            .iter()
            .find(|func| func.name == "main")
            .ok_or(IrInterpError::MissingMain)?;
        self.run_function(main.id, Vec::new())
    }

    fn run_function(
        &mut self,
        function_id: FunctionId,
        args: Vec<RtValue>,
    ) -> Result<RtValue, IrInterpError> {
        let func = self
            .program
            .functions
            .iter()
            .find(|func| func.id == function_id)
            .ok_or(IrInterpError::MissingFunction(function_id))?;
        if func.params.len() != args.len() {
            return Err(IrInterpError::InvalidOperand("call arity mismatch"));
        }
        let mut frame = Frame::new(func, args);
        let mut current_block = func.entry;

        loop {
            let block = func
                .blocks
                .iter()
                .find(|block| block.id == current_block)
                .ok_or(IrInterpError::MissingBlock(current_block))?;

            for instr in &block.instrs {
                self.exec_instr(func, &mut frame, instr)?;
            }

            match &block.terminator {
                Terminator::Jump(next) => current_block = *next,
                Terminator::Branch(branch) => current_block = self.eval_branch(&frame, branch)?,
                Terminator::Return(value) => {
                    return Ok(match value {
                        Some(operand) => frame.read_operand(operand, &self.globals)?,
                        None => RtValue::Unit,
                    });
                }
                Terminator::Panic { message } => {
                    return Err(IrInterpError::InvalidOperand(Box::leak(
                        message.clone().into_boxed_str(),
                    )));
                }
                Terminator::Unreachable => return Ok(RtValue::Unit),
            }
        }
    }

    fn eval_branch(
        &self,
        frame: &Frame,
        branch: &BranchTerminator,
    ) -> Result<crate::ir::BlockId, IrInterpError> {
        match frame.read_operand(&branch.cond, &self.globals)? {
            RtValue::Bool(true) => Ok(branch.then_block),
            RtValue::Bool(false) => Ok(branch.else_block),
            _ => Err(IrInterpError::TypeMismatch("branch condition must be bool")),
        }
    }
}

fn runtime_type_name(ty: &IrType) -> &'static str {
    match ty {
        IrType::Int => "Int",
        IrType::Float => "Float",
        IrType::Bool => "Bool",
        IrType::String => "String",
        IrType::Bytes => "Bytes",
        IrType::Array { .. } => "Array",
        IrType::Vec { .. } => "Vec",
        IrType::Fn { .. } => "Function",
        IrType::Named(_) => "Struct",
        IrType::Opaque(_) => "Handle",
        IrType::Void => "Void",
        IrType::Unknown => "Unknown",
    }
}
