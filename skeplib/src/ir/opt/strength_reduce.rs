use crate::ir::{BinaryOp, ConstValue, Instr, IrProgram, Operand};

pub fn run(program: &mut IrProgram) -> bool {
    let mut changed = false;

    for func in &mut program.functions {
        for block in &mut func.blocks {
            for instr in &mut block.instrs {
                let replacement = match instr {
                    Instr::Binary {
                        dst,
                        ty,
                        op: BinaryOp::Add,
                        left,
                        right,
                    } => reduce_add(*dst, ty.clone(), left.clone(), right.clone()),
                    Instr::Binary {
                        dst,
                        ty,
                        op: BinaryOp::Sub,
                        left,
                        right,
                    } => reduce_sub(*dst, ty.clone(), left.clone(), right.clone()),
                    Instr::Binary {
                        dst,
                        ty,
                        op: BinaryOp::Mul,
                        left,
                        right,
                    } => reduce_mul(*dst, ty.clone(), left.clone(), right.clone()),
                    Instr::Binary {
                        dst,
                        ty,
                        op: BinaryOp::Div,
                        left,
                        right,
                    } => reduce_div(*dst, ty.clone(), left.clone(), right.clone()),
                    Instr::Binary {
                        dst,
                        ty,
                        op: BinaryOp::Mod,
                        left,
                        right,
                    } => reduce_mod(*dst, ty.clone(), left.clone(), right.clone()),
                    _ => None,
                };

                if let Some(new_instr) = replacement {
                    *instr = new_instr;
                    changed = true;
                }
            }
        }
    }

    changed
}

fn reduce_add(
    dst: crate::ir::TempId,
    ty: crate::ir::IrType,
    left: Operand,
    right: Operand,
) -> Option<Instr> {
    match (&left, &right) {
        // Float ±0.0 is not a safe identity because of signed zero.
        (_, Operand::Const(ConstValue::Int(0))) => Some(Instr::Copy { dst, ty, src: left }),
        (Operand::Const(ConstValue::Int(0)), _) => Some(Instr::Copy {
            dst,
            ty,
            src: right,
        }),
        _ => None,
    }
}

fn reduce_sub(
    dst: crate::ir::TempId,
    ty: crate::ir::IrType,
    left: Operand,
    right: Operand,
) -> Option<Instr> {
    match right {
        Operand::Const(ConstValue::Int(0)) => Some(Instr::Copy { dst, ty, src: left }),
        _ => None,
    }
}

fn reduce_mul(
    dst: crate::ir::TempId,
    ty: crate::ir::IrType,
    left: Operand,
    right: Operand,
) -> Option<Instr> {
    match (&left, &right) {
        (_, Operand::Const(ConstValue::Int(1))) | (_, Operand::Const(ConstValue::Float(1.0))) => {
            Some(Instr::Copy { dst, ty, src: left })
        }
        (Operand::Const(ConstValue::Int(1)), _) | (Operand::Const(ConstValue::Float(1.0)), _) => {
            Some(Instr::Copy {
                dst,
                ty,
                src: right,
            })
        }
        (_, Operand::Const(ConstValue::Int(2))) | (Operand::Const(ConstValue::Int(2)), _) => {
            let src = if matches!(right, Operand::Const(ConstValue::Int(2))) {
                left.clone()
            } else {
                right.clone()
            };
            Some(Instr::Binary {
                dst,
                ty,
                op: BinaryOp::Add,
                left: src.clone(),
                right: src,
            })
        }
        _ => None,
    }
}

fn reduce_div(
    dst: crate::ir::TempId,
    ty: crate::ir::IrType,
    left: Operand,
    right: Operand,
) -> Option<Instr> {
    match right {
        Operand::Const(ConstValue::Int(1)) | Operand::Const(ConstValue::Float(1.0)) => {
            Some(Instr::Copy { dst, ty, src: left })
        }
        _ => None,
    }
}

fn reduce_mod(
    dst: crate::ir::TempId,
    ty: crate::ir::IrType,
    _left: Operand,
    right: Operand,
) -> Option<Instr> {
    match right {
        Operand::Const(ConstValue::Int(1)) => Some(Instr::Const {
            dst,
            ty,
            value: ConstValue::Int(0),
        }),
        _ => None,
    }
}
