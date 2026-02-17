use std::collections::HashMap;

use crate::ast::{AssignTarget, Stmt};
use crate::types::TypeInfo;

use super::Checker;

impl Checker {
    fn lookup_assignment_target(
        &mut self,
        target: &AssignTarget,
        scopes: &mut [HashMap<String, TypeInfo>],
    ) -> TypeInfo {
        match target {
            AssignTarget::Ident(name) => self.lookup_var(name, scopes),
            AssignTarget::Path(parts) => {
                if parts.len() >= 2 {
                    self.error(
                        "Path assignment semantic typing is not supported yet in v0 checker"
                            .to_string(),
                    );
                }
                TypeInfo::Unknown
            }
            AssignTarget::Index { .. } => {
                if let AssignTarget::Index { base, index } = target {
                    let base_ty = self.check_expr(base, scopes);
                    let idx_ty = self.check_expr(index, scopes);
                    if idx_ty != TypeInfo::Int && idx_ty != TypeInfo::Unknown {
                        self.error("Array index must be Int".to_string());
                    }
                    match base_ty {
                        TypeInfo::Array { elem, .. } => *elem,
                        TypeInfo::Unknown => TypeInfo::Unknown,
                        other => {
                            self.error(format!(
                                "Cannot index-assign into non-array type {:?}",
                                other
                            ));
                            TypeInfo::Unknown
                        }
                    }
                } else {
                    TypeInfo::Unknown
                }
            }
            AssignTarget::Field { base, field } => {
                let base_ty = self.check_expr(base, scopes);
                self.error(format!(
                    "Field assignment not supported yet in v0 checker for base {:?} and field `{}`",
                    base_ty, field
                ));
                TypeInfo::Unknown
            }
        }
    }

    pub(super) fn check_stmt(
        &mut self,
        stmt: &Stmt,
        scopes: &mut Vec<HashMap<String, TypeInfo>>,
        expected_ret: &TypeInfo,
    ) {
        match stmt {
            Stmt::Let { name, ty, value } => {
                let expr_ty = self.check_expr(value, scopes);
                let var_ty = match ty {
                    Some(t) => {
                        let declared = TypeInfo::from_ast(t);
                        if expr_ty != TypeInfo::Unknown && declared != expr_ty {
                            self.error(format!(
                                "Type mismatch in let `{name}`: declared {:?}, got {:?}",
                                declared, expr_ty
                            ));
                        }
                        declared
                    }
                    None => expr_ty,
                };
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(name.clone(), var_ty);
                }
            }
            Stmt::Assign { target, value } => {
                let target_ty = self.lookup_assignment_target(target, scopes);
                let value_ty = self.check_expr(value, scopes);
                if target_ty != TypeInfo::Unknown
                    && value_ty != TypeInfo::Unknown
                    && target_ty != value_ty
                {
                    self.error(format!(
                        "Assignment type mismatch: target {:?}, value {:?}",
                        target_ty, value_ty
                    ));
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr, scopes);
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => {
                let cond_ty = self.check_expr(cond, scopes);
                if cond_ty != TypeInfo::Bool && cond_ty != TypeInfo::Unknown {
                    self.error("if condition must be Bool".to_string());
                }

                scopes.push(HashMap::new());
                for s in then_body {
                    self.check_stmt(s, scopes, expected_ret);
                }
                scopes.pop();

                scopes.push(HashMap::new());
                for s in else_body {
                    self.check_stmt(s, scopes, expected_ret);
                }
                scopes.pop();
            }
            Stmt::While { cond, body } => {
                let cond_ty = self.check_expr(cond, scopes);
                if cond_ty != TypeInfo::Bool && cond_ty != TypeInfo::Unknown {
                    self.error("while condition must be Bool".to_string());
                }

                self.loop_depth += 1;
                scopes.push(HashMap::new());
                for s in body {
                    self.check_stmt(s, scopes, expected_ret);
                }
                scopes.pop();
                self.loop_depth = self.loop_depth.saturating_sub(1);
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
            } => {
                scopes.push(HashMap::new());
                if let Some(init) = init {
                    self.check_stmt(init, scopes, expected_ret);
                }

                if let Some(cond) = cond {
                    let cond_ty = self.check_expr(cond, scopes);
                    if cond_ty != TypeInfo::Bool && cond_ty != TypeInfo::Unknown {
                        self.error("for condition must be Bool".to_string());
                    }
                }

                self.loop_depth += 1;
                for s in body {
                    self.check_stmt(s, scopes, expected_ret);
                }
                if let Some(step) = step {
                    self.check_stmt(step, scopes, expected_ret);
                }
                self.loop_depth = self.loop_depth.saturating_sub(1);
                scopes.pop();
            }
            Stmt::Break => {
                if self.loop_depth == 0 {
                    self.error("`break` is only allowed inside a loop".to_string());
                }
            }
            Stmt::Continue => {
                if self.loop_depth == 0 {
                    self.error("`continue` is only allowed inside a loop".to_string());
                }
            }
            Stmt::Return(expr_opt) => {
                let ret_ty = match expr_opt {
                    Some(expr) => self.check_expr(expr, scopes),
                    None => TypeInfo::Void,
                };
                if ret_ty != TypeInfo::Unknown && &ret_ty != expected_ret {
                    self.error(format!(
                        "Return type mismatch: expected {:?}, got {:?}",
                        expected_ret, ret_ty
                    ));
                }
            }
        }
    }
}
