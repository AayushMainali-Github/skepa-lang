use super::{BytecodeModule, Instr, IntLocalConstOp, Value};

impl BytecodeModule {
    pub fn disassemble(&self) -> String {
        let mut out = String::new();
        let mut funcs: Vec<_> = self.functions.values().collect();
        funcs.sort_by(|a, b| a.name.cmp(&b.name));
        for f in funcs {
            out.push_str(&format!(
                "fn {} (params={}, locals={})\n",
                f.name, f.param_count, f.locals_count
            ));
            for (ip, instr) in f.code.iter().enumerate() {
                out.push_str(&format!("  {:04} {}\n", ip, fmt_instr(instr)));
            }
        }
        out
    }
}

fn fmt_value(v: &Value) -> String {
    match v {
        Value::Int(i) => format!("Int({i})"),
        Value::Float(n) => format!("Float({n})"),
        Value::Bool(b) => format!("Bool({b})"),
        Value::String(s) => format!("String({s:?})"),
        Value::Array(items) => format!("Array(len={})", items.len()),
        Value::VecHandle(id) => format!("VecHandle({id})"),
        Value::Function(name) => format!("Function({name})"),
        Value::FunctionIdx(idx) => format!("FunctionIdx({idx})"),
        Value::Struct { shape, fields } => {
            format!("Struct({}, fields={})", shape.name, fields.len())
        }
        Value::Unit => "Unit".to_string(),
    }
}

