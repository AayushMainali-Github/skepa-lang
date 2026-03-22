use std::collections::HashMap;

use crate::ir::{
    BinaryOp, BranchTerminator, CmpOp, ConstValue, Instr, IrProgram, LogicOp, Operand,
    Terminator, UnaryOp,
};

pub fn run(program: &mut IrProgram) -> bool {
    let mut changed = false;

    for func in &mut program.functions {
        for block in &mut func.blocks {
            let mut consts = HashMap::new();
            for instr in &mut block.instrs {
                changed |= fold_instr(instr, &mut consts);
            }
            changed |= fold_terminator(&mut block.terminator, &consts);
        }
    }

    changed
}

fn fold_instr(instr: &mut Instr, consts: &mut HashMap<crate::ir::TempId, ConstValue>) -> bool {
    match instr {
        Instr::Const { dst, value, .. } => {
            consts.insert(*dst, value.clone());
            false
        }
        Instr::Copy { dst, src, ty } => {
            if let Some(value) = resolve_const(src, consts) {
                let dst_id = *dst;
                let out_ty = ty.clone();
                *instr = Instr::Const {
                    dst: dst_id,
                    ty: out_ty,
                    value: value.clone(),
                };
                consts.insert(dst_id, value);
                true
            } else {
                consts.remove(dst);
                false
            }
        }
        Instr::Unary {
            dst,
            ty,
            op,
            operand,
        } => {
            if let Some(value) = resolve_const(operand, consts)
                && let Some(folded) = eval_unary(*op, &value)
            {
                let dst_id = *dst;
                let out_ty = ty.clone();
                *instr = Instr::Const {
                    dst: dst_id,
                    ty: out_ty,
                    value: folded.clone(),
                };
                consts.insert(dst_id, folded);
                return true;
            }
            consts.remove(dst);
            false
        }
        Instr::Binary {
            dst,
            ty,
            op,
            left,
            right,
        } => {
            if let (Some(left), Some(right)) =
                (resolve_const(left, consts), resolve_const(right, consts))
                && let Some(folded) = eval_binary(*op, &left, &right)
            {
                let dst_id = *dst;
                let out_ty = ty.clone();
                *instr = Instr::Const {
                    dst: dst_id,
                    ty: out_ty,
                    value: folded.clone(),
                };
                consts.insert(dst_id, folded);
                return true;
            }
            consts.remove(dst);
            false
        }
        Instr::Compare {
            dst,
            op,
            left,
            right,
        } => {
            if let (Some(left), Some(right)) =
                (resolve_const(left, consts), resolve_const(right, consts))
                && let Some(folded) = eval_compare(*op, &left, &right)
            {
                let dst_id = *dst;
                *instr = Instr::Const {
                    dst: dst_id,
                    ty: crate::ir::IrType::Bool,
                    value: ConstValue::Bool(folded),
                };
                consts.insert(dst_id, ConstValue::Bool(folded));
                return true;
            }
            consts.remove(dst);
            false
        }
        Instr::Logic {
            dst,
            op,
            left,
            right,
        } => {
            if let (Some(left), Some(right)) =
                (resolve_const(left, consts), resolve_const(right, consts))
                && let Some(folded) = eval_logic(*op, &left, &right)
            {
                let dst_id = *dst;
                *instr = Instr::Const {
                    dst: dst_id,
                    ty: crate::ir::IrType::Bool,
                    value: ConstValue::Bool(folded),
                };
                consts.insert(dst_id, ConstValue::Bool(folded));
                return true;
            }
            consts.remove(dst);
            false
        }
        Instr::LoadGlobal { dst, .. }
        | Instr::LoadLocal { dst, .. }
        | Instr::MakeArray { dst, .. }
        | Instr::MakeArrayRepeat { dst, .. }
        | Instr::VecNew { dst, .. }
        | Instr::VecLen { dst, .. }
        | Instr::ArrayGet { dst, .. }
        | Instr::VecGet { dst, .. }
        | Instr::VecDelete { dst, .. }
        | Instr::MakeStruct { dst, .. }
        | Instr::StructGet { dst, .. }
        | Instr::MakeClosure { dst, .. } => {
            consts.remove(dst);
            false
        }
        Instr::StoreGlobal { .. }
        | Instr::StoreLocal { .. }
        | Instr::ArraySet { .. }
        | Instr::VecPush { .. }
        | Instr::VecSet { .. }
        | Instr::StructSet { .. }
        | Instr::CallDirect { .. }
        | Instr::CallIndirect { .. } => false,
        Instr::CallBuiltin { .. } => false,
    }
}

