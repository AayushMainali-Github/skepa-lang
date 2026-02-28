use super::{BytecodeModule, Instr, Value};

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
        Value::Struct { name, fields } => format!("Struct({name}, fields={})", fields.len()),
        Value::Unit => "Unit".to_string(),
    }
}

fn fmt_instr(i: &Instr) -> String {
    match i {
        Instr::LoadConst(v) => format!("LoadConst {}", fmt_value(v)),
        Instr::LoadLocal(s) => format!("LoadLocal {s}"),
        Instr::StoreLocal(s) => format!("StoreLocal {s}"),
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
        Instr::Call { name, argc } => format!("Call {name} argc={argc}"),
        Instr::CallIdx { idx, argc } => format!("CallIdx {idx} argc={argc}"),
        Instr::CallValue { argc } => format!("CallValue argc={argc}"),
        Instr::CallMethod { name, argc } => format!("CallMethod {name} argc={argc}"),
        Instr::CallBuiltin {
            package,
            name,
            argc,
        } => format!("CallBuiltin {package}.{name} argc={argc}"),
        Instr::MakeArray(n) => format!("MakeArray {n}"),
        Instr::MakeArrayRepeat(n) => format!("MakeArrayRepeat {n}"),
        Instr::ArrayGet => "ArrayGet".to_string(),
        Instr::ArraySet => "ArraySet".to_string(),
        Instr::ArraySetChain(n) => format!("ArraySetChain {n}"),
        Instr::ArrayLen => "ArrayLen".to_string(),
        Instr::MakeStruct { name, fields } => {
            format!("MakeStruct {name} fields={}", fields.join(","))
        }
        Instr::StructGet(field) => format!("StructGet {field}"),
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
