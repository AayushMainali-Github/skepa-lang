use crate::{RtFunctionRef, RtHost, RtResult, RtValue};
use std::panic::{catch_unwind, AssertUnwindSafe};

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
        let function = self.functions.get(function.0 as usize).ok_or_else(|| {
            crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                format!("unknown runtime function id {}", function.0),
            )
        })?;
        catch_unwind(AssertUnwindSafe(|| function(host, args))).map_err(|_| {
            crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "runtime function call panicked, likely due to invalid arguments",
            )
        })?
    }
}
