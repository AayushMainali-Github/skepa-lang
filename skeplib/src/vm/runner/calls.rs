use crate::bytecode::{BytecodeModule, FunctionChunk, Value};
use crate::vm::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

pub(super) struct CallEnv<'a> {
    pub host: &'a mut dyn BuiltinHost,
    pub reg: &'a BuiltinRegistry,
}

pub(super) struct Site<'a> {
    pub function_name: &'a str,
    pub ip: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ModuleCacheKey {
    ptr: usize,
    len: usize,
    name_fingerprint: u64,
}

type MethodMap = HashMap<String, HashMap<String, usize>>;
type MethodCache = HashMap<ModuleCacheKey, MethodMap>;
type MethodIdMap = HashMap<String, HashMap<usize, usize>>;
type MethodIdCache = HashMap<ModuleCacheKey, MethodIdMap>;

pub(super) fn take_call_args(stack: &mut Vec<Value>, argc: usize) -> Vec<Value> {
    let split = stack.len() - argc;
    stack.split_off(split)
}

fn module_cache_key(module: &BytecodeModule) -> ModuleCacheKey {
    let mut name_fingerprint = 0u64;
    for name in module.functions.keys() {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hasher);
        name_fingerprint ^= hasher.finish();
    }
    ModuleCacheKey {
        ptr: module as *const BytecodeModule as usize,
        len: module.functions.len(),
        name_fingerprint,
    }
}

pub(super) fn resolve_chunk<'a>(
    module: &'a BytecodeModule,
    callee_name: &str,
    site: Site<'_>,
) -> Result<&'a FunctionChunk, VmError> {
    module.functions.get(callee_name).ok_or_else(|| {
        super::err_at(
            VmErrorKind::UnknownFunction,
            format!("Unknown function `{callee_name}`"),
            site.function_name,
            site.ip,
        )
    })
}

pub(super) fn resolve_chunk_idx<'a>(
    fn_table: &'a [&'a FunctionChunk],
    callee_idx: usize,
    site: Site<'_>,
) -> Result<&'a FunctionChunk, VmError> {
    fn_table.get(callee_idx).copied().ok_or_else(|| {
        super::err_at(
            VmErrorKind::UnknownFunction,
            format!("Invalid function index `{callee_idx}`"),
            site.function_name,
            site.ip,
        )
    })
}

pub(super) fn resolve_function_value<'a>(
    module: &'a BytecodeModule,
    fn_table: &'a [&'a FunctionChunk],
    callee: Value,
    site: Site<'_>,
) -> Result<&'a FunctionChunk, VmError> {
    match callee {
        Value::FunctionIdx(callee_idx) => resolve_chunk_idx(fn_table, callee_idx, site),
        Value::Function(callee_name) => {
            module.functions.get(callee_name.as_ref()).ok_or_else(|| {
                super::err_at(
                    VmErrorKind::UnknownFunction,
                    format!("Unknown function `{callee_name}`"),
                    site.function_name,
                    site.ip,
                )
            })
        }
        _ => Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "CallValue callee must be Function",
            site.function_name,
            site.ip,
        )),
    }
}

fn resolve_method_idx(
    module: &BytecodeModule,
    fn_table: &[&FunctionChunk],
    struct_name: &str,
    method_name: &str,
) -> Option<usize> {
    static METHOD_CACHE: OnceLock<Mutex<MethodCache>> = OnceLock::new();
    let cache = METHOD_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let module_key = module_cache_key(module);

    {
        let cache = cache.lock().expect("method cache poisoned");
        if let Some(idx) = cache
            .get(&module_key)
            .and_then(|methods| methods.get(struct_name))
            .and_then(|methods| methods.get(method_name))
            .copied()
        {
            return Some(idx);
        }
    }

    let mangled = format!("__impl_{struct_name}__{method_name}");
    let idx = fn_table.iter().position(|chunk| chunk.name == mangled)?;

    let mut cache = cache.lock().expect("method cache poisoned");
    cache
        .entry(module_key)
        .or_default()
        .entry(struct_name.to_string())
        .or_default()
        .insert(method_name.to_string(), idx);
    Some(idx)
}

