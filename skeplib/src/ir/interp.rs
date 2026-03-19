use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::ir::{
    BinaryOp, BranchTerminator, CmpOp, ConstValue, FunctionId, Instr, IrFunction, IrProgram,
    Operand, Terminator, UnaryOp,
};
use skepart::{
    NoopHost, RtArray, RtError, RtErrorKind, RtFunctionRef, RtHost, RtString, RtStruct,
    RtStructLayout, RtValue, RtVec, builtins,
};

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

    fn exec_instr(
        &mut self,
        func: &IrFunction,
        frame: &mut Frame,
        instr: &Instr,
    ) -> Result<(), IrInterpError> {
        match instr {
            Instr::Const { dst, value, .. } => {
                frame.temps.insert(*dst, Self::const_to_value(value));
            }
            Instr::Copy { dst, src, .. } => {
                let value = frame.read_operand(src, &self.globals)?;
                frame.temps.insert(*dst, value);
            }
            Instr::Unary {
                dst, op, operand, ..
            } => {
                let value = frame.read_operand(operand, &self.globals)?;
                let out = match (op, value) {
                    (UnaryOp::Neg, RtValue::Int(v)) => RtValue::Int(-v),
                    (UnaryOp::Neg, RtValue::Float(v)) => RtValue::Float(-v),
                    (UnaryOp::Not, RtValue::Bool(v)) => RtValue::Bool(!v),
                    _ => return Err(IrInterpError::TypeMismatch("bad unary operand")),
                };
                frame.temps.insert(*dst, out);
            }
            Instr::Binary {
                dst,
                op,
                left,
                right,
                ..
            } => {
                let left = frame.read_operand(left, &self.globals)?;
                let right = frame.read_operand(right, &self.globals)?;
                let out = self.eval_binary(*op, left, right)?;
                frame.temps.insert(*dst, out);
            }
            Instr::Compare {
                dst,
                op,
                left,
                right,
            } => {
                let left = frame.read_operand(left, &self.globals)?;
                let right = frame.read_operand(right, &self.globals)?;
                frame
                    .temps
                    .insert(*dst, RtValue::Bool(self.eval_compare(*op, left, right)?));
            }
            Instr::Logic {
                dst,
                op,
                left,
                right,
            } => {
                let left = frame.read_operand(left, &self.globals)?;
                let right = frame.read_operand(right, &self.globals)?;
                let out = match (op, left, right) {
                    (crate::ir::LogicOp::And, RtValue::Bool(a), RtValue::Bool(b)) => {
                        RtValue::Bool(a && b)
                    }
                    (crate::ir::LogicOp::Or, RtValue::Bool(a), RtValue::Bool(b)) => {
                        RtValue::Bool(a || b)
                    }
                    _ => return Err(IrInterpError::TypeMismatch("bad logical operands")),
                };
                frame.temps.insert(*dst, out);
            }
            Instr::LoadGlobal { dst, global, .. } => {
                let value = self
                    .globals
                    .get(global.0)
                    .cloned()
                    .ok_or(IrInterpError::InvalidOperand("global load out of range"))?;
                frame.temps.insert(*dst, value);
            }
            Instr::StoreGlobal { global, value, .. } => {
                let value = frame.read_operand(value, &self.globals)?;
                let slot = self
                    .globals
                    .get_mut(global.0)
                    .ok_or(IrInterpError::InvalidOperand("global store out of range"))?;
                *slot = value;
            }
            Instr::LoadLocal { dst, local, .. } => {
                let value = frame
                    .locals
                    .get(&local.0)
                    .cloned()
                    .ok_or(IrInterpError::InvalidOperand("local load out of range"))?;
                frame.temps.insert(*dst, value);
            }
            Instr::StoreLocal { local, value, .. } => {
                let value = frame.read_operand(value, &self.globals)?;
                frame.locals.insert(local.0, value);
            }
            Instr::MakeArray { dst, items, .. } => {
                let values = items
                    .iter()
                    .map(|item| frame.read_operand(item, &self.globals))
                    .collect::<Result<Vec<_>, _>>()?;
                frame
                    .temps
                    .insert(*dst, RtValue::Array(RtArray::new(values)));
            }
            Instr::MakeArrayRepeat {
                dst, value, size, ..
            } => {
                let value = frame.read_operand(value, &self.globals)?;
                frame
                    .temps
                    .insert(*dst, RtValue::Array(RtArray::repeat(value, *size)));
            }
            Instr::ArrayGet {
                dst, array, index, ..
            } => {
                let array = frame.read_operand(array, &self.globals)?;
                let index = self.read_index(frame, index)?;
                let value = match array {
                    RtValue::Array(items) => {
                        items.get(index).map_err(IrInterpError::from_runtime)?
                    }
                    _ => return Err(IrInterpError::TypeMismatch("array get on non-array")),
                };
                frame.temps.insert(*dst, value);
            }
            Instr::ArraySet {
                array,
                index,
                value,
                ..
            } => {
                let index = self.read_index(frame, index)?;
                let value = frame.read_operand(value, &self.globals)?;
                match array {
                    Operand::Local(local) => {
                        let slot = frame
                            .locals
                            .get_mut(&local.0)
                            .ok_or(IrInterpError::InvalidOperand("array local missing"))?;
                        match slot {
                            RtValue::Array(items) => {
                                items
                                    .set(index, value)
                                    .map_err(IrInterpError::from_runtime)?;
                            }
                            _ => {
                                return Err(IrInterpError::TypeMismatch("array set on non-array"));
                            }
                        }
                    }
                    Operand::Global(global) => {
                        let slot = self
                            .globals
                            .get_mut(global.0)
                            .ok_or(IrInterpError::InvalidOperand("array global missing"))?;
                        match slot {
                            RtValue::Array(items) => {
                                items
                                    .set(index, value)
                                    .map_err(IrInterpError::from_runtime)?;
                            }
                            _ => {
                                return Err(IrInterpError::TypeMismatch("array set on non-array"));
                            }
                        }
                    }
                    _ => return Err(IrInterpError::InvalidOperand("array set needs lvalue")),
                }
            }
            Instr::VecNew { dst, .. } => {
                frame.temps.insert(*dst, RtValue::Vec(RtVec::new()));
            }
            Instr::VecLen { dst, vec } => {
                let vec = frame.read_operand(vec, &self.globals)?;
                let len = match vec {
                    RtValue::Vec(items) => items.len() as i64,
                    _ => return Err(IrInterpError::TypeMismatch("vec.len on non-vec")),
                };
                frame.temps.insert(*dst, RtValue::Int(len));
            }
            Instr::VecPush { vec, value } => {
                let vec = frame.read_operand(vec, &self.globals)?;
                let value = frame.read_operand(value, &self.globals)?;
                match vec {
                    RtValue::Vec(items) => items.push(value),
                    _ => return Err(IrInterpError::TypeMismatch("vec.push on non-vec")),
                }
            }
            Instr::VecGet {
                dst, vec, index, ..
            } => {
                let vec = frame.read_operand(vec, &self.globals)?;
                let index = self.read_index(frame, index)?;
                let value = match vec {
                    RtValue::Vec(items) => items.get(index).map_err(IrInterpError::from_runtime)?,
                    _ => return Err(IrInterpError::TypeMismatch("vec.get on non-vec")),
                };
                frame.temps.insert(*dst, value);
            }
            Instr::VecSet {
                vec, index, value, ..
            } => {
                let vec = frame.read_operand(vec, &self.globals)?;
                let index = self.read_index(frame, index)?;
                let value = frame.read_operand(value, &self.globals)?;
                match vec {
                    RtValue::Vec(items) => {
                        items
                            .set(index, value)
                            .map_err(IrInterpError::from_runtime)?;
                    }
                    _ => return Err(IrInterpError::TypeMismatch("vec.set on non-vec")),
                }
            }
            Instr::VecDelete {
                dst, vec, index, ..
            } => {
                let vec = frame.read_operand(vec, &self.globals)?;
                let index = self.read_index(frame, index)?;
                let value = match vec {
                    RtValue::Vec(items) => {
                        items.delete(index).map_err(IrInterpError::from_runtime)?
                    }
                    _ => return Err(IrInterpError::TypeMismatch("vec.delete on non-vec")),
                };
                frame.temps.insert(*dst, value);
            }
            Instr::MakeStruct {
                dst,
                struct_id,
                fields,
            } => {
                let fields = fields
                    .iter()
                    .map(|field| frame.read_operand(field, &self.globals))
                    .collect::<Result<Vec<_>, _>>()?;
                let layout = self
                    .struct_layouts
                    .get(struct_id.0)
                    .cloned()
                    .ok_or_else(|| {
                        IrInterpError::InvalidField(format!("unknown struct {:?}", struct_id))
                    })?;
                frame
                    .temps
                    .insert(*dst, RtValue::Struct(RtStruct::new(layout, fields)));
            }
            Instr::StructGet {
                dst, base, field, ..
            } => {
                let base = frame.read_operand(base, &self.globals)?;
                let value = match base {
                    RtValue::Struct(value) => value
                        .get_field(field.index)
                        .map_err(IrInterpError::from_runtime)?,
                    _ => return Err(IrInterpError::TypeMismatch("struct get on non-struct")),
                };
                frame.temps.insert(*dst, value);
            }
            Instr::StructSet {
                base, field, value, ..
            } => {
                let value = frame.read_operand(value, &self.globals)?;
                match base {
                    Operand::Local(local) => {
                        let slot = frame
                            .locals
                            .get_mut(&local.0)
                            .ok_or(IrInterpError::InvalidOperand("struct local missing"))?;
                        match slot {
                            RtValue::Struct(strukt) => {
                                strukt
                                    .set_field(field.index, value)
                                    .map_err(IrInterpError::from_runtime)?;
                            }
                            _ => {
                                return Err(IrInterpError::TypeMismatch(
                                    "struct set on non-struct",
                                ));
                            }
                        }
                    }
                    Operand::Global(global) => {
                        let slot = self
                            .globals
                            .get_mut(global.0)
                            .ok_or(IrInterpError::InvalidOperand("struct global missing"))?;
                        match slot {
                            RtValue::Struct(strukt) => {
                                strukt
                                    .set_field(field.index, value)
                                    .map_err(IrInterpError::from_runtime)?;
                            }
                            _ => {
                                return Err(IrInterpError::TypeMismatch(
                                    "struct set on non-struct",
                                ));
                            }
                        }
                    }
                    _ => return Err(IrInterpError::InvalidOperand("struct set needs lvalue")),
                }
            }
            Instr::MakeClosure { dst, function } => {
                frame
                    .temps
                    .insert(*dst, RtValue::Function(RtFunctionRef(function.0 as u32)));
            }
            Instr::CallDirect {
                dst,
                function,
                args,
                ..
            } => {
                let args = args
                    .iter()
                    .map(|arg| frame.read_operand(arg, &self.globals))
                    .collect::<Result<Vec<_>, _>>()?;
                let value = self.run_function(*function, args)?;
                if let Some(dst) = dst {
                    frame.temps.insert(*dst, value);
                }
            }
            Instr::CallIndirect {
                dst, callee, args, ..
            } => {
                let callee = frame.read_operand(callee, &self.globals)?;
                let function = match callee {
                    RtValue::Function(function) => FunctionId(function.0 as usize),
                    _ => return Err(IrInterpError::TypeMismatch("indirect call on non-closure")),
                };
                let args = args
                    .iter()
                    .map(|arg| frame.read_operand(arg, &self.globals))
                    .collect::<Result<Vec<_>, _>>()?;
                let value = self.run_function(function, args)?;
                if let Some(dst) = dst {
                    frame.temps.insert(*dst, value);
                }
            }
            Instr::CallBuiltin { builtin, .. } => {
                let args = builtin_args(frame, &self.globals, instr)?;
                let value = self.eval_builtin(builtin, &args)?;
                if let Instr::CallBuiltin { dst, .. } = instr
                    && let Some(dst) = dst
                {
                    frame.temps.insert(*dst, value);
                }
            }
        }
        let _ = func;
        Ok(())
    }

    fn eval_builtin(
        &mut self,
        builtin: &crate::ir::BuiltinCall,
        args: &[RtValue],
    ) -> Result<RtValue, IrInterpError> {
        builtins::call_with_host(self.host.as_mut(), &builtin.package, &builtin.name, args)
            .map_err(IrInterpError::from_runtime)
    }

    fn read_index(&self, frame: &Frame, operand: &Operand) -> Result<usize, IrInterpError> {
        match frame.read_operand(operand, &self.globals)? {
            RtValue::Int(idx) => usize::try_from(idx).map_err(|_| IrInterpError::IndexOutOfBounds),
            _ => Err(IrInterpError::TypeMismatch("index must be int")),
        }
    }

    fn eval_binary(
        &self,
        op: BinaryOp,
        left: RtValue,
        right: RtValue,
    ) -> Result<RtValue, IrInterpError> {
        match (op, left, right) {
            (BinaryOp::Add, RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Int(a + b)),
            (BinaryOp::Sub, RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Int(a - b)),
            (BinaryOp::Mul, RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Int(a * b)),
            (BinaryOp::Div, RtValue::Int(_), RtValue::Int(0))
            | (BinaryOp::Mod, RtValue::Int(_), RtValue::Int(0)) => {
                Err(IrInterpError::DivisionByZero)
            }
            (BinaryOp::Div, RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Int(a / b)),
            (BinaryOp::Mod, RtValue::Int(a), RtValue::Int(b)) => Ok(RtValue::Int(a % b)),
            (BinaryOp::Add, RtValue::Float(a), RtValue::Float(b)) => Ok(RtValue::Float(a + b)),
            (BinaryOp::Sub, RtValue::Float(a), RtValue::Float(b)) => Ok(RtValue::Float(a - b)),
            (BinaryOp::Mul, RtValue::Float(a), RtValue::Float(b)) => Ok(RtValue::Float(a * b)),
            (BinaryOp::Div, RtValue::Float(a), RtValue::Float(b)) => Ok(RtValue::Float(a / b)),
            (BinaryOp::Add, RtValue::String(a), RtValue::String(b)) => Ok(RtValue::String(
                RtString::from(format!("{}{}", a.as_str(), b.as_str())),
            )),
            _ => Err(IrInterpError::TypeMismatch("bad binary operands")),
        }
    }

    fn eval_compare(
        &self,
        op: CmpOp,
        left: RtValue,
        right: RtValue,
    ) -> Result<bool, IrInterpError> {
        match (left, right) {
            (RtValue::Int(a), RtValue::Int(b)) => Ok(match op {
                CmpOp::Eq => a == b,
                CmpOp::Ne => a != b,
                CmpOp::Lt => a < b,
                CmpOp::Le => a <= b,
                CmpOp::Gt => a > b,
                CmpOp::Ge => a >= b,
            }),
            (RtValue::Float(a), RtValue::Float(b)) => Ok(match op {
                CmpOp::Eq => a == b,
                CmpOp::Ne => a != b,
                CmpOp::Lt => a < b,
                CmpOp::Le => a <= b,
                CmpOp::Gt => a > b,
                CmpOp::Ge => a >= b,
            }),
            (RtValue::Bool(a), RtValue::Bool(b)) => Ok(match op {
                CmpOp::Eq => a == b,
                CmpOp::Ne => a != b,
                _ => return Err(IrInterpError::TypeMismatch("unsupported bool comparison")),
            }),
            (RtValue::String(a), RtValue::String(b)) => Ok(match op {
                CmpOp::Eq => a.as_str() == b.as_str(),
                CmpOp::Ne => a.as_str() != b.as_str(),
                _ => return Err(IrInterpError::TypeMismatch("unsupported string comparison")),
            }),
            _ => Err(IrInterpError::TypeMismatch("bad compare operands")),
        }
    }

    fn const_to_value(value: &ConstValue) -> RtValue {
        match value {
            ConstValue::Int(v) => RtValue::Int(*v),
            ConstValue::Float(v) => RtValue::Float(*v),
            ConstValue::Bool(v) => RtValue::Bool(*v),
            ConstValue::String(v) => RtValue::String(RtString::from(v.clone())),
            ConstValue::Unit => RtValue::Unit,
        }
    }
}

