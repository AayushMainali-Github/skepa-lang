use crate::bytecode::{StructShape, Value};
use crate::vm::{VmError, VmErrorKind};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};

type FieldSlotCache = HashMap<String, HashMap<String, usize>>;
type ShapeCache = HashMap<String, Rc<StructShape>>;

thread_local! {
    static SHAPE_CACHE: RefCell<ShapeCache> = RefCell::new(HashMap::new());
}

fn cached_shape(name: &str, fields: &[String]) -> Rc<StructShape> {
    SHAPE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache
            .entry(name.to_string())
            .or_insert_with(|| {
                Rc::new(StructShape {
                    name: name.to_string(),
                    field_names: Rc::<[String]>::from(fields.to_vec()),
                })
            })
            .clone()
    })
}

fn cached_field_slot(name: &str, field_names: &[String], field: &str) -> Option<usize> {
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

    let slot = field_names.iter().position(|k| k == field)?;
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
    let shape = cached_shape(name, fields);
    stack.push(Value::Struct {
        shape,
        fields: Rc::<[Value]>::from(values),
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
    let Value::Struct { shape, fields } = base else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "StructGet expects Struct",
            function_name,
            ip,
        ));
    };
    let Some(slot) = cached_field_slot(&shape.name, &shape.field_names, field) else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            format!("Unknown struct field `{field}` on `{}`", shape.name),
            function_name,
            ip,
        ));
    };
    stack.push(fields[slot].clone());
    Ok(())
}

pub(super) fn struct_get_slot(
    stack: &mut Vec<Value>,
    slot: usize,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let _timer = super::super::profiler::ScopedTimer::new(super::super::profiler::Event::StructGet);
    let Some(base) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "StructGetSlot expects struct value",
            function_name,
            ip,
        ));
    };
    let Value::Struct { shape, fields } = base else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "StructGetSlot expects Struct",
            function_name,
            ip,
        ));
    };
    let Some(value) = fields.get(slot) else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            format!("Unknown struct field slot `{slot}` on `{}`", shape.name),
            function_name,
            ip,
        ));
    };
    stack.push(value.clone());
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

pub(super) fn struct_set_path_slots(
    stack: &mut Vec<Value>,
    path: &[usize],
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let _timer =
        super::super::profiler::ScopedTimer::new(super::super::profiler::Event::StructSetPath);
    if path.is_empty() {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "StructSetPathSlots requires non-empty field path",
            function_name,
            ip,
        ));
    }
    let Some(value) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "StructSetPathSlots expects value",
            function_name,
            ip,
        ));
    };
    let Some(base) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "StructSetPathSlots expects struct value",
            function_name,
            ip,
        ));
    };
    let updated = set_field_path_slots(base, path, value).map_err(|msg| {
        super::err_at(
            VmErrorKind::TypeMismatch,
            format!("StructSetPathSlots failed: {msg}"),
            function_name,
            ip,
        )
    })?;
    stack.push(updated);
    Ok(())
}

fn set_field_path(cur: Value, path: &[String], value: Value) -> Result<Value, String> {
    let Value::Struct { shape, fields } = cur else {
        return Err("expected Struct along field path".to_string());
    };
    let key = &path[0];
    let Some(pos) = cached_field_slot(&shape.name, &shape.field_names, key) else {
        return Err(format!("unknown field `{key}` on struct `{}`", shape.name));
    };
    let mut fields = fields.as_ref().to_vec();
    if path.len() == 1 {
        fields[pos] = value;
        return Ok(Value::Struct {
            shape,
            fields: Rc::<[Value]>::from(fields),
        });
    }
    let child = fields[pos].clone();
    let next = set_field_path(child, &path[1..], value)?;
    fields[pos] = next;
    Ok(Value::Struct {
        shape,
        fields: Rc::<[Value]>::from(fields),
    })
}

fn set_field_path_slots(cur: Value, path: &[usize], value: Value) -> Result<Value, String> {
    let Value::Struct { shape, fields } = cur else {
        return Err("expected Struct along field path".to_string());
    };
    let Some(_) = fields.get(path[0]) else {
        return Err(format!(
            "unknown field slot `{}` on struct `{}`",
            path[0], shape.name
        ));
    };
    let mut fields = fields.as_ref().to_vec();
    if path.len() == 1 {
        fields[path[0]] = value;
        return Ok(Value::Struct {
            shape,
            fields: Rc::<[Value]>::from(fields),
        });
    }
    let child = fields[path[0]].clone();
    let next = set_field_path_slots(child, &path[1..], value)?;
    fields[path[0]] = next;
    Ok(Value::Struct {
        shape,
        fields: Rc::<[Value]>::from(fields),
    })
}