fn fmt_instr(i: &Instr) -> String {
    match i {
        Instr::LoadConst(v) => format!("LoadConst {}", fmt_value(v)),
        Instr::LoadLocal(s) => format!("LoadLocal {s}"),
        Instr::StoreLocal(s) => format!("StoreLocal {s}"),
        Instr::AddLocalToLocal { dst, src } => format!("AddLocalToLocal dst={dst} src={src}"),
        Instr::AddConstToLocal { slot, rhs } => format!("AddConstToLocal slot={slot} rhs={rhs}"),
        Instr::IntLocalLocalOp { lhs, rhs, op } => {
            let op_name = match op {
                IntLocalConstOp::Add => "Add",
                IntLocalConstOp::Sub => "Sub",
                IntLocalConstOp::Mul => "Mul",
                IntLocalConstOp::Div => "Div",
                IntLocalConstOp::Mod => "Mod",
            };
            format!("IntLocalLocalOp lhs={lhs} rhs={rhs} op={op_name}")
        }
        Instr::IntLocalConstOp { slot, op, rhs } => {
            let op_name = match op {
                IntLocalConstOp::Add => "Add",
                IntLocalConstOp::Sub => "Sub",
                IntLocalConstOp::Mul => "Mul",
                IntLocalConstOp::Div => "Div",
                IntLocalConstOp::Mod => "Mod",
            };
            format!("IntLocalConstOp slot={slot} op={op_name} rhs={rhs}")
        }
        Instr::IntLocalConstOpToLocal { src, dst, op, rhs } => {
            let op_name = match op {
                IntLocalConstOp::Add => "Add",
                IntLocalConstOp::Sub => "Sub",
                IntLocalConstOp::Mul => "Mul",
                IntLocalConstOp::Div => "Div",
                IntLocalConstOp::Mod => "Mod",
            };
            format!("IntLocalConstOpToLocal src={src} dst={dst} op={op_name} rhs={rhs}")
        }
        Instr::IntStackOpToLocal { slot, op } => {
            let op_name = match op {
                IntLocalConstOp::Add => "Add",
                IntLocalConstOp::Sub => "Sub",
                IntLocalConstOp::Mul => "Mul",
                IntLocalConstOp::Div => "Div",
                IntLocalConstOp::Mod => "Mod",
            };
            format!("IntStackOpToLocal slot={slot} op={op_name}")
        }
        Instr::IntStackConstOp { op, rhs } => {
            let op_name = match op {
                IntLocalConstOp::Add => "Add",
                IntLocalConstOp::Sub => "Sub",
                IntLocalConstOp::Mul => "Mul",
                IntLocalConstOp::Div => "Div",
                IntLocalConstOp::Mod => "Mod",
            };
            format!("IntStackConstOp op={op_name} rhs={rhs}")
        }
        Instr::IntLocalLocalOpToLocal { lhs, rhs, dst, op } => {
            let op_name = match op {
                IntLocalConstOp::Add => "Add",
                IntLocalConstOp::Sub => "Sub",
                IntLocalConstOp::Mul => "Mul",
                IntLocalConstOp::Div => "Div",
                IntLocalConstOp::Mod => "Mod",
            };
            format!("IntLocalLocalOpToLocal lhs={lhs} rhs={rhs} dst={dst} op={op_name}")
        }
        Instr::LoadGlobal(s) => format!("LoadGlobal {s}"),
        Instr::StoreGlobal(s) => format!("StoreGlobal {s}"),
        Instr::Pop => "Pop".to_string(),
        Instr::NegInt => "NegInt".to_string(),
        Instr::NotBool => "NotBool".to_string(),
        Instr::Add => "Add".to_string(),
        Instr::SubInt => "SubInt".to_string(),
        Instr::MulInt => "MulInt".to_string(),
        Instr::DivInt => "DivInt".to_string(),
        Instr::ModInt => "ModInt".to_string(),
        Instr::Eq => "Eq".to_string(),
        Instr::Neq => "Neq".to_string(),
        Instr::LtInt => "LtInt".to_string(),
        Instr::LteInt => "LteInt".to_string(),
        Instr::GtInt => "GtInt".to_string(),
        Instr::GteInt => "GteInt".to_string(),
        Instr::AndBool => "AndBool".to_string(),
        Instr::OrBool => "OrBool".to_string(),
        Instr::Jump(t) => format!("Jump {t}"),
        Instr::JumpIfFalse(t) => format!("JumpIfFalse {t}"),
        Instr::JumpIfTrue(t) => format!("JumpIfTrue {t}"),
        Instr::JumpIfLocalLtConst { slot, rhs, target } => {
            format!("JumpIfLocalLtConst slot={slot} rhs={rhs} target={target}")
        }
        Instr::Call { name, argc } => format!("Call {name} argc={argc}"),
        Instr::CallIdx { idx, argc } => format!("CallIdx {idx} argc={argc}"),
        Instr::CallIdxAddConst(rhs) => format!("CallIdxAddConst {rhs}"),
        Instr::CallIdxStructFieldAdd(slot) => format!("CallIdxStructFieldAdd field_slot={slot}"),
        Instr::CallValue { argc } => format!("CallValue argc={argc}"),
        Instr::CallMethod { name, argc } => format!("CallMethod {name} argc={argc}"),
        Instr::CallMethodId { id, argc } => format!("CallMethodId {id} argc={argc}"),
        Instr::CallBuiltin {
            package,
            name,
            argc,
        } => format!("CallBuiltin {package}.{name} argc={argc}"),
        Instr::CallBuiltinId { id, argc } => format!("CallBuiltinId {id} argc={argc}"),
        Instr::StrLen => "StrLen".to_string(),
        Instr::StrLenLocal(slot) => format!("StrLenLocal {slot}"),
        Instr::StrIndexOfConst(needle) => format!("StrIndexOfConst {needle:?}"),
        Instr::StrIndexOfLocalConst { slot, needle } => {
            format!("StrIndexOfLocalConst slot={slot} {needle:?}")
        }
        Instr::StrSliceConst { start, end } => format!("StrSliceConst start={start} end={end}"),
        Instr::StrSliceLocalConst { slot, start, end } => {
            format!("StrSliceLocalConst slot={slot} start={start} end={end}")
        }
        Instr::StrContainsConst(needle) => format!("StrContainsConst {needle:?}"),
        Instr::StrContainsLocalConst { slot, needle } => {
            format!("StrContainsLocalConst slot={slot} {needle:?}")
        }
        Instr::MakeArray(n) => format!("MakeArray {n}"),
        Instr::MakeArrayRepeat(n) => format!("MakeArrayRepeat {n}"),
        Instr::ArrayGet => "ArrayGet".to_string(),
        Instr::ArrayGetLocal(slot) => format!("ArrayGetLocal {slot}"),
        Instr::ArraySet => "ArraySet".to_string(),
        Instr::ArraySetLocal(slot) => format!("ArraySetLocal {slot}"),
        Instr::ArrayIncLocal(slot) => format!("ArrayIncLocal {slot}"),
        Instr::ArraySetChain(n) => format!("ArraySetChain {n}"),
        Instr::ArrayLen => "ArrayLen".to_string(),
        Instr::MakeStruct { name, fields } => {
            format!("MakeStruct {name} fields={}", fields.join(","))
        }
        Instr::MakeStructId { id } => format!("MakeStructId {id}"),
        Instr::StructGet(field) => format!("StructGet {field}"),
        Instr::StructGetLocalSlot { slot, field_slot } => {
            format!("StructGetLocalSlot slot={slot} field_slot={field_slot}")
        }
        Instr::StructGetLocalSlotAddToLocal {
            struct_slot,
            field_slot,
            dst,
        } => format!(
            "StructGetLocalSlotAddToLocal struct_slot={struct_slot} field_slot={field_slot} dst={dst}"
        ),
        Instr::StructGetSlot(slot) => format!("StructGetSlot {slot}"),
        Instr::StructSetPath(path) => format!("StructSetPath {}", path.join(".")),
        Instr::StructSetPathSlots(path) => format!(
            "StructSetPathSlots {}",
            path.iter()
                .map(|slot| slot.to_string())
                .collect::<Vec<_>>()
                .join(".")
        ),
        Instr::Return => "Return".to_string(),
    }
}