fn builtin_args(
    frame: &Frame,
    globals: &[RtValue],
    instr: &Instr,
) -> Result<Vec<RtValue>, IrInterpError> {
    match instr {
        Instr::CallBuiltin { args, .. } => args
            .iter()
            .map(|arg| frame.read_operand(arg, globals))
            .collect::<Result<Vec<_>, _>>(),
        _ => Err(IrInterpError::InvalidOperand(
            "builtin args on non-builtin instr",
        )),
    }
}

struct Frame {
    params: HashMap<usize, RtValue>,
    locals: HashMap<usize, RtValue>,
    temps: HashMap<crate::ir::TempId, RtValue>,
}

impl Frame {
    fn new(func: &IrFunction, args: Vec<RtValue>) -> Self {
        let mut params = HashMap::new();
        let mut locals = HashMap::new();
        for (param, value) in func.params.iter().zip(args) {
            params.insert(param.id.0, value.clone());
            if let Some(local) = func.locals.iter().find(|local| local.name == param.name) {
                locals.insert(local.id.0, value);
            }
        }
        Self {
            params,
            locals,
            temps: HashMap::new(),
        }
    }

    fn read_operand(
        &self,
        operand: &Operand,
        globals: &[RtValue],
    ) -> Result<RtValue, IrInterpError> {
        match operand {
            Operand::Const(value) => Ok(IrInterpreter::const_to_value(value)),
            Operand::Temp(id) => self
                .temps
                .get(id)
                .cloned()
                .ok_or(IrInterpError::InvalidOperand("temp missing")),
            Operand::Local(id) => self
                .locals
                .get(&id.0)
                .cloned()
                .or_else(|| self.params.get(&id.0).cloned())
                .ok_or(IrInterpError::InvalidOperand("local missing")),
            Operand::Global(id) => globals
                .get(id.0)
                .cloned()
                .ok_or(IrInterpError::InvalidOperand("global missing")),
        }
    }
}
