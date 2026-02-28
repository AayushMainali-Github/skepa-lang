use crate::bytecode::Value;
use crate::vm::{VmError, VmErrorKind};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};

type FieldSlotCache = HashMap<String, HashMap<String, usize>>;

fn cached_field_slot(name: &str, fields: &[(String, Value)], field: &str) -> Option<usize> {
    static CACHE: OnceLock<Mutex<FieldSlotCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let cache = cache.lock().expect("struct field cache poisoned");
        if let Some(slot) = cache
            .get(name)
            .and_then(|field_slots| field_slots.get(field))
            .copied()
        {
            return Some(slot);
        }
    }

    let slot = fields.iter().position(|(k, _)| k == field)?;
    let mut cache = cache.lock().expect("struct field cache poisoned");
    cache
        .entry(name.to_string())
        .or_default()
        .insert(field.to_string(), slot);
    Some(slot)
}

pub(super) fn make_struct(
    stack: &mut Vec<Value>,
    name: &str,
    fields: &[String],
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let _timer =
        super::super::profiler::ScopedTimer::new(super::super::profiler::Event::StructMake);
    if stack.len() < fields.len() {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "MakeStruct expects enough stack values",
            function_name,
            ip,
        ));
    }
    let start = stack.len() - fields.len();
    let values = stack.split_off(start);
    let zipped = fields.iter().cloned().zip(values).collect::<Vec<_>>();
    stack.push(Value::Struct {
        name: name.to_string(),
        fields: Rc::<[(String, Value)]>::from(zipped),
    });
    Ok(())
}

pub(super) fn struct_get(
    stack: &mut Vec<Value>,
    field: &str,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let _timer = super::super::profiler::ScopedTimer::new(super::super::profiler::Event::StructGet);
    let Some(base) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "StructGet expects struct value",
            function_name,
            ip,
        ));
    };
    let Value::Struct { name, fields } = base else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "StructGet expects Struct",
            function_name,
            ip,
        ));
    };
    let Some(slot) = cached_field_slot(&name, &fields, field) else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            format!("Unknown struct field `{field}` on `{name}`"),
            function_name,
            ip,
        ));
    };
    stack.push(fields[slot].1.clone());
    Ok(())
}

pub(super) fn struct_set_path(
    stack: &mut Vec<Value>,
    path: &[String],
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let _timer =
        super::super::profiler::ScopedTimer::new(super::super::profiler::Event::StructSetPath);
    if path.is_empty() {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "StructSetPath requires non-empty field path",
            function_name,
            ip,
        ));
    }
    let Some(value) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "StructSetPath expects value",
            function_name,
            ip,
        ));
    };
    let Some(base) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "StructSetPath expects struct value",
            function_name,
            ip,
        ));
    };
    let updated = set_field_path(base, path, value).map_err(|msg| {
        super::err_at(
            VmErrorKind::TypeMismatch,
            format!("StructSetPath failed: {msg}"),
            function_name,
            ip,
        )
    })?;
    stack.push(updated);
    Ok(())
}

fn set_field_path(cur: Value, path: &[String], value: Value) -> Result<Value, String> {
    let Value::Struct { name, fields } = cur else {
        return Err("expected Struct along field path".to_string());
    };
    let key = &path[0];
    let Some(pos) = cached_field_slot(&name, &fields, key) else {
        return Err(format!("unknown field `{key}` on struct `{name}`"));
    };
    let mut fields = fields.as_ref().to_vec();
    if path.len() == 1 {
        fields[pos].1 = value;
        return Ok(Value::Struct {
            name,
            fields: Rc::<[(String, Value)]>::from(fields),
        });
    }
    let child = fields[pos].1.clone();
    let next = set_field_path(child, &path[1..], value)?;
    fields[pos].1 = next;
    Ok(Value::Struct {
        name,
        fields: Rc::<[(String, Value)]>::from(fields),
    })
}
