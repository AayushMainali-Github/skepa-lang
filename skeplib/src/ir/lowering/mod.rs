use std::collections::HashMap;

use crate::ast::{
    AssignTarget, BinaryOp as AstBinaryOp, Expr, FnDecl, Program, Stmt, UnaryOp as AstUnaryOp,
};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::ir::{
    BinaryOp, BlockId, BranchTerminator, ConstValue, Instr, IrBuilder, IrProgram, IrType,
    IrVerifier, Operand, Terminator, UnaryOp,
};
use crate::parser::Parser;
use crate::types::TypeInfo;

pub fn compile_source(source: &str) -> Result<IrProgram, DiagnosticBag> {
    let (program, mut diags) = Parser::parse_source(source);
    if !diags.is_empty() {
        return Err(diags);
    }

    let mut lowerer = IrLowerer::new();
    let ir = lowerer.compile_program(&program);
    for diag in lowerer.diags.into_vec() {
        diags.push(diag);
    }

    if diags.is_empty() {
        match IrVerifier::verify_program(&ir) {
            Ok(()) => Ok(ir),
            Err(err) => {
                diags.error(format!("IR verification failed: {err:?}"), Span::default());
                Err(diags)
            }
        }
    } else {
        Err(diags)
    }
}

struct IrLowerer {
    builder: IrBuilder,
    diags: DiagnosticBag,
}

struct FunctionLowering {
    current_block: BlockId,
    locals: HashMap<String, crate::ir::LocalId>,
}

impl IrLowerer {
    fn new() -> Self {
        Self {
            builder: IrBuilder::new(),
            diags: DiagnosticBag::new(),
        }
    }

    fn compile_program(&mut self, program: &Program) -> IrProgram {
        let mut out = self.builder.begin_program();

        for func in &program.functions {
            if let Some(lowered) = self.compile_function(func) {
                out.functions.push(lowered);
            }
        }

        out
    }

    fn compile_function(&mut self, func: &FnDecl) -> Option<crate::ir::IrFunction> {
        let ret_ty = func
            .return_type
            .as_ref()
            .map(TypeInfo::from_ast)
            .map(|ty| IrType::from(&ty))
            .unwrap_or(IrType::Void);
        let mut out = self
            .builder
            .begin_function(func.name.clone(), ret_ty.clone());
        let mut lowering = FunctionLowering {
            current_block: out.entry,
            locals: HashMap::new(),
        };

        for param in &func.params {
            let ty_info = TypeInfo::from_ast(&param.ty);
            self.builder
                .push_param(&mut out, param.name.clone(), IrType::from(&ty_info));
            let local =
                self.builder
                    .push_local(&mut out, param.name.clone(), IrType::from(&ty_info));
            lowering.locals.insert(param.name.clone(), local);
        }

        for stmt in &func.body {
            if !self.compile_stmt(&mut out, &mut lowering, stmt) {
                return None;
            }
        }

        if matches!(
            out.blocks
                .iter()
                .find(|block| block.id == lowering.current_block)
                .map(|block| &block.terminator),
            Some(Terminator::Unreachable)
        ) {
            let terminator = if ret_ty.is_void() {
                Terminator::Return(None)
            } else {
                self.diags.error(
                    format!(
                        "IR lowering currently requires explicit return in non-void function `{}`",
                        func.name
                    ),
                    Span::default(),
                );
                return None;
            };
            self.builder
                .set_terminator(&mut out, lowering.current_block, terminator);
        }

        Some(out)
    }

