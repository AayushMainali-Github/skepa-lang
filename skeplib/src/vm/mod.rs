//! Bytecode VM entrypoint and public VM surface.
//!
//! Internals are split into:
//! - `error`: VM runtime error types
//! - `config`: runtime execution configuration
//! - `host_trait` + `host`: I/O boundary for builtins
//! - `builtins`: builtin registry and package handlers
//! - `runner`: bytecode interpreter loop and instruction handlers

mod builtins;
mod config;
mod error;
mod host;
mod host_trait;
mod runner;

use crate::bytecode::{BytecodeModule, FunctionChunk, Value};
pub use config::VmConfig;
pub use error::{VmError, VmErrorKind};
pub use host::{StdIoHost, TestHost};
pub use host_trait::BuiltinHost;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vm;

pub use builtins::{BuiltinHandler, BuiltinRegistry};

impl Vm {
    pub fn run_module_main(module: &BytecodeModule) -> Result<Value, VmError> {
        Self::run_module_main_with_config(module, VmConfig::default())
    }

    pub fn run_module_main_with_config(
        module: &BytecodeModule,
        config: VmConfig,
    ) -> Result<Value, VmError> {
        let mut host = StdIoHost::default();
        let reg = BuiltinRegistry::with_defaults();
        let globals_init_locals = module
            .functions
            .get("__globals_init")
            .map(|f| f.locals_count)
            .unwrap_or(0);
        let mut globals = vec![Value::Unit; globals_init_locals];
        if module.functions.contains_key("__globals_init") {
            let _ = runner::run_function(
                module,
                "__globals_init",
                Vec::new(),
                &mut globals,
                &mut host,
                &reg,
                runner::RunOptions { depth: 0, config },
            )?;
        }
        runner::run_function(
            module,
            "main",
            Vec::new(),
            &mut globals,
            &mut host,
            &reg,
            runner::RunOptions { depth: 0, config },
        )
    }

    pub fn run_module_main_with_host(
        module: &BytecodeModule,
        host: &mut dyn BuiltinHost,
    ) -> Result<Value, VmError> {
        let reg = BuiltinRegistry::with_defaults();
        let globals_init_locals = module
            .functions
            .get("__globals_init")
            .map(|f| f.locals_count)
            .unwrap_or(0);
        let mut globals = vec![Value::Unit; globals_init_locals];
        if module.functions.contains_key("__globals_init") {
            let _ = runner::run_function(
                module,
                "__globals_init",
                Vec::new(),
                &mut globals,
                host,
                &reg,
                runner::RunOptions {
                    depth: 0,
                    config: VmConfig::default(),
                },
            )?;
        }
        runner::run_function(
            module,
            "main",
            Vec::new(),
            &mut globals,
            host,
            &reg,
            runner::RunOptions {
                depth: 0,
                config: VmConfig::default(),
            },
        )
    }

    pub fn run_module_main_with_registry(
        module: &BytecodeModule,
        host: &mut dyn BuiltinHost,
        reg: &BuiltinRegistry,
    ) -> Result<Value, VmError> {
        let globals_init_locals = module
            .functions
            .get("__globals_init")
            .map(|f| f.locals_count)
            .unwrap_or(0);
        let mut globals = vec![Value::Unit; globals_init_locals];
        if module.functions.contains_key("__globals_init") {
            let _ = runner::run_function(
                module,
                "__globals_init",
                Vec::new(),
                &mut globals,
                host,
                reg,
                runner::RunOptions {
                    depth: 0,
                    config: VmConfig::default(),
                },
            )?;
        }
        runner::run_function(
            module,
            "main",
            Vec::new(),
            &mut globals,
            host,
            reg,
            runner::RunOptions {
                depth: 0,
                config: VmConfig::default(),
            },
        )
    }

    pub fn run_main(chunk: &FunctionChunk) -> Result<Value, VmError> {
        let module = BytecodeModule {
            functions: vec![(chunk.name.clone(), chunk.clone())]
                .into_iter()
                .collect(),
        };
        Self::run_module_main(&module)
    }
}
