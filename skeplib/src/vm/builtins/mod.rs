mod arr;
mod datetime;
mod fs;
mod io;
mod os;
mod random;
mod str;
mod vec;

use std::collections::HashMap;

use crate::bytecode::Value;

use super::{BuiltinHost, VmError, VmErrorKind};

pub type BuiltinHandler = fn(&mut dyn BuiltinHost, Vec<Value>) -> Result<Value, VmError>;

#[derive(Default)]
pub struct BuiltinRegistry {
    handlers: HashMap<String, BuiltinHandler>,
}

impl BuiltinRegistry {
    pub fn with_defaults() -> Self {
        let mut r = Self::default();
        io::register(&mut r);
        str::register(&mut r);
        arr::register(&mut r);
        datetime::register(&mut r);
        fs::register(&mut r);
        os::register(&mut r);
        random::register(&mut r);
        vec::register(&mut r);
        r
    }

    fn key(package: &str, name: &str) -> String {
        format!("{package}.{name}")
    }

    pub fn register(&mut self, package: &str, name: &str, handler: BuiltinHandler) {
        self.handlers.insert(Self::key(package, name), handler);
    }

    pub(crate) fn call(
        &self,
        host: &mut dyn BuiltinHost,
        package: &str,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, VmError> {
        let key = Self::key(package, name);
        let Some(handler) = self.handlers.get(&key).copied() else {
            return Err(VmError::new(
                VmErrorKind::UnknownBuiltin,
                format!("Unknown builtin `{key}`"),
            ));
        };
        handler(host, args)
    }
}