pub(super) fn resolve_method<'a>(
    module: &'a BytecodeModule,
    fn_table: &'a [&'a FunctionChunk],
    receiver: &Value,
    method_name: &str,
    site: Site<'_>,
) -> Result<&'a FunctionChunk, VmError> {
    let Value::Struct { shape, .. } = receiver else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "CallMethod receiver must be Struct",
            site.function_name,
            site.ip,
        ));
    };
    let Some(callee_idx) = resolve_method_idx(module, fn_table, &shape.name, method_name) else {
        return Err(super::err_at(
            VmErrorKind::UnknownFunction,
            format!(
                "Unknown method `{}` on struct `{}`",
                method_name, shape.name
            ),
            site.function_name,
            site.ip,
        ));
    };
    Ok(fn_table[callee_idx])
}

pub(super) fn resolve_method_id<'a>(
    module: &'a BytecodeModule,
    fn_table: &'a [&'a FunctionChunk],
    receiver: &Value,
    method_id: usize,
    site: Site<'_>,
) -> Result<&'a FunctionChunk, VmError> {
    static METHOD_ID_CACHE: OnceLock<Mutex<MethodIdCache>> = OnceLock::new();
    let cache = METHOD_ID_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let module_key = module_cache_key(module);
    let Value::Struct { shape, .. } = receiver else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "CallMethod receiver must be Struct",
            site.function_name,
            site.ip,
        ));
    };

    {
        let cache = cache.lock().expect("method id cache poisoned");
        if let Some(idx) = cache
            .get(&module_key)
            .and_then(|methods| methods.get(&shape.name))
            .and_then(|methods| methods.get(&method_id))
            .copied()
        {
            return Ok(fn_table[idx]);
        }
    }

    let Some(method_name) = module.method_names.get(method_id) else {
        return Err(super::err_at(
            VmErrorKind::UnknownFunction,
            format!("Unknown method id `{method_id}`"),
            site.function_name,
            site.ip,
        ));
    };
    let Some(callee_idx) = resolve_method_idx(module, fn_table, &shape.name, method_name) else {
        return Err(super::err_at(
            VmErrorKind::UnknownFunction,
            format!(
                "Unknown method `{}` on struct `{}`",
                method_name, shape.name
            ),
            site.function_name,
            site.ip,
        ));
    };

    let mut cache = cache.lock().expect("method id cache poisoned");
    cache
        .entry(module_key)
        .or_default()
        .entry(shape.name.clone())
        .or_default()
        .insert(method_id, callee_idx);
    Ok(fn_table[callee_idx])
}

pub(super) fn call_builtin(
    stack: &mut Vec<Value>,
    package: &str,
    name: &str,
    argc: usize,
    env: &mut CallEnv<'_>,
    site: Site<'_>,
) -> Result<(), VmError> {
    let _timer =
        super::super::profiler::ScopedTimer::new(super::super::profiler::Event::CallBuiltin);
    if stack.len() < argc {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on CallBuiltin",
            site.function_name,
            site.ip,
        ));
    }
    let call_args = take_call_args(stack, argc);
    let dispatch_started_at = if super::super::profiler::enabled() {
        Some(std::time::Instant::now())
    } else {
        None
    };
    let _dispatch_timer =
        super::super::profiler::ScopedTimer::new(super::super::profiler::Event::BuiltinDispatch);
    let ret = env.reg.call(env.host, package, name, call_args)?;
    if let Some(started_at) = dispatch_started_at {
        super::super::profiler::record_builtin_call(
            &format!("{package}.{name}"),
            started_at.elapsed(),
        );
    }
    stack.push(ret);
    Ok(())
}

pub(super) fn call_builtin_id(
    stack: &mut Vec<Value>,
    id: u16,
    argc: usize,
    env: &mut CallEnv<'_>,
    site: Site<'_>,
) -> Result<(), VmError> {
    let _timer =
        super::super::profiler::ScopedTimer::new(super::super::profiler::Event::CallBuiltin);
    if stack.len() < argc {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on CallBuiltinId",
            site.function_name,
            site.ip,
        ));
    }
    let call_args = take_call_args(stack, argc);
    let dispatch_started_at = if super::super::profiler::enabled() {
        Some(std::time::Instant::now())
    } else {
        None
    };
    let _dispatch_timer =
        super::super::profiler::ScopedTimer::new(super::super::profiler::Event::BuiltinDispatch);
    let ret = env.reg.call_by_id(env.host, id, call_args)?;
    if let Some(started_at) = dispatch_started_at {
        let label = super::super::builtins::default_builtin_name_by_id(id)
            .map(str::to_string)
            .unwrap_or_else(|| format!("builtin_id.{id}"));
        super::super::profiler::record_builtin_call(&label, started_at.elapsed());
    }
    stack.push(ret);
    Ok(())
}
