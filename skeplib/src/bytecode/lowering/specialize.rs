use crate::ast::{BinaryOp, Expr};

use super::context::{Compiler, FnCtx, InlinableFunction, PrimitiveType, SpecializedArrayAssign};
use super::{Instr, IntLocalConstOp};

impl Compiler {
    pub(super) fn specialized_cond_jump_false(cond: &Expr, ctx: &FnCtx) -> Option<Instr> {
        let Expr::Binary { left, op, right } = cond else {
            return None;
        };
        let Expr::Ident(name) = &**left else {
            return None;
        };
        let Expr::IntLit(rhs) = &**right else {
            return None;
        };
        if *op != BinaryOp::Lt {
            return None;
        }
        let slot = ctx.lookup(name)?;
        Some(Instr::JumpIfLocalLtConst {
            slot,
            rhs: *rhs,
            target: usize::MAX,
        })
    }

    pub(super) fn specialized_local_assign(value: &Expr, ctx: &FnCtx, dst: usize) -> Option<Instr> {
        let Expr::Binary { left, op, right } = value else {
            return None;
        };
        match (&**left, op, &**right) {
            (Expr::Ident(name), BinaryOp::Add, Expr::IntLit(rhs)) => {
                let slot = ctx.lookup(name)?;
                if slot == dst {
                    Some(Instr::AddConstToLocal { slot, rhs: *rhs })
                } else {
                    None
                }
            }
            (Expr::IntLit(rhs), BinaryOp::Add, Expr::Ident(name)) => {
                let slot = ctx.lookup(name)?;
                if slot == dst {
                    Some(Instr::AddConstToLocal { slot, rhs: *rhs })
                } else {
                    None
                }
            }
            (Expr::Ident(left_name), BinaryOp::Add, Expr::Ident(right_name)) => {
                let left_slot = ctx.lookup(left_name)?;
                let right_slot = ctx.lookup(right_name)?;
                if left_slot == dst {
                    Some(Instr::AddLocalToLocal {
                        dst: left_slot,
                        src: right_slot,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(super) fn specialized_local_stack_assign<'a>(
        value: &'a Expr,
        ctx: &FnCtx,
        dst_name: &str,
        dst: usize,
    ) -> Option<(&'a Expr, Instr)> {
        let Expr::Binary { left, op, right } = value else {
            return None;
        };
        let Expr::Ident(name) = &**left else {
            return None;
        };
        if name != dst_name || ctx.primitive_type(name) != Some(PrimitiveType::Int) {
            return None;
        }
        let op = match op {
            BinaryOp::Add => IntLocalConstOp::Add,
            BinaryOp::Sub => IntLocalConstOp::Sub,
            BinaryOp::Mul => IntLocalConstOp::Mul,
            BinaryOp::Div => IntLocalConstOp::Div,
            BinaryOp::Mod => IntLocalConstOp::Mod,
            _ => return None,
        };
        Some((right, Instr::IntStackOpToLocal { slot: dst, op }))
    }

    pub(super) fn specialized_local_value_to_local(
        &self,
        value: &Expr,
        ctx: &FnCtx,
        dst: usize,
    ) -> Option<Instr> {
        if let Some(instr) = self.specialized_inlined_function_call_to_local(value, ctx, dst) {
            return Some(instr);
        }
        let Expr::Binary { left, op, right } = value else {
            return None;
        };
        if let Some((src, rhs, op)) = Self::specialized_local_const_source(op, left, right, ctx) {
            return Some(Instr::IntLocalConstOpToLocal { src, dst, op, rhs });
        }
        let Expr::Ident(left_name) = &**left else {
            return None;
        };
        let Expr::Ident(right_name) = &**right else {
            return None;
        };
        if ctx.primitive_type(left_name) != Some(PrimitiveType::Int)
            || ctx.primitive_type(right_name) != Some(PrimitiveType::Int)
        {
            return None;
        }
        let lhs = ctx.lookup(left_name)?;
        let rhs = ctx.lookup(right_name)?;
        let op = match op {
            BinaryOp::Add => IntLocalConstOp::Add,
            BinaryOp::Sub => IntLocalConstOp::Sub,
            BinaryOp::Mul => IntLocalConstOp::Mul,
            BinaryOp::Div => IntLocalConstOp::Div,
            BinaryOp::Mod => IntLocalConstOp::Mod,
            _ => return None,
        };
        Some(Instr::IntLocalLocalOpToLocal { lhs, rhs, dst, op })
    }

    pub(super) fn specialized_local_array_assign(
        root: &str,
        index: &Expr,
        value: &Expr,
    ) -> Option<SpecializedArrayAssign> {
        let Expr::Binary { left, op, right } = value else {
            return None;
        };
        if *op != BinaryOp::Add {
            return None;
        }
        match (&**left, &**right) {
            (Expr::IntLit(1), other) | (other, Expr::IntLit(1)) => {
                if Self::is_same_local_index_expr(root, index, other) {
                    return Some(SpecializedArrayAssign::IncLocal);
                }
            }
            _ => {}
        }
        None
    }

    fn is_same_local_index_expr(root: &str, index: &Expr, expr: &Expr) -> bool {
        let Expr::Index {
            base,
            index: other_index,
        } = expr
        else {
            return false;
        };
        let Expr::Ident(name) = &**base else {
            return false;
        };
        name == root && **other_index == *index
    }

    pub(super) fn specialized_local_const_expr(
        op: &BinaryOp,
        left: &Expr,
        right: &Expr,
        ctx: &FnCtx,
    ) -> Option<Instr> {
        let (slot, rhs, op) = Self::specialized_local_const_source(op, left, right, ctx)?;
        Some(Instr::IntLocalConstOp { slot, op, rhs })
    }

    pub(super) fn specialized_local_const_source(
        op: &BinaryOp,
        left: &Expr,
        right: &Expr,
        ctx: &FnCtx,
    ) -> Option<(usize, i64, IntLocalConstOp)> {
        let Expr::Ident(name) = left else {
            return None;
        };
        let Expr::IntLit(rhs) = right else {
            return None;
        };
        if ctx.primitive_type(name) != Some(PrimitiveType::Int) {
            return None;
        }
        let slot = ctx.lookup(name)?;
        let op = match op {
            BinaryOp::Sub => IntLocalConstOp::Sub,
            BinaryOp::Mul => IntLocalConstOp::Mul,
            BinaryOp::Div => IntLocalConstOp::Div,
            BinaryOp::Mod => IntLocalConstOp::Mod,
            _ => return None,
        };
        Some((slot, *rhs, op))
    }

    fn specialized_inlined_function_call_to_local(
        &self,
        value: &Expr,
        ctx: &FnCtx,
        dst: usize,
    ) -> Option<Instr> {
        let Expr::Call { callee, args } = value else {
            return None;
        };
        if args.len() != 1 {
            return None;
        }
        let Expr::Ident(arg_name) = &args[0] else {
            return None;
        };
        if ctx.primitive_type(arg_name) != Some(PrimitiveType::Int) {
            return None;
        }
        let src = ctx.lookup(arg_name)?;
        let callee_name = match &**callee {
            Expr::Ident(name) => self
                .local_fn_qualified
                .get(name)
                .cloned()
                .or_else(|| self.direct_import_calls.get(name).cloned())?,
            Expr::Path(parts) => parts.join("."),
            _ => return None,
        };
        match self.inlinable_functions.get(&callee_name).copied()? {
            InlinableFunction::AddConst(rhs) if src == dst => {
                Some(Instr::AddConstToLocal { slot: src, rhs })
            }
            InlinableFunction::AddConst(rhs) => Some(Instr::IntLocalConstOpToLocal {
                src,
                dst,
                op: IntLocalConstOp::Add,
                rhs,
            }),
        }
    }

    pub(super) fn specialized_stack_const_expr<'a>(
        op: &BinaryOp,
        left: &'a Expr,
        right: &Expr,
        ctx: &FnCtx,
    ) -> Option<(&'a Expr, Instr)> {
        let Expr::IntLit(rhs) = right else {
            return None;
        };
        if Self::infer_expr_primitive_type(left, ctx) != Some(PrimitiveType::Int) {
            return None;
        }
        let op = match op {
            BinaryOp::Sub => IntLocalConstOp::Sub,
            BinaryOp::Mul => IntLocalConstOp::Mul,
            BinaryOp::Div => IntLocalConstOp::Div,
            BinaryOp::Mod => IntLocalConstOp::Mod,
            _ => return None,
        };
        Some((left, Instr::IntStackConstOp { op, rhs: *rhs }))
    }

    pub(super) fn specialized_local_local_expr(
        op: &BinaryOp,
        left: &Expr,
        right: &Expr,
        ctx: &FnCtx,
    ) -> Option<Instr> {
        let Expr::Ident(left_name) = left else {
            return None;
        };
        let Expr::Ident(right_name) = right else {
            return None;
        };
        if ctx.primitive_type(left_name) != Some(PrimitiveType::Int)
            || ctx.primitive_type(right_name) != Some(PrimitiveType::Int)
        {
            return None;
        }
        let lhs = ctx.lookup(left_name)?;
        let rhs = ctx.lookup(right_name)?;
        let op = match op {
            BinaryOp::Add => IntLocalConstOp::Add,
            BinaryOp::Sub => IntLocalConstOp::Sub,
            BinaryOp::Mul => IntLocalConstOp::Mul,
            BinaryOp::Div => IntLocalConstOp::Div,
            BinaryOp::Mod => IntLocalConstOp::Mod,
            _ => return None,
        };
        Some(Instr::IntLocalLocalOp { lhs, rhs, op })
    }
}
