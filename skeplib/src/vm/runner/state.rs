use crate::bytecode::{FunctionChunk, Value};

pub(super) struct CallFrame<'a> {
    pub chunk: &'a FunctionChunk,
    pub function_name: &'a str,
    pub ip: usize,
    pub locals: Vec<Value>,
    pub stack: Vec<Value>,
}

pub(super) struct FrameStorage {
    pub locals: Vec<Value>,
    pub stack: Vec<Value>,
}

impl FrameStorage {
    pub(super) fn new(
        mut locals: Vec<Value>,
        mut stack: Vec<Value>,
        locals_len: usize,
        stack_capacity: usize,
    ) -> Self {
        locals.clear();
        if locals.capacity() < locals_len {
            locals.reserve(locals_len - locals.capacity());
        }
        stack.clear();
        if stack.capacity() < stack_capacity {
            stack.reserve(stack_capacity - stack.capacity());
        }
        Self { locals, stack }
    }
}

pub(super) fn acquire_storage(
    locals_len: usize,
    stack_capacity: usize,
    locals_pool: &mut Vec<Vec<Value>>,
    stack_pool: &mut Vec<Vec<Value>>,
) -> FrameStorage {
    let locals = locals_pool.pop().unwrap_or_default();
    let stack = stack_pool.pop().unwrap_or_default();
    FrameStorage::new(locals, stack, locals_len, stack_capacity.max(8))
}

impl<'a> CallFrame<'a> {
    pub(super) fn with_storage(
        chunk: &'a FunctionChunk,
        function_name: &'a str,
        storage: FrameStorage,
    ) -> Self {
        Self {
            chunk,
            function_name,
            ip: 0,
            locals: storage.locals,
            stack: storage.stack,
        }
    }

    pub(super) fn into_storage(mut self) -> FrameStorage {
        self.locals.clear();
        self.stack.clear();
        FrameStorage {
            locals: self.locals,
            stack: self.stack,
        }
    }

    #[inline(always)]
    pub(super) fn read_local_cloned(&self, slot: usize) -> Option<Value> {
        if slot < self.locals.len() {
            // Compiled bytecode keeps frame locals pre-sized, so this is the hot path.
            Some(unsafe { self.locals.get_unchecked(slot).clone() })
        } else {
            None
        }
    }

