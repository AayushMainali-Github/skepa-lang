use crate::bytecode::{BytecodeModule, StructShape, Value};
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

pub(super) fn make_struct_id(
    stack: &mut Vec<Value>,
    module: &BytecodeModule,
    id: usize,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let Some(shape) = module.struct_shapes.get(id) else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            format!("Unknown struct shape id `{id}`"),
            function_name,
            ip,
        ));
    };
    make_struct(stack, &shape.name, &shape.field_names, function_name, ip)
}

pub(super) fn struct_get(
    stack: &mut Vec<Value>,
    field: &str,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
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

pub(super) fn struct_get_local_slot(
    locals: &[Value],
    stack: &mut Vec<Value>,
    local_slot: usize,
    field_slot: usize,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let Some(base) = locals.get(local_slot) else {
        return Err(super::err_at(
            VmErrorKind::InvalidLocal,
            format!("Invalid local slot {local_slot}"),
            function_name,
            ip,
        ));
    };
    let Value::Struct { shape, fields } = base else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "StructGetLocalSlot expects Struct local",
            function_name,
            ip,
        ));
    };
    let Some(value) = fields.get(field_slot) else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            format!(
                "Unknown struct field slot `{field_slot}` on `{}`",
                shape.name
            ),
            function_name,
            ip,
        ));
    };
    stack.push(value.clone());
    Ok(())
}

pub(super) fn struct_get_local_slot_add_to_local(
    locals: &mut [Value],
    struct_slot: usize,
    field_slot: usize,
    dst: usize,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let Some(base) = locals.get(struct_slot) else {
        return Err(super::err_at(
            VmErrorKind::InvalidLocal,
            format!("Invalid local slot {struct_slot}"),
            function_name,
            ip,
        ));
    };
    let Value::Struct { shape, fields } = base else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "StructGetLocalSlotAddToLocal expects Struct local",
            function_name,
            ip,
        ));
    };
    let Some(value) = fields.get(field_slot) else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            format!(
                "Unknown struct field slot `{field_slot}` on `{}`",
                shape.name
            ),
            function_name,
            ip,
        ));
    };
    let Value::Int(field_value) = value else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "StructGetLocalSlotAddToLocal expects Int field",
            function_name,
            ip,
        ));
    };
    let field_value = *field_value;
    let Some(dst_slot) = locals.get_mut(dst) else {
        return Err(super::err_at(
            VmErrorKind::InvalidLocal,
            format!("Invalid local slot {dst}"),
            function_name,
            ip,
        ));
    };
    match dst_slot {
        Value::Int(acc) => {
            *acc += field_value;
            Ok(())
        }
        _ => Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "StructGetLocalSlotAddToLocal expects Int destination local",
            function_name,
            ip,
        )),
    }
}

pub(super) fn struct_field_add_mul_field_mod_local_to_local(
    locals: &mut [Value],
    struct_slot: usize,
    arg_slot: usize,
    arg_op: crate::bytecode::IntLocalConstOp,
    arg_rhs: i64,
    lhs_field_slot: usize,
    rhs_field_slot: usize,
    mul: i64,
    modulo: i64,
    dst: usize,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let Some(base) = locals.get(struct_slot) else {
        return Err(super::err_at(
            VmErrorKind::InvalidLocal,
            format!("Invalid local slot {struct_slot}"),
            function_name,
            ip,
        ));
    };
    let Value::Struct { shape, fields } = base else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "StructFieldAddMulFieldModLocalToLocal expects Struct local",
            function_name,
            ip,
        ));
    };
    let lhs_field = match fields.get(lhs_field_slot) {
        Some(Value::Int(v)) => *v,
        Some(_) => {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                "StructFieldAddMulFieldModLocalToLocal expects Int lhs field",
                function_name,
                ip,
            ))
        }
        None => {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                format!(
                    "Unknown struct field slot `{lhs_field_slot}` on `{}`",
                    shape.name
                ),
                function_name,
                ip,
            ))
        }
    };
    let rhs_field = match fields.get(rhs_field_slot) {
        Some(Value::Int(v)) => *v,
        Some(_) => {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                "StructFieldAddMulFieldModLocalToLocal expects Int rhs field",
                function_name,
                ip,
            ))
        }
        None => {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                format!(
                    "Unknown struct field slot `{rhs_field_slot}` on `{}`",
                    shape.name
                ),
                function_name,
                ip,
            ))
        }
    };
    let arg_value = match locals.get(arg_slot) {
        Some(Value::Int(v)) => *v,
        Some(_) => {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                "StructFieldAddMulFieldModLocalToLocal expects Int argument local",
                function_name,
                ip,
            ))
        }
        None => {
            return Err(super::err_at(
                VmErrorKind::InvalidLocal,
                format!("Invalid local slot {arg_slot}"),
                function_name,
                ip,
            ))
        }
    };
    let arg_value = match arg_op {
        crate::bytecode::IntLocalConstOp::Add => arg_value + arg_rhs,
        crate::bytecode::IntLocalConstOp::Sub => arg_value - arg_rhs,
        crate::bytecode::IntLocalConstOp::Mul => arg_value * arg_rhs,
        crate::bytecode::IntLocalConstOp::Div => {
            if arg_rhs == 0 {
                return Err(super::err_at(
                    VmErrorKind::DivisionByZero,
                    "division by zero",
                    function_name,
                    ip,
                ));
            }
            arg_value / arg_rhs
        }
        crate::bytecode::IntLocalConstOp::Mod => {
            if arg_rhs == 0 {
                return Err(super::err_at(
                    VmErrorKind::DivisionByZero,
                    "modulo by zero",
                    function_name,
                    ip,
                ));
            }
            arg_value % arg_rhs
        }
    };
    if modulo == 0 {
        return Err(super::err_at(
            VmErrorKind::DivisionByZero,
            "modulo by zero",
            function_name,
            ip,
        ));
    }
    let result = (((lhs_field + arg_value) * mul) + rhs_field) % modulo;
    let Some(dst_slot) = locals.get_mut(dst) else {
        return Err(super::err_at(
            VmErrorKind::InvalidLocal,
            format!("Invalid local slot {dst}"),
            function_name,
            ip,
        ));
    };
    match dst_slot {
        Value::Int(acc) => {
            *acc += result;
            Ok(())
        }
        _ => Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "StructFieldAddMulFieldModLocalToLocal expects Int destination local",
            function_name,
            ip,
        )),
    }
}

pub(super) fn struct_set_path(
    stack: &mut Vec<Value>,
    path: &[String],
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
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
