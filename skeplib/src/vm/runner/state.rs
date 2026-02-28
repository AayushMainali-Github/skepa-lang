use crate::bytecode::{FunctionChunk, Value};

pub(super) struct CallFrame<'a> {
    pub chunk: &'a FunctionChunk,
    pub function_name: &'a str,
    pub ip: usize,
    pub locals: Vec<Value>,
    pub stack: Vec<Value>,
}

impl<'a> CallFrame<'a> {
    pub(super) fn new(
        chunk: &'a FunctionChunk,
        function_name: &'a str,
        args: Vec<Value>,
        stack_capacity: usize,
    ) -> Self {
        let mut locals = args;
        if locals.len() < chunk.locals_count {
            locals.resize(chunk.locals_count, Value::Unit);
        }
        Self {
            chunk,
            function_name,
            ip: 0,
            locals,
            stack: Vec::with_capacity(stack_capacity.max(8)),
        }
    }
}
