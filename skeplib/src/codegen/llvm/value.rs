use std::collections::HashMap;

use crate::codegen::CodegenError;
use crate::codegen::llvm::types::llvm_ty;
use crate::ir::{ConstValue, IrFunction, Operand, TempId};

pub struct ValueNames {
    temp_names: HashMap<TempId, String>,
}

impl ValueNames {
    pub fn new(func: &IrFunction) -> Self {
        let temp_names = func
            .temps
            .iter()
            .map(|temp| (temp.id, format!("%t{}", temp.id.0)))
            .collect();
        Self { temp_names }
    }

    pub fn temp(&self, temp: TempId) -> Result<&str, CodegenError> {
        self.temp_names
            .get(&temp)
            .map(String::as_str)
            .ok_or_else(|| CodegenError::InvalidIr(format!("unknown temp {:?}", temp)))
    }
}

pub fn llvm_symbol(name: &str) -> String {
    let escaped = name.replace('\\', "\\5C").replace('"', "\\22");
    format!("@\"{escaped}\"")
}

pub fn operand_value(
    names: &ValueNames,
    operand: &Operand,
    _func: &IrFunction,
) -> Result<String, CodegenError> {
    match operand {
        Operand::Const(ConstValue::Int(v)) => Ok(v.to_string()),
        Operand::Const(ConstValue::Float(v)) => Ok(llvm_float_literal(*v)),
        Operand::Const(ConstValue::Bool(v)) => Ok(if *v { "1".into() } else { "0".into() }),
        Operand::Temp(id) => Ok(names.temp(*id)?.to_string()),
        Operand::Local(id) => Ok(format!("%local{}", id.0)),
        Operand::Global(id) => Ok(format!("@g{}", id.0)),
        Operand::Const(_) => Err(CodegenError::Unsupported(
            "string constants require operand_load in LLVM lowering",
        )),
    }
}

pub fn operand_load(
    names: &ValueNames,
    operand: &Operand,
    func: &IrFunction,
    lines: &mut Vec<String>,
    counter: &mut usize,
    expected_ty: &crate::ir::IrType,
    string_literals: &HashMap<String, String>,
) -> Result<String, CodegenError> {
    match operand {
        Operand::Const(ConstValue::String(value)) => {
            let name = string_literals.get(value).ok_or_else(|| {
                CodegenError::InvalidIr("missing string literal declaration".into())
            })?;
            let gep = format!("%v{counter}");
            *counter += 1;
            let bytes = value.len() + 1;
            lines.push(format!(
                "  {gep} = getelementptr inbounds [{bytes} x i8], ptr {name}, i64 0, i64 0"
            ));
            let string = format!("%v{counter}");
            *counter += 1;
            lines.push(format!(
                "  {string} = call ptr @skp_rt_string_from_utf8(ptr {gep}, i64 {})",
                value.len()
            ));
            Ok(string)
        }
        Operand::Local(id) => {
            let name = format!("%v{counter}");
            *counter += 1;
            lines.push(format!(
                "  {name} = load {}, ptr %local{}, align 8",
                llvm_ty(expected_ty)?,
                id.0
            ));
            Ok(name)
        }
        Operand::Global(id) => {
            let name = format!("%v{counter}");
            *counter += 1;
            lines.push(format!(
                "  {name} = load {}, ptr @g{}, align 8",
                llvm_ty(expected_ty)?,
                id.0
            ));
            Ok(name)
        }
        _ => operand_value(names, operand, func),
    }
}

pub fn raw_string_ptr(
    value: &str,
    lines: &mut Vec<String>,
    counter: &mut usize,
    string_literals: &HashMap<String, String>,
) -> Result<String, CodegenError> {
    let name = string_literals
        .get(value)
        .ok_or_else(|| CodegenError::InvalidIr("missing string literal declaration".into()))?;
    let gep = format!("%v{counter}");
    *counter += 1;
    let bytes = value.len() + 1;
    lines.push(format!(
        "  {gep} = getelementptr inbounds [{bytes} x i8], ptr {name}, i64 0, i64 0"
    ));
    Ok(gep)
}

pub fn llvm_float_literal(value: f64) -> String {
    let literal = value.to_string();
    if literal.contains(['.', 'e', 'E']) {
        literal
    } else {
        format!("{literal}.0")
    }
}