fn fold_terminator(
    terminator: &mut Terminator,
    consts: &HashMap<crate::ir::TempId, ConstValue>,
) -> bool {
    match terminator {
        Terminator::Branch(BranchTerminator {
            cond,
            then_block,
            else_block,
        }) => {
            if let Some(ConstValue::Bool(cond)) = resolve_const(cond, consts) {
                *terminator = Terminator::Jump(if cond { *then_block } else { *else_block });
                true
            } else {
                false
            }
        }
        Terminator::Jump(_)
        | Terminator::Return(_)
        | Terminator::Panic { .. }
        | Terminator::Unreachable => false,
    }
}

fn resolve_const(
    operand: &Operand,
    consts: &HashMap<crate::ir::TempId, ConstValue>,
) -> Option<ConstValue> {
    match operand {
        Operand::Const(value) => Some(value.clone()),
        Operand::Temp(id) => consts.get(id).cloned(),
        Operand::Local(_) | Operand::Global(_) => None,
    }
}

fn eval_unary(op: UnaryOp, value: &ConstValue) -> Option<ConstValue> {
    match (op, value) {
        (UnaryOp::Neg, ConstValue::Int(v)) => Some(ConstValue::Int(-v)),
        (UnaryOp::Neg, ConstValue::Float(v)) => Some(ConstValue::Float(-v)),
        (UnaryOp::Not, ConstValue::Bool(v)) => Some(ConstValue::Bool(!v)),
        _ => None,
    }
}

fn eval_binary(op: BinaryOp, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
    match (op, left, right) {
        (BinaryOp::Add, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Int(a + b)),
        (BinaryOp::Sub, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Int(a - b)),
        (BinaryOp::Mul, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Int(a * b)),
        (BinaryOp::Div, ConstValue::Int(_), ConstValue::Int(0)) => None,
        (BinaryOp::Div, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Int(a / b)),
        (BinaryOp::Mod, ConstValue::Int(_), ConstValue::Int(0)) => None,
        (BinaryOp::Mod, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Int(a % b)),
        (BinaryOp::Add, ConstValue::Float(a), ConstValue::Float(b)) => {
            Some(ConstValue::Float(a + b))
        }
        (BinaryOp::Sub, ConstValue::Float(a), ConstValue::Float(b)) => {
            Some(ConstValue::Float(a - b))
        }
        (BinaryOp::Mul, ConstValue::Float(a), ConstValue::Float(b)) => {
            Some(ConstValue::Float(a * b))
        }
        (BinaryOp::Div, ConstValue::Float(a), ConstValue::Float(b)) => {
            Some(ConstValue::Float(a / b))
        }
        (BinaryOp::Add, ConstValue::String(a), ConstValue::String(b)) => {
            Some(ConstValue::String(format!("{a}{b}")))
        }
        _ => None,
    }
}

fn eval_compare(op: CmpOp, left: &ConstValue, right: &ConstValue) -> Option<bool> {
    match (left, right) {
        (ConstValue::Int(a), ConstValue::Int(b)) => Some(match op {
            CmpOp::Eq => a == b,
            CmpOp::Ne => a != b,
            CmpOp::Lt => a < b,
            CmpOp::Le => a <= b,
            CmpOp::Gt => a > b,
            CmpOp::Ge => a >= b,
        }),
        (ConstValue::Float(a), ConstValue::Float(b)) => Some(match op {
            CmpOp::Eq => a == b,
            CmpOp::Ne => a != b,
            CmpOp::Lt => a < b,
            CmpOp::Le => a <= b,
            CmpOp::Gt => a > b,
            CmpOp::Ge => a >= b,
        }),
        (ConstValue::Bool(a), ConstValue::Bool(b)) => Some(match op {
            CmpOp::Eq => a == b,
            CmpOp::Ne => a != b,
            _ => return None,
        }),
        (ConstValue::String(a), ConstValue::String(b)) => Some(match op {
            CmpOp::Eq => a == b,
            CmpOp::Ne => a != b,
            _ => return None,
        }),
        _ => None,
    }
}

fn eval_logic(op: LogicOp, left: &ConstValue, right: &ConstValue) -> Option<bool> {
    match (op, left, right) {
        (LogicOp::And, ConstValue::Bool(a), ConstValue::Bool(b)) => Some(*a && *b),
        (LogicOp::Or, ConstValue::Bool(a), ConstValue::Bool(b)) => Some(*a || *b),
        _ => None,
    }
}
