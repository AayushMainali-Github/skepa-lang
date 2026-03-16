use std::collections::HashMap;

use crate::ast::{BinaryOp as AstBinaryOp, Expr, Stmt, UnaryOp as AstUnaryOp};
use crate::diagnostic::Span;
use crate::ir::{
    BinaryOp, BlockId, BranchTerminator, ConstValue, Instr, IrType, Operand, Terminator, UnaryOp,
};
use crate::types::TypeInfo;

use super::context::{FunctionLowering, IrLowerer};

impl IrLowerer {
    pub(super) fn compile_expr(
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
                    self.imported_global_names
                        .get(name)
                        .and_then(|qualified| self.globals.get(qualified))
                        .map(|(id, _)| Operand::Global(*id))
                })
                .or_else(|| {
                    self.globals
                        .get(name)
                        .or_else(|| self.globals.get(&self.qualify_name(name)))
                        .map(|(id, _)| Operand::Global(*id))
                })
                .or_else(|| self.function_value(func, lowering.current_block, name))
                .or_else(|| {
                    self.unsupported(format!("reference to unresolved identifier `{name}`"));
                    None
                }),
            Expr::Path(parts) => {
                let name = parts.join(".");
                if let Some(qualified) = self.imported_global_names.get(&name)
                    && let Some((id, _)) = self.globals.get(qualified)
                {
                    return Some(Operand::Global(*id));
                }
                self.globals
                    .get(&name)
                    .or_else(|| self.globals.get(&self.qualify_name(&name)))
                    .map(|(id, _)| Operand::Global(*id))
                    .or_else(|| {
                        self.unsupported(format!(
                            "path `{name}` is not in the initial IR lowering subset"
                        ));
                        None
                    })
            }
            Expr::Field { base, field } => {
                let base = self.compile_expr(func, lowering, base)?;
                let ty = self.field_type(func, &base, field);
                let field_ref = self.resolve_field_ref(func, &base, field);
                let dst = self.builder.push_temp(func, ty.clone());
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::StructGet {
                        dst,
                        ty,
                        base,
                        field: field_ref,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::ArrayLit(items) => {
                let mut lowered_items = Vec::with_capacity(items.len());
                for item in items {
                    lowered_items.push(self.compile_expr(func, lowering, item)?);
                }
                let elem_ty = lowered_items
                    .first()
                    .map(|item| self.infer_operand_type(func, item))
                    .unwrap_or(IrType::Unknown);
                let ty = IrType::Array {
                    elem: Box::new(elem_ty.clone()),
                    size: lowered_items.len(),
                };
                let dst = self.builder.push_temp(func, ty);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::MakeArray {
                        dst,
                        elem_ty,
                        items: lowered_items,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::ArrayRepeat { value, size } => {
                let value = self.compile_expr(func, lowering, value)?;
                let elem_ty = self.infer_operand_type(func, &value);
                let ty = IrType::Array {
                    elem: Box::new(elem_ty.clone()),
                    size: *size,
                };
                let dst = self.builder.push_temp(func, ty);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::MakeArrayRepeat {
                        dst,
                        elem_ty,
                        value,
                        size: *size,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::StructLit { name, fields } => {
                let runtime_name = self.resolve_struct_runtime_name(name);
                let Some((struct_id, struct_fields)) = self.structs.get(&runtime_name).cloned()
                else {
                    self.unsupported(format!("unknown struct `{name}` in IR lowering"));
                    return None;
                };
                let mut ordered = Vec::with_capacity(struct_fields.len());
                for declared in &struct_fields {
                    let Some((_, expr)) = fields
                        .iter()
                        .find(|(field_name, _)| field_name == &declared.name)
                    else {
                        self.unsupported(format!(
                            "missing field `{}` in struct literal `{name}`",
                            declared.name
                        ));
                        return None;
                    };
                    ordered.push(self.compile_expr(func, lowering, expr)?);
                }
                let dst = self
                    .builder
                    .push_temp(func, IrType::Named(runtime_name.clone()));
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::MakeStruct {
                        dst,
                        struct_id,
                        fields: ordered,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::FnLit {
                params,
                return_type,
                body,
            } => self.compile_fn_lit(func, lowering.current_block, params, return_type, body),
            Expr::Index { base, index } => {
                let array = self.compile_expr(func, lowering, base)?;
                let index = self.compile_expr(func, lowering, index)?;
                let elem_ty = self.array_element_type(func, &array);
                let dst = self.builder.push_temp(func, elem_ty.clone());
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::ArrayGet {
                        dst,
                        elem_ty,
                        array,
                        index,
                    },
                );
                Some(Operand::Temp(dst))
            }
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
                if matches!(op, AstBinaryOp::AndAnd | AstBinaryOp::OrOr) {
                    return self.compile_short_circuit(func, lowering, left, op, right);
                }
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
            Expr::Call { callee, args } => self.compile_call(func, lowering, callee, args),
            _ => {
                self.unsupported("expression form is not in the initial IR lowering subset");
                None
            }
        }
    }

    fn compile_short_circuit(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        left: &Expr,
        op: &AstBinaryOp,
        right: &Expr,
    ) -> Option<Operand> {
        let left_value = self.compile_expr(func, lowering, left)?;
        let result_local = self.builder.push_local(
            func,
            format!("__sc{}", lowering.scratch_counter),
            IrType::Bool,
        );
        lowering.scratch_counter += 1;

        let rhs_block = self.builder.push_block(func, "sc_rhs");
        let short_block = self.builder.push_block(func, "sc_short");
        let join_block = self.builder.push_block(func, "sc_join");

        let (then_block, else_block, short_value) = match op {
            AstBinaryOp::AndAnd => (rhs_block, short_block, false),
            AstBinaryOp::OrOr => (short_block, rhs_block, true),
            _ => return None,
        };

        self.builder.set_terminator(
            func,
            lowering.current_block,
            Terminator::Branch(BranchTerminator {
                cond: left_value,
                then_block,
                else_block,
            }),
        );

        self.builder.push_instr(
            func,
            short_block,
            Instr::StoreLocal {
                local: result_local,
                ty: IrType::Bool,
                value: Operand::Const(ConstValue::Bool(short_value)),
            },
        );
        self.builder
            .set_terminator(func, short_block, Terminator::Jump(join_block));

        lowering.current_block = rhs_block;
        let right_value = self.compile_expr(func, lowering, right)?;
        self.builder.push_instr(
            func,
            rhs_block,
            Instr::StoreLocal {
                local: result_local,
                ty: IrType::Bool,
                value: right_value,
            },
        );
        self.builder
            .set_terminator(func, rhs_block, Terminator::Jump(join_block));

        lowering.current_block = join_block;
        Some(Operand::Local(result_local))
    }

    pub(super) fn infer_operand_type(
        &self,
        func: &crate::ir::IrFunction,
        operand: &Operand,
    ) -> IrType {
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

    pub(super) fn array_element_type(
        &self,
        func: &crate::ir::IrFunction,
        operand: &Operand,
    ) -> IrType {
        match self.infer_operand_type(func, operand) {
            IrType::Array { elem, .. } => *elem,
            IrType::Vec { elem } => *elem,
            _ => IrType::Unknown,
        }
    }

    pub(super) fn field_type(
        &self,
        func: &crate::ir::IrFunction,
        base: &Operand,
        field: &str,
    ) -> IrType {
        let IrType::Named(struct_name) = self.infer_operand_type(func, base) else {
            return IrType::Unknown;
        };
        self.structs
            .get(&struct_name)
            .and_then(|(_, fields)| fields.iter().find(|entry| entry.name == field))
            .map(|entry| entry.ty.clone())
            .unwrap_or(IrType::Unknown)
    }

    pub(super) fn resolve_field_ref(
        &self,
        func: &crate::ir::IrFunction,
        base: &Operand,
        field: &str,
    ) -> crate::ir::FieldRef {
        let index = match self.infer_operand_type(func, base) {
            IrType::Named(struct_name) => self
                .structs
                .get(&struct_name)
                .and_then(|(_, fields)| fields.iter().position(|entry| entry.name == field))
                .unwrap_or(usize::MAX),
            _ => usize::MAX,
        };
        crate::ir::FieldRef {
            index,
            name: field.to_string(),
        }
    }

    fn compile_call(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        callee: &Expr,
        args: &[Expr],
    ) -> Option<Operand> {
        let mut lowered_args = Vec::with_capacity(args.len());
        for arg in args {
            lowered_args.push(self.compile_expr(func, lowering, arg)?);
        }

        match callee {
            Expr::Ident(name) => {
                let direct_name = self
                    .direct_import_calls
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| self.qualify_name(name));
                if let Some((function, ret_ty)) = self.functions.get(&direct_name).cloned() {
                    let dst = if ret_ty.is_void() {
                        None
                    } else {
                        Some(self.builder.push_temp(func, ret_ty.clone()))
                    };
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::CallDirect {
                            dst,
                            ret_ty: ret_ty.clone(),
                            function,
                            args: lowered_args,
                        },
                    );
                    OkOperand::from_call_result(dst)
                } else {
                    let callee = self.compile_expr(func, lowering, callee)?;
                    let dst = self.builder.push_temp(func, IrType::Unknown);
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::CallIndirect {
                            dst: Some(dst),
                            ret_ty: IrType::Unknown,
                            callee,
                            args: lowered_args,
                        },
                    );
                    Some(Operand::Temp(dst))
                }
            }
            Expr::Field { base, field } => {
                if let Expr::Ident(package) = base.as_ref() {
                    let is_value_receiver = lowering.locals.contains_key(package)
                        || self.globals.contains_key(package)
                        || self.globals.contains_key(&self.qualify_name(package))
                        || self.functions.contains_key(package);
                    if package == "vec" {
                        return self.compile_vec_call(
                            func,
                            lowering.current_block,
                            field,
                            lowered_args,
                        );
                    }
                    if !is_value_receiver
                        && let Some(target_name) = self
                            .namespace_call_targets
                            .get(&format!("{package}.{field}"))
                        && let Some((function, ret_ty)) = self.functions.get(target_name).cloned()
                    {
                        let dst = if ret_ty.is_void() {
                            None
                        } else {
                            Some(self.builder.push_temp(func, ret_ty.clone()))
                        };
                        self.builder.push_instr(
                            func,
                            lowering.current_block,
                            Instr::CallDirect {
                                dst,
                                ret_ty: ret_ty.clone(),
                                function,
                                args: lowered_args,
                            },
                        );
                        return OkOperand::from_call_result(dst);
                    }
                    if !is_value_receiver {
                        let dst = self.builder.push_temp(func, IrType::Unknown);
                        self.builder.push_instr(
                            func,
                            lowering.current_block,
                            Instr::CallBuiltin {
                                dst: Some(dst),
                                ret_ty: IrType::Unknown,
                                builtin: crate::ir::BuiltinCall {
                                    package: package.clone(),
                                    name: field.clone(),
                                },
                                args: lowered_args,
                            },
                        );
                        return Some(Operand::Temp(dst));
                    }
                }
                self.compile_method_call(func, lowering, base, field, lowered_args)
            }
            _ => {
                let callee = self.compile_expr(func, lowering, callee)?;
                let dst = self.builder.push_temp(func, IrType::Unknown);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::CallIndirect {
                        dst: Some(dst),
                        ret_ty: IrType::Unknown,
                        callee,
                        args: lowered_args,
                    },
                );
                Some(Operand::Temp(dst))
            }
        }
    }

    fn compile_method_call(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        base: &Expr,
        field: &str,
        mut args: Vec<Operand>,
    ) -> Option<Operand> {
        let receiver = self.compile_expr(func, lowering, base)?;
        let IrType::Named(struct_name) = self.infer_operand_type(func, &receiver) else {
            self.unsupported(
                "method call on non-struct receiver is not in the initial IR lowering subset",
            );
            return None;
        };
        let method_name = Self::mangle_method_name(&struct_name, field);
        let Some((function, ret_ty)) = self.functions.get(&method_name).cloned() else {
            self.unsupported(format!(
                "unknown method `{field}` for struct `{struct_name}` in IR lowering"
            ));
            return None;
        };
        let mut call_args = Vec::with_capacity(args.len() + 1);
        call_args.push(receiver);
        call_args.append(&mut args);
        let dst = if ret_ty.is_void() {
            None
        } else {
            Some(self.builder.push_temp(func, ret_ty.clone()))
        };
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::CallDirect {
                dst,
                ret_ty: ret_ty.clone(),
                function,
                args: call_args,
            },
        );
        OkOperand::from_call_result(dst)
    }

    pub(super) fn try_compile_vec_new_let(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        name: &str,
        ty: &Option<crate::ast::TypeName>,
        value: &Expr,
    ) -> Option<bool> {
        let Some(crate::ast::TypeName::Vec { elem }) = ty else {
            return None;
        };
        let Expr::Call { callee, args } = value else {
            return None;
        };
        if !args.is_empty()
            || !matches!(&**callee, Expr::Field { base, field } if field == "new" && matches!(&**base, Expr::Ident(pkg) if pkg == "vec"))
        {
            return None;
        }

        let elem_ty = IrType::from(&TypeInfo::from_ast(elem));
        let local_ty = IrType::Vec {
            elem: Box::new(elem_ty.clone()),
        };
        let local = self
            .builder
            .push_local(func, name.to_string(), local_ty.clone());
        lowering.locals.insert(name.to_string(), local);
        let dst = self.builder.push_temp(func, local_ty.clone());
        self.builder
            .push_instr(func, lowering.current_block, Instr::VecNew { dst, elem_ty });
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::StoreLocal {
                local,
                ty: local_ty,
                value: Operand::Temp(dst),
            },
        );
        Some(true)
    }

    fn compile_vec_call(
        &mut self,
        func: &mut crate::ir::IrFunction,
        block: BlockId,
        field: &str,
        args: Vec<Operand>,
    ) -> Option<Operand> {
        match (field, args.as_slice()) {
            ("new", []) => {
                let dst = self.builder.push_temp(
                    func,
                    IrType::Vec {
                        elem: Box::new(IrType::Unknown),
                    },
                );
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecNew {
                        dst,
                        elem_ty: IrType::Unknown,
                    },
                );
                Some(Operand::Temp(dst))
            }
            ("len", [vec]) => {
                let dst = self.builder.push_temp(func, IrType::Int);
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecLen {
                        dst,
                        vec: vec.clone(),
                    },
                );
                Some(Operand::Temp(dst))
            }
            ("push", [vec, value]) => {
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecPush {
                        vec: vec.clone(),
                        value: value.clone(),
                    },
                );
                Some(Operand::Const(ConstValue::Unit))
            }
            ("get", [vec, index]) => {
                let elem_ty = self.array_element_type(func, vec);
                let dst = self.builder.push_temp(func, elem_ty.clone());
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecGet {
                        dst,
                        elem_ty,
                        vec: vec.clone(),
                        index: index.clone(),
                    },
                );
                Some(Operand::Temp(dst))
            }
            ("set", [vec, index, value]) => {
                let elem_ty = self.array_element_type(func, vec);
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecSet {
                        elem_ty,
                        vec: vec.clone(),
                        index: index.clone(),
                        value: value.clone(),
                    },
                );
                Some(Operand::Const(ConstValue::Unit))
            }
            ("delete", [vec, index]) => {
                let elem_ty = self.array_element_type(func, vec);
                let dst = self.builder.push_temp(func, elem_ty.clone());
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecDelete {
                        dst,
                        elem_ty,
                        vec: vec.clone(),
                        index: index.clone(),
                    },
                );
                Some(Operand::Temp(dst))
            }
            _ => {
                self.unsupported(format!("vec.{field} is not supported in IR lowering"));
                None
            }
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

    fn function_value(
        &mut self,
        func: &mut crate::ir::IrFunction,
        block: BlockId,
        name: &str,
    ) -> Option<Operand> {
        let (function, ret_ty) = self.functions.get(name).cloned()?;
        let dst = self.builder.push_temp(
            func,
            IrType::Fn {
                params: Vec::new(),
                ret: Box::new(ret_ty),
            },
        );
        self.builder
            .push_instr(func, block, Instr::MakeClosure { dst, function });
        Some(Operand::Temp(dst))
    }

    fn compile_fn_lit(
        &mut self,
        outer_func: &mut crate::ir::IrFunction,
        block: BlockId,
        params: &[crate::ast::Param],
        return_type: &crate::ast::TypeName,
        body: &[Stmt],
    ) -> Option<Operand> {
        self.fn_lit_counter += 1;
        let name = format!("__fn_lit_{}", self.fn_lit_counter);
        let ret_ty = IrType::from(&TypeInfo::from_ast(return_type));
        let function_id = crate::ir::FunctionId(self.functions.len() + self.lifted_functions.len());
        self.functions
            .insert(name.clone(), (function_id, ret_ty.clone()));

        let mut lifted = self.builder.begin_function(name, ret_ty.clone());
        lifted.id = function_id;
        let mut lowering = FunctionLowering {
            current_block: lifted.entry,
            locals: HashMap::new(),
            scratch_counter: 0,
        };
        let mut param_types = Vec::with_capacity(params.len());
        for param in params {
            let ty_info = TypeInfo::from_ast(&param.ty);
            let ir_ty = IrType::from(&ty_info);
            param_types.push(ir_ty.clone());
            self.builder
                .push_param(&mut lifted, param.name.clone(), ir_ty.clone());
            let local = self
                .builder
                .push_local(&mut lifted, param.name.clone(), ir_ty);
            lowering.locals.insert(param.name.clone(), local);
        }
        for stmt in body {
            if !self.compile_stmt(&mut lifted, &mut lowering, stmt) {
                return None;
            }
        }
        if matches!(
            lifted
                .blocks
                .iter()
                .find(|block| block.id == lowering.current_block)
                .map(|block| &block.terminator),
            Some(Terminator::Unreachable)
        ) {
            let terminator = if ret_ty.is_void() {
                Terminator::Return(None)
            } else {
                self.diags.error(
                    "IR lowering currently requires explicit return in non-void function literal",
                    Span::default(),
                );
                return None;
            };
            self.builder
                .set_terminator(&mut lifted, lowering.current_block, terminator);
        }
        self.lifted_functions.push(lifted);

        let dst = self.builder.push_temp(
            outer_func,
            IrType::Fn {
                params: param_types,
                ret: Box::new(ret_ty),
            },
        );
        self.builder.push_instr(
            outer_func,
            block,
            Instr::MakeClosure {
                dst,
                function: function_id,
            },
        );
        Some(Operand::Temp(dst))
    }
}

struct OkOperand;

impl OkOperand {
    fn from_call_result(dst: Option<crate::ir::TempId>) -> Option<Operand> {
        Some(match dst {
            Some(dst) => Operand::Temp(dst),
            None => Operand::Const(ConstValue::Unit),
        })
    }
}
