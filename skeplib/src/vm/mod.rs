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
mod profiler;
mod runner;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use crate::bytecode::{BytecodeModule, FunctionChunk, Value};
pub use config::VmConfig;
pub use error::{VmError, VmErrorKind};
pub use host::{StdIoHost, TestHost};
pub use host_trait::BuiltinHost;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vm;

pub(crate) use builtins::default_builtin_id;
pub use builtins::{BuiltinHandler, BuiltinRegistry};

impl Vm {
    fn default_registry() -> &'static BuiltinRegistry {
        static REGISTRY: OnceLock<BuiltinRegistry> = OnceLock::new();
        REGISTRY.get_or_init(BuiltinRegistry::with_defaults)
    }

    fn function_table(module: &BytecodeModule) -> Vec<&FunctionChunk> {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        struct CacheKey {
            ptr: usize,
            len: usize,
            name_fingerprint: u64,
        }

        static FN_TABLE_CACHE: OnceLock<Mutex<HashMap<CacheKey, Vec<String>>>> = OnceLock::new();
        let cache = FN_TABLE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut name_fingerprint = 0u64;
        for name in module.functions.keys() {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            name.hash(&mut hasher);
            name_fingerprint ^= hasher.finish();
        }
        let key = CacheKey {
            ptr: module as *const BytecodeModule as usize,
            len: module.functions.len(),
            name_fingerprint,
        };
        let names = {
            let mut cache = cache.lock().expect("function table cache poisoned");
            cache
                .entry(key)
                .or_insert_with(|| {
                    let mut names = module.functions.keys().cloned().collect::<Vec<_>>();
                    names.sort();
                    names
                })
                .clone()
        };
        names
            .iter()
            .filter_map(|name| module.functions.get(name))
            .collect()
    }

    pub fn run_module_main(module: &BytecodeModule) -> Result<Value, VmError> {
        Self::run_module_main_with_config(module, VmConfig::default())
    }

    pub fn run_module_main_with_config(
        module: &BytecodeModule,
        config: VmConfig,
    ) -> Result<Value, VmError> {
        let _profile_session = profiler::SessionGuard::start("run_module_main");
        let _startup_timer = profiler::ScopedTimer::new(profiler::Event::VmStartup);
        let mut host = StdIoHost::default();
        let reg = Self::default_registry();
        let fn_table = Self::function_table(module);
        let globals_init_locals = module
            .functions
            .get("__globals_init")
            .map(|f| f.locals_count)
            .unwrap_or(0);
        let mut globals = vec![Value::Unit; globals_init_locals];
        if module.functions.contains_key("__globals_init") {
            let _globals_timer = profiler::ScopedTimer::new(profiler::Event::GlobalsInit);
            let mut env = runner::ExecEnv {
                module,
                fn_table: &fn_table,
                globals: &mut globals,
                host: &mut host,
                reg,
            };
            let _ = runner::run_function(
                &mut env,
                "__globals_init",
                Vec::new(),
                runner::RunOptions { depth: 0, config },
            )?;
        }
        let mut env = runner::ExecEnv {
            module,
            fn_table: &fn_table,
            globals: &mut globals,
            host: &mut host,
            reg,
        };
        let _main_timer = profiler::ScopedTimer::new(profiler::Event::MainRun);
        runner::run_function(
            &mut env,
            "main",
            Vec::new(),
            runner::RunOptions { depth: 0, config },
        )
    }

    pub fn run_module_main_with_host(
        module: &BytecodeModule,
        host: &mut dyn BuiltinHost,
    ) -> Result<Value, VmError> {
        let _profile_session = profiler::SessionGuard::start("run_module_main_with_host");
        let _startup_timer = profiler::ScopedTimer::new(profiler::Event::VmStartup);
        let reg = Self::default_registry();
        let fn_table = Self::function_table(module);
        let globals_init_locals = module
            .functions
            .get("__globals_init")
            .map(|f| f.locals_count)
            .unwrap_or(0);
        let mut globals = vec![Value::Unit; globals_init_locals];
        if module.functions.contains_key("__globals_init") {
            let _globals_timer = profiler::ScopedTimer::new(profiler::Event::GlobalsInit);
            let mut env = runner::ExecEnv {
                module,
                fn_table: &fn_table,
                globals: &mut globals,
                host,
                reg,
            };
            let _ = runner::run_function(
                &mut env,
                "__globals_init",
                Vec::new(),
                runner::RunOptions {
                    depth: 0,
                    config: VmConfig::default(),
                },
            )?;
        }
        let mut env = runner::ExecEnv {
            module,
            fn_table: &fn_table,
            globals: &mut globals,
            host,
            reg,
        };
        let _main_timer = profiler::ScopedTimer::new(profiler::Event::MainRun);
        runner::run_function(
            &mut env,
            "main",
            Vec::new(),
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
        let _profile_session = profiler::SessionGuard::start("run_module_main_with_registry");
        let _startup_timer = profiler::ScopedTimer::new(profiler::Event::VmStartup);
        let fn_table = Self::function_table(module);
        let globals_init_locals = module
            .functions
            .get("__globals_init")
            .map(|f| f.locals_count)
            .unwrap_or(0);
        let mut globals = vec![Value::Unit; globals_init_locals];
        if module.functions.contains_key("__globals_init") {
            let _globals_timer = profiler::ScopedTimer::new(profiler::Event::GlobalsInit);
            let mut env = runner::ExecEnv {
                module,
                fn_table: &fn_table,
                globals: &mut globals,
                host,
                reg,
            };
            let _ = runner::run_function(
                &mut env,
                "__globals_init",
                Vec::new(),
                runner::RunOptions {
                    depth: 0,
                    config: VmConfig::default(),
                },
            )?;
        }
        let mut env = runner::ExecEnv {
            module,
            fn_table: &fn_table,
            globals: &mut globals,
            host,
            reg,
        };
        let _main_timer = profiler::ScopedTimer::new(profiler::Event::MainRun);
        runner::run_function(
            &mut env,
            "main",
            Vec::new(),
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
            method_names: Vec::new(),
            struct_shapes: Vec::new(),
        };
        Self::run_module_main(&module)
    }
}
