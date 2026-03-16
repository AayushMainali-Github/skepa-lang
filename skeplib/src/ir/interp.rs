use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::ir::{
    BinaryOp, BranchTerminator, CmpOp, ConstValue, FunctionId, Instr, IrFunction, IrProgram,
    Operand, Terminator, UnaryOp,
};

#[derive(Debug, Clone, PartialEq)]
pub enum IrValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Vec<IrValue>),
    Vec(Rc<RefCell<Vec<IrValue>>>),
    Struct {
        struct_id: crate::ir::StructId,
        fields: Vec<IrValue>,
    },
    Closure(FunctionId),
    Unit,
}

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

pub struct IrInterpreter<'a> {
    program: &'a IrProgram,
    globals: Vec<IrValue>,
}

impl<'a> IrInterpreter<'a> {
    pub fn new(program: &'a IrProgram) -> Self {
        Self {
            program,
            globals: vec![IrValue::Unit; program.globals.len()],
        }
    }

    pub fn run_main(mut self) -> Result<IrValue, IrInterpError> {
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
        args: Vec<IrValue>,
    ) -> Result<IrValue, IrInterpError> {
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
                Terminator::Branch(branch) => {
                    current_block = self.eval_branch(&frame, branch)?;
                }
                Terminator::Return(value) => {
                    return Ok(match value {
                        Some(operand) => frame.read_operand(operand, &self.globals)?,
                        None => IrValue::Unit,
                    });
                }
                Terminator::Panic { message } => {
                    return Err(IrInterpError::InvalidOperand(Box::leak(
                        message.clone().into_boxed_str(),
                    )));
                }
                Terminator::Unreachable => return Ok(IrValue::Unit),
            }
        }
    }

    fn eval_branch(
        &self,
        frame: &Frame,
        branch: &BranchTerminator,
    ) -> Result<crate::ir::BlockId, IrInterpError> {
        match frame.read_operand(&branch.cond, &self.globals)? {
            IrValue::Bool(true) => Ok(branch.then_block),
            IrValue::Bool(false) => Ok(branch.else_block),
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
                    (UnaryOp::Neg, IrValue::Int(v)) => IrValue::Int(-v),
                    (UnaryOp::Neg, IrValue::Float(v)) => IrValue::Float(-v),
                    (UnaryOp::Not, IrValue::Bool(v)) => IrValue::Bool(!v),
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
                    .insert(*dst, IrValue::Bool(self.eval_compare(*op, left, right)?));
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
                    (crate::ir::LogicOp::And, IrValue::Bool(a), IrValue::Bool(b)) => {
                        IrValue::Bool(a && b)
                    }
                    (crate::ir::LogicOp::Or, IrValue::Bool(a), IrValue::Bool(b)) => {
                        IrValue::Bool(a || b)
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
                frame.temps.insert(*dst, IrValue::Array(values));
            }
            Instr::MakeArrayRepeat {
                dst, value, size, ..
            } => {
                let value = frame.read_operand(value, &self.globals)?;
                frame.temps.insert(*dst, IrValue::Array(vec![value; *size]));
            }
            Instr::ArrayGet {
                dst, array, index, ..
            } => {
                let array = frame.read_operand(array, &self.globals)?;
                let index = self.read_index(frame, index)?;
                let value = match array {
                    IrValue::Array(items) => items
                        .get(index)
                        .cloned()
                        .ok_or(IrInterpError::IndexOutOfBounds)?,
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
                            IrValue::Array(items) => {
                                let item = items
                                    .get_mut(index)
                                    .ok_or(IrInterpError::IndexOutOfBounds)?;
                                *item = value;
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
                            IrValue::Array(items) => {
                                let item = items
                                    .get_mut(index)
                                    .ok_or(IrInterpError::IndexOutOfBounds)?;
                                *item = value;
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
                frame
                    .temps
                    .insert(*dst, IrValue::Vec(Rc::new(RefCell::new(Vec::new()))));
            }
            Instr::VecLen { dst, vec } => {
                let vec = frame.read_operand(vec, &self.globals)?;
                let len = match vec {
                    IrValue::Vec(items) => items.borrow().len() as i64,
                    _ => return Err(IrInterpError::TypeMismatch("vec.len on non-vec")),
                };
                frame.temps.insert(*dst, IrValue::Int(len));
            }
            Instr::VecPush { vec, value } => {
                let vec = frame.read_operand(vec, &self.globals)?;
                let value = frame.read_operand(value, &self.globals)?;
                match vec {
                    IrValue::Vec(items) => items.borrow_mut().push(value),
                    _ => return Err(IrInterpError::TypeMismatch("vec.push on non-vec")),
                }
            }
            Instr::VecGet {
                dst, vec, index, ..
            } => {
                let vec = frame.read_operand(vec, &self.globals)?;
                let index = self.read_index(frame, index)?;
                let value = match vec {
                    IrValue::Vec(items) => items
                        .borrow()
                        .get(index)
                        .cloned()
                        .ok_or(IrInterpError::IndexOutOfBounds)?,
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
                    IrValue::Vec(items) => {
                        let mut items = items.borrow_mut();
                        let slot = items
                            .get_mut(index)
                            .ok_or(IrInterpError::IndexOutOfBounds)?;
                        *slot = value;
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
                    IrValue::Vec(items) => {
                        let mut items = items.borrow_mut();
                        if index >= items.len() {
                            return Err(IrInterpError::IndexOutOfBounds);
                        }
                        items.remove(index)
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
                frame.temps.insert(
                    *dst,
                    IrValue::Struct {
                        struct_id: *struct_id,
                        fields,
                    },
                );
            }
            Instr::StructGet {
                dst, base, field, ..
            } => {
                let base = frame.read_operand(base, &self.globals)?;
                let value = match base {
                    IrValue::Struct { fields, .. } => fields
                        .get(field.index)
                        .cloned()
                        .ok_or_else(|| IrInterpError::InvalidField(field.name.clone()))?,
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
                            IrValue::Struct { fields, .. } => {
                                let target = fields.get_mut(field.index).ok_or_else(|| {
                                    IrInterpError::InvalidField(field.name.clone())
                                })?;
                                *target = value;
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
                            IrValue::Struct { fields, .. } => {
                                let target = fields.get_mut(field.index).ok_or_else(|| {
                                    IrInterpError::InvalidField(field.name.clone())
                                })?;
                                *target = value;
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
                frame.temps.insert(*dst, IrValue::Closure(*function));
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
                let IrValue::Closure(function) = callee else {
                    return Err(IrInterpError::TypeMismatch("indirect call on non-closure"));
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
                return Err(IrInterpError::UnsupportedBuiltin(format!(
                    "{}.{}",
                    builtin.package, builtin.name
                )));
            }
        }
        let _ = func;
        Ok(())
    }

    fn read_index(&self, frame: &Frame, operand: &Operand) -> Result<usize, IrInterpError> {
        match frame.read_operand(operand, &self.globals)? {
            IrValue::Int(idx) => usize::try_from(idx).map_err(|_| IrInterpError::IndexOutOfBounds),
            _ => Err(IrInterpError::TypeMismatch("index must be int")),
        }
    }

    fn eval_binary(
        &self,
        op: BinaryOp,
        left: IrValue,
        right: IrValue,
    ) -> Result<IrValue, IrInterpError> {
        match (op, left, right) {
            (BinaryOp::Add, IrValue::Int(a), IrValue::Int(b)) => Ok(IrValue::Int(a + b)),
            (BinaryOp::Sub, IrValue::Int(a), IrValue::Int(b)) => Ok(IrValue::Int(a - b)),
            (BinaryOp::Mul, IrValue::Int(a), IrValue::Int(b)) => Ok(IrValue::Int(a * b)),
            (BinaryOp::Div, IrValue::Int(_), IrValue::Int(0))
            | (BinaryOp::Mod, IrValue::Int(_), IrValue::Int(0)) => {
                Err(IrInterpError::DivisionByZero)
            }
            (BinaryOp::Div, IrValue::Int(a), IrValue::Int(b)) => Ok(IrValue::Int(a / b)),
            (BinaryOp::Mod, IrValue::Int(a), IrValue::Int(b)) => Ok(IrValue::Int(a % b)),
            (BinaryOp::Add, IrValue::Float(a), IrValue::Float(b)) => Ok(IrValue::Float(a + b)),
            (BinaryOp::Sub, IrValue::Float(a), IrValue::Float(b)) => Ok(IrValue::Float(a - b)),
            (BinaryOp::Mul, IrValue::Float(a), IrValue::Float(b)) => Ok(IrValue::Float(a * b)),
            (BinaryOp::Div, IrValue::Float(a), IrValue::Float(b)) => Ok(IrValue::Float(a / b)),
            (BinaryOp::Add, IrValue::String(a), IrValue::String(b)) => Ok(IrValue::String(a + &b)),
            _ => Err(IrInterpError::TypeMismatch("bad binary operands")),
        }
    }

    fn eval_compare(
        &self,
        op: CmpOp,
        left: IrValue,
        right: IrValue,
    ) -> Result<bool, IrInterpError> {
        match (left, right) {
            (IrValue::Int(a), IrValue::Int(b)) => Ok(match op {
                CmpOp::Eq => a == b,
                CmpOp::Ne => a != b,
                CmpOp::Lt => a < b,
                CmpOp::Le => a <= b,
                CmpOp::Gt => a > b,
                CmpOp::Ge => a >= b,
            }),
            (IrValue::Bool(a), IrValue::Bool(b)) => Ok(match op {
                CmpOp::Eq => a == b,
                CmpOp::Ne => a != b,
                _ => return Err(IrInterpError::TypeMismatch("unsupported bool comparison")),
            }),
            (IrValue::String(a), IrValue::String(b)) => Ok(match op {
                CmpOp::Eq => a == b,
                CmpOp::Ne => a != b,
                _ => return Err(IrInterpError::TypeMismatch("unsupported string comparison")),
            }),
            _ => Err(IrInterpError::TypeMismatch("bad compare operands")),
        }
    }

    fn const_to_value(value: &ConstValue) -> IrValue {
        match value {
            ConstValue::Int(v) => IrValue::Int(*v),
            ConstValue::Float(v) => IrValue::Float(*v),
            ConstValue::Bool(v) => IrValue::Bool(*v),
            ConstValue::String(v) => IrValue::String(v.clone()),
            ConstValue::Unit => IrValue::Unit,
        }
    }
}

struct Frame {
    params: HashMap<usize, IrValue>,
    locals: HashMap<usize, IrValue>,
    temps: HashMap<crate::ir::TempId, IrValue>,
}

impl Frame {
    fn new(func: &IrFunction, args: Vec<IrValue>) -> Self {
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
        globals: &[IrValue],
    ) -> Result<IrValue, IrInterpError> {
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
