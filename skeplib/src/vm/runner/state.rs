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
}
