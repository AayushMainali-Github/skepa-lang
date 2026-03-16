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

pub fn operand_value(
    names: &ValueNames,
    operand: &Operand,
    _func: &IrFunction,
) -> Result<String, CodegenError> {
    match operand {
        Operand::Const(ConstValue::Int(v)) => Ok(v.to_string()),
        Operand::Const(ConstValue::Bool(v)) => Ok(if *v { "1".into() } else { "0".into() }),
        Operand::Temp(id) => Ok(names.temp(*id)?.to_string()),
        Operand::Local(id) => Ok(format!("%local{}", id.0)),
        Operand::Global(id) => Ok(format!("@g{}", id.0)),
        Operand::Const(_) => Err(CodegenError::Unsupported(
            "only Int and Bool constants are supported in initial LLVM lowering",
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
) -> Result<String, CodegenError> {
    match operand {
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
