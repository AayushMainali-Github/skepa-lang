use crate::{RtFunctionRef, RtHost, RtResult, RtValue};

pub type RtNativeFn = fn(&mut dyn RtHost, &[RtValue]) -> RtResult<RtValue>;

#[derive(Default)]
pub struct RtFunctionRegistry {
    functions: Vec<RtNativeFn>,
}

impl RtFunctionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, function: RtNativeFn) -> RtFunctionRef {
        let id = self.functions.len() as u32;
        self.functions.push(function);
        RtFunctionRef(id)
    }

    pub fn call(
        &self,
        host: &mut dyn RtHost,
        function: RtFunctionRef,
        args: &[RtValue],
    ) -> RtResult<RtValue> {
        let function = self
            .functions
            .get(function.0 as usize)
            .ok_or_else(|| crate::RtError::unsupported_builtin("runtime.function"))?;
        function(host, args)
    }
}