    fn compile_stmt(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        stmt: &Stmt,
    ) -> bool {
        match stmt {
            Stmt::Let { name, ty, value } => {
                let rhs = match self.compile_expr(func, lowering, value) {
                    Some(value) => value,
                    None => return false,
                };
                let ir_ty = ty
                    .as_ref()
                    .map(TypeInfo::from_ast)
                    .map(|ty| IrType::from(&ty))
                    .unwrap_or_else(|| self.infer_operand_type(func, &rhs));
                let local = self.builder.push_local(func, name.clone(), ir_ty.clone());
                lowering.locals.insert(name.clone(), local);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::StoreLocal {
                        local,
                        ty: ir_ty,
                        value: rhs,
                    },
                );
                true
            }
            Stmt::Assign {
                target: AssignTarget::Ident(name),
                value,
            } => {
                let rhs = match self.compile_expr(func, lowering, value) {
                    Some(value) => value,
                    None => return false,
                };
                let Some(&local) = lowering.locals.get(name) else {
                    self.unsupported(format!("assignment to unknown local `{name}`"));
                    return false;
                };
                let ty = func
                    .locals
                    .iter()
                    .find(|entry| entry.id == local)
                    .map(|entry| entry.ty.clone())
                    .unwrap_or(IrType::Unknown);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::StoreLocal {
                        local,
                        ty,
                        value: rhs,
                    },
                );
                true
            }
            Stmt::Expr(expr) => self.compile_expr(func, lowering, expr).is_some(),
            Stmt::Return(value) => {
                let ret = match value {
                    Some(expr) => match self.compile_expr(func, lowering, expr) {
                        Some(value) => Some(value),
                        None => return false,
                    },
                    None => None,
                };
                self.builder
                    .set_terminator(func, lowering.current_block, Terminator::Return(ret));
                true
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => self.compile_if(func, lowering, cond, then_body, else_body),
            Stmt::While { cond, body } => self.compile_while(func, lowering, cond, body),
            _ => {
                self.unsupported("statement form is not in the initial IR lowering subset");
                false
            }
        }
    }

    fn compile_if(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        cond: &Expr,
        then_body: &[Stmt],
        else_body: &[Stmt],
    ) -> bool {
        let cond_value = match self.compile_expr(func, lowering, cond) {
            Some(value) => value,
            None => return false,
        };
        let then_block = self.builder.push_block(func, "if_then");
        let else_block = self.builder.push_block(func, "if_else");
        let join_block = self.builder.push_block(func, "if_join");
        self.builder.set_terminator(
            func,
            lowering.current_block,
            Terminator::Branch(BranchTerminator {
                cond: cond_value,
                then_block,
                else_block,
            }),
        );

        lowering.current_block = then_block;
        for stmt in then_body {
            if !self.compile_stmt(func, lowering, stmt) {
                return false;
            }
        }
        self.ensure_fallthrough_jump(func, lowering.current_block, join_block);

        lowering.current_block = else_block;
        for stmt in else_body {
            if !self.compile_stmt(func, lowering, stmt) {
                return false;
            }
        }
        self.ensure_fallthrough_jump(func, lowering.current_block, join_block);

        lowering.current_block = join_block;
        true
    }

    fn compile_while(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        cond: &Expr,
        body: &[Stmt],
    ) -> bool {
        let cond_block = self.builder.push_block(func, "while_cond");
        let body_block = self.builder.push_block(func, "while_body");
        let exit_block = self.builder.push_block(func, "while_exit");

        self.builder
            .set_terminator(func, lowering.current_block, Terminator::Jump(cond_block));

        lowering.current_block = cond_block;
        let cond_value = match self.compile_expr(func, lowering, cond) {
            Some(value) => value,
            None => return false,
        };
        self.builder.set_terminator(
            func,
            cond_block,
            Terminator::Branch(BranchTerminator {
                cond: cond_value,
                then_block: body_block,
                else_block: exit_block,
            }),
        );

        lowering.current_block = body_block;
        for stmt in body {
            if !self.compile_stmt(func, lowering, stmt) {
                return false;
            }
        }
        self.ensure_fallthrough_jump(func, lowering.current_block, cond_block);

        lowering.current_block = exit_block;
        true
    }

    fn compile_expr(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        expr: &Expr,
    ) -> Option<Operand> {
        match expr {
            Expr::IntLit(value) => Some(Operand::Const(ConstValue::Int(*value))),
            Expr::BoolLit(value) => Some(Operand::Const(ConstValue::Bool(*value))),
            Expr::StringLit(value) => Some(Operand::Const(ConstValue::String(value.clone()))),
            Expr::Ident(name) => lowering
                .locals
                .get(name)
                .copied()
                .map(Operand::Local)
                .or_else(|| {
                    self.unsupported(format!("reference to unresolved identifier `{name}`"));
                    None
                }),
            Expr::Group(inner) => self.compile_expr(func, lowering, inner),
            Expr::Unary { op, expr } => {
                let operand = self.compile_expr(func, lowering, expr)?;
                let ty = self.infer_operand_type(func, &operand);
                let dst = self.builder.push_temp(func, ty.clone());
                let op = match op {
                    AstUnaryOp::Neg => UnaryOp::Neg,
                    AstUnaryOp::Not => UnaryOp::Not,
                    AstUnaryOp::Pos => {
                        self.unsupported("unary operator is not in the initial IR lowering subset");
                        return None;
                    }
                };
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::Unary {
                        dst,
                        ty,
                        op,
                        operand,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::Binary { left, op, right } => {
                let left = self.compile_expr(func, lowering, left)?;
                let right = self.compile_expr(func, lowering, right)?;
                let ty = self.infer_binary_type(func, &left, op, &right);
                let dst = self.builder.push_temp(func, ty.clone());
                if let Some(op) = self.lower_binary_op(op) {
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::Binary {
                            dst,
                            ty,
                            op,
                            left,
                            right,
                        },
                    );
                } else if let Some(op) = self.lower_cmp_op(op) {
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::Compare {
                            dst,
                            op,
                            left,
                            right,
                        },
                    );
                } else {
                    self.unsupported("binary operator is not in the initial IR lowering subset");
                    return None;
                }
                Some(Operand::Temp(dst))
            }
            _ => {
                self.unsupported("expression form is not in the initial IR lowering subset");
                None
            }
        }
    }

    fn infer_operand_type(&self, func: &crate::ir::IrFunction, operand: &Operand) -> IrType {
        match operand {
            Operand::Const(ConstValue::Int(_)) => IrType::Int,
            Operand::Const(ConstValue::Float(_)) => IrType::Float,
            Operand::Const(ConstValue::Bool(_)) => IrType::Bool,
            Operand::Const(ConstValue::String(_)) => IrType::String,
            Operand::Const(ConstValue::Unit) => IrType::Void,
            Operand::Temp(id) => func
                .temps
                .iter()
                .find(|temp| temp.id == *id)
                .map(|temp| temp.ty.clone())
                .unwrap_or(IrType::Unknown),
            Operand::Local(id) => func
                .locals
                .iter()
                .find(|local| local.id == *id)
                .map(|local| local.ty.clone())
                .unwrap_or(IrType::Unknown),
            Operand::Global(_) => IrType::Unknown,
        }
    }

    fn infer_binary_type(
        &self,
        func: &crate::ir::IrFunction,
        left: &Operand,
        op: &AstBinaryOp,
        right: &Operand,
    ) -> IrType {
        if self.lower_cmp_op(op).is_some() {
            IrType::Bool
        } else {
            let left_ty = self.infer_operand_type(func, left);
            let right_ty = self.infer_operand_type(func, right);
            if left_ty == right_ty {
                left_ty
            } else {
                IrType::Unknown
            }
        }
    }

    fn lower_binary_op(&self, op: &AstBinaryOp) -> Option<BinaryOp> {
        match op {
            AstBinaryOp::Add => Some(BinaryOp::Add),
            AstBinaryOp::Sub => Some(BinaryOp::Sub),
            AstBinaryOp::Mul => Some(BinaryOp::Mul),
            AstBinaryOp::Div => Some(BinaryOp::Div),
            AstBinaryOp::Mod => Some(BinaryOp::Mod),
            AstBinaryOp::AndAnd | AstBinaryOp::OrOr => None,
            _ => None,
        }
    }

    fn lower_cmp_op(&self, op: &AstBinaryOp) -> Option<crate::ir::CmpOp> {
        match op {
            AstBinaryOp::EqEq => Some(crate::ir::CmpOp::Eq),
            AstBinaryOp::Neq => Some(crate::ir::CmpOp::Ne),
            AstBinaryOp::Lt => Some(crate::ir::CmpOp::Lt),
            AstBinaryOp::Lte => Some(crate::ir::CmpOp::Le),
            AstBinaryOp::Gt => Some(crate::ir::CmpOp::Gt),
            AstBinaryOp::Gte => Some(crate::ir::CmpOp::Ge),
            _ => None,
        }
    }

    fn ensure_fallthrough_jump(
        &self,
        func: &mut crate::ir::IrFunction,
        from: BlockId,
        to: BlockId,
    ) {
        if matches!(
            func.blocks
                .iter()
                .find(|block| block.id == from)
                .map(|block| &block.terminator),
            Some(Terminator::Unreachable)
        ) {
            self.builder
                .set_terminator(func, from, Terminator::Jump(to));
        }
    }

    fn unsupported(&mut self, message: impl Into<String>) {
        self.diags.error(message, Span::default());
    }
}