    #[inline(always)]
    pub(super) fn write_local_fast(&mut self, slot: usize, value: Value) -> bool {
        if slot < self.locals.len() {
            // Compiled bytecode keeps frame locals pre-sized, so this is the hot path.
            unsafe {
                *self.locals.get_unchecked_mut(slot) = value;
            }
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub(super) fn add_const_to_int_local(
        &mut self,
        slot: usize,
        rhs: i64,
    ) -> Option<Result<(), ()>> {
        if slot >= self.locals.len() {
            return None;
        }
        match unsafe { self.locals.get_unchecked_mut(slot) } {
            Value::Int(lhs) => {
                *lhs += rhs;
                Some(Ok(()))
            }
            _ => Some(Err(())),
        }
    }

    #[inline(always)]
    pub(super) fn add_int_local_to_local(
        &mut self,
        dst: usize,
        src: usize,
    ) -> Option<Result<(), ()>> {
        if dst >= self.locals.len() || src >= self.locals.len() {
            return None;
        }
        if dst == src {
            match unsafe { self.locals.get_unchecked_mut(dst) } {
                Value::Int(value) => {
                    *value += *value;
                    Some(Ok(()))
                }
                _ => Some(Err(())),
            }
        } else if dst < src {
            let (left, right) = self.locals.split_at_mut(src);
            match (&mut left[dst], &right[0]) {
                (Value::Int(dst_value), Value::Int(src_value)) => {
                    *dst_value += *src_value;
                    Some(Ok(()))
                }
                _ => Some(Err(())),
            }
        } else {
            let (left, right) = self.locals.split_at_mut(dst);
            match (&mut right[0], &left[src]) {
                (Value::Int(dst_value), Value::Int(src_value)) => {
                    *dst_value += *src_value;
                    Some(Ok(()))
                }
                _ => Some(Err(())),
            }
        }
    }

    #[inline(always)]
    pub(super) fn apply_stack_int_to_local(
        &mut self,
        slot: usize,
        op: crate::bytecode::IntLocalConstOp,
    ) -> Option<Result<(), crate::vm::VmErrorKind>> {
        if slot >= self.locals.len() {
            return None;
        }
        let Some(rhs) = self.stack.pop() else {
            return Some(Err(crate::vm::VmErrorKind::StackUnderflow));
        };
        match (unsafe { self.locals.get_unchecked_mut(slot) }, rhs) {
            (Value::Int(lhs), Value::Int(rhs)) => {
                match op {
                    crate::bytecode::IntLocalConstOp::Add => *lhs += rhs,
                    crate::bytecode::IntLocalConstOp::Sub => *lhs -= rhs,
                    crate::bytecode::IntLocalConstOp::Mul => *lhs *= rhs,
                    crate::bytecode::IntLocalConstOp::Div => {
                        if rhs == 0 {
                            return Some(Err(crate::vm::VmErrorKind::DivisionByZero));
                        }
                        *lhs /= rhs;
                    }
                    crate::bytecode::IntLocalConstOp::Mod => {
                        if rhs == 0 {
                            return Some(Err(crate::vm::VmErrorKind::DivisionByZero));
                        }
                        *lhs %= rhs;
                    }
                }
                Some(Ok(()))
            }
            (_, rhs) => {
                self.stack.push(rhs);
                Some(Err(crate::vm::VmErrorKind::TypeMismatch))
            }
        }
    }

    #[inline(always)]
    pub(super) fn compute_int_local_const_to_local(
        &mut self,
        src: usize,
        dst: usize,
        op: crate::bytecode::IntLocalConstOp,
        rhs: i64,
    ) -> Option<Result<(), crate::vm::VmErrorKind>> {
        if src >= self.locals.len() || dst >= self.locals.len() {
            return None;
        }
        let result = match self.locals.get(src)? {
            Value::Int(lhs) => match op {
                crate::bytecode::IntLocalConstOp::Add => *lhs + rhs,
                crate::bytecode::IntLocalConstOp::Sub => *lhs - rhs,
                crate::bytecode::IntLocalConstOp::Mul => *lhs * rhs,
                crate::bytecode::IntLocalConstOp::Div => {
                    if rhs == 0 {
                        return Some(Err(crate::vm::VmErrorKind::DivisionByZero));
                    }
                    *lhs / rhs
                }
                crate::bytecode::IntLocalConstOp::Mod => {
                    if rhs == 0 {
                        return Some(Err(crate::vm::VmErrorKind::DivisionByZero));
                    }
                    *lhs % rhs
                }
            },
            _ => return Some(Err(crate::vm::VmErrorKind::TypeMismatch)),
        };
        match unsafe { self.locals.get_unchecked_mut(dst) } {
            Value::Int(value) => {
                *value = result;
                Some(Ok(()))
            }
            slot => {
                *slot = Value::Int(result);
                Some(Ok(()))
            }
        }
    }

    #[inline(always)]
    pub(super) fn compute_int_local_local_to_local(
        &mut self,
        lhs: usize,
        rhs: usize,
        dst: usize,
        op: crate::bytecode::IntLocalConstOp,
    ) -> Option<Result<(), crate::vm::VmErrorKind>> {
        if lhs >= self.locals.len() || rhs >= self.locals.len() || dst >= self.locals.len() {
            return None;
        }
        let left = match self.locals.get(lhs)? {
            Value::Int(value) => *value,
            _ => return Some(Err(crate::vm::VmErrorKind::TypeMismatch)),
        };
        let right = match self.locals.get(rhs)? {
            Value::Int(value) => *value,
            _ => return Some(Err(crate::vm::VmErrorKind::TypeMismatch)),
        };
        let result = match op {
            crate::bytecode::IntLocalConstOp::Add => left + right,
            crate::bytecode::IntLocalConstOp::Sub => left - right,
            crate::bytecode::IntLocalConstOp::Mul => left * right,
            crate::bytecode::IntLocalConstOp::Div => {
                if right == 0 {
                    return Some(Err(crate::vm::VmErrorKind::DivisionByZero));
                }
                left / right
            }
            crate::bytecode::IntLocalConstOp::Mod => {
                if right == 0 {
                    return Some(Err(crate::vm::VmErrorKind::DivisionByZero));
                }
                left % right
            }
        };
        match unsafe { self.locals.get_unchecked_mut(dst) } {
            Value::Int(value) => {
                *value = result;
                Some(Ok(()))
            }
            slot => {
                *slot = Value::Int(result);
                Some(Ok(()))
            }
        }
    }

    #[inline(always)]
    pub(super) fn pop2(&mut self) -> Option<(Value, Value)> {
        let rhs = self.stack.pop()?;
        let lhs = self.stack.pop()?;
        Some((lhs, rhs))
    }
}
