use crate::bytecode::Value;
use crate::vm::{VmError, VmErrorKind};

pub(super) struct VmState {
    stack: Vec<Value>,
    locals: Vec<Value>,
}

impl VmState {
    pub(super) fn new(locals_count: usize, args: Vec<Value>) -> Self {
        let mut locals = vec![Value::Unit; locals_count.max(1)];
        for (i, arg) in args.into_iter().enumerate() {
            if i < locals.len() {
                locals[i] = arg;
            }
        }
        Self {
            stack: Vec::new(),
            locals,
        }
    }

    pub(super) fn stack_mut(&mut self) -> &mut Vec<Value> {
        &mut self.stack
    }

    pub(super) fn push_const(&mut self, v: Value) {
        self.stack.push(v);
    }

    pub(super) fn load_local(
        &mut self,
        slot: usize,
        function_name: &str,
        ip: usize,
    ) -> Result<(), VmError> {
        let Some(v) = self.locals.get(slot).cloned() else {
            return Err(super::err_at(
                VmErrorKind::InvalidLocal,
                format!("Invalid local slot {slot}"),
                function_name,
                ip,
            ));
        };
        self.stack.push(v);
        Ok(())
    }

    pub(super) fn store_local(
        &mut self,
        slot: usize,
        function_name: &str,
        ip: usize,
    ) -> Result<(), VmError> {
        let Some(v) = self.stack.pop() else {
            return Err(super::err_at(
                VmErrorKind::StackUnderflow,
                "Stack underflow on StoreLocal",
                function_name,
                ip,
            ));
        };
        if slot >= self.locals.len() {
            self.locals.resize(slot + 1, Value::Unit);
        }
        self.locals[slot] = v;
        Ok(())
    }

    pub(super) fn pop_discard(&mut self, function_name: &str, ip: usize) -> Result<(), VmError> {
        if self.stack.pop().is_none() {
            return Err(super::err_at(
                VmErrorKind::StackUnderflow,
                "Stack underflow on Pop",
                function_name,
                ip,
            ));
        }
        Ok(())
    }

    pub(super) fn finish_return(&mut self) -> Value {
        self.stack.pop().unwrap_or(Value::Unit)
    }
}
