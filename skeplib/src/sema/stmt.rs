use std::collections::HashMap;

use crate::ast::{AssignTarget, Expr, MatchLiteral, MatchPattern, Stmt};
use crate::types::{TypeInfo, display_type};

use super::Checker;

impl Checker {
    fn refine_result_type_from_expected(value_ty: TypeInfo, expected: &TypeInfo) -> TypeInfo {
        match (expected, value_ty) {
            (
                TypeInfo::Result { ok, err },
                TypeInfo::Result {
                    ok: value_ok,
                    err: value_err,
                },
            ) => TypeInfo::Result {
                ok: Box::new(if matches!(*value_ok, TypeInfo::Unknown) {
                    (**ok).clone()
                } else {
                    (*value_ok).clone()
                }),
                err: Box::new(if matches!(*value_err, TypeInfo::Unknown) {
                    (**err).clone()
                } else {
                    (*value_err).clone()
                }),
            },
            (_, value_ty) => value_ty,
        }
    }

    pub(super) fn is_vec_new_call(expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::Call { callee, args }
                if args.is_empty()
                    && matches!(
                        &**callee,
                        Expr::Path(parts) if parts.len() == 2 && parts[0] == "vec" && parts[1] == "new"
                    )
                        || matches!(
                            &**callee,
                            Expr::Field { base, field }
                                if field == "new"
                                    && matches!(&**base, Expr::Ident(pkg) if pkg == "vec")
                        )
        )
    }

    pub(super) fn is_map_new_call(expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::Call { callee, args }
                if args.is_empty()
                    && matches!(
                        &**callee,
                        Expr::Path(parts) if parts.len() == 2 && parts[0] == "map" && parts[1] == "new"
                    )
                        || matches!(
                            &**callee,
                            Expr::Field { base, field }
                                if field == "new"
                                    && matches!(&**base, Expr::Ident(pkg) if pkg == "map")
                        )
        )
    }

    pub(super) fn is_task_channel_call(expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::Call { callee, args }
                if args.is_empty()
                    && matches!(
                        &**callee,
                        Expr::Path(parts) if parts.len() == 2 && parts[0] == "task" && parts[1] == "channel"
                    )
                        || matches!(
                            &**callee,
                            Expr::Field { base, field }
                                if field == "channel"
                                    && matches!(&**base, Expr::Ident(pkg) if pkg == "task")
                        )
        )
    }

    fn match_pattern_literal_key_and_label(pat: &MatchPattern) -> Option<(String, String)> {
        match pat {
            MatchPattern::Literal(MatchLiteral::Int(v)) => {
                Some((format!("int:{v}"), format!("Int literal `{v}`")))
            }
            MatchPattern::Literal(MatchLiteral::Bool(v)) => {
                Some((format!("bool:{v}"), format!("Bool literal `{v}`")))
            }
            MatchPattern::Literal(MatchLiteral::String(v)) => {
                Some((format!("string:{v}"), format!("String literal \"{v}\"")))
            }
            MatchPattern::Literal(MatchLiteral::Float(v)) => {
                Some((format!("float:{v}"), format!("Float literal `{v}`")))
            }
            MatchPattern::Variant { name, .. } => {
                Some((format!("variant:{name}"), format!("variant `{name}`")))
            }
            MatchPattern::Wildcard | MatchPattern::Or(_) => None,
        }
    }

    pub(super) fn match_variant_binding_type(
        pat: &MatchPattern,
        target_ty: &TypeInfo,
    ) -> Option<TypeInfo> {
        match (pat, target_ty) {
            (MatchPattern::Variant { name, .. }, TypeInfo::Option { value }) if name == "Some" => {
                Some((**value).clone())
            }
            (MatchPattern::Variant { name, .. }, TypeInfo::Result { ok, .. }) if name == "Ok" => {
                Some((**ok).clone())
            }
            (MatchPattern::Variant { name, .. }, TypeInfo::Result { err, .. }) if name == "Err" => {
                Some((**err).clone())
            }
            _ => None,
        }
    }

    fn match_variant_allowed(name: &str, target_ty: &TypeInfo) -> bool {
        match target_ty {
            TypeInfo::Option { .. } => matches!(name, "Some" | "None"),
            TypeInfo::Result { .. } => matches!(name, "Ok" | "Err"),
            TypeInfo::Unknown => matches!(name, "Some" | "None" | "Ok" | "Err"),
            _ => false,
        }
    }

    fn pattern_has_wildcard(pat: &MatchPattern) -> bool {
        match pat {
            MatchPattern::Wildcard => true,
            MatchPattern::Or(parts) => parts.iter().any(Self::pattern_has_wildcard),
            _ => false,
        }
    }

    pub(super) fn check_match_exhaustiveness(
        &mut self,
        target_ty: &TypeInfo,
        seen_wildcard: bool,
        seen_literals: &std::collections::HashSet<String>,
    ) {
        if seen_wildcard {
            return;
        }
        match target_ty {
            TypeInfo::Option { .. } => {
                let has_some = seen_literals.contains("variant:Some");
                let has_none = seen_literals.contains("variant:None");
                if !(has_some && has_none) {
                    self.error(
                        "Non-exhaustive match on Option: add both `Some(...)` and `None` arms, or add a wildcard arm `_`"
                            .to_string(),
                    );
                }
            }
            TypeInfo::Result { .. } => {
                let has_ok = seen_literals.contains("variant:Ok");
                let has_err = seen_literals.contains("variant:Err");
                if !(has_ok && has_err) {
                    self.error(
                        "Non-exhaustive match on Result: add both `Ok(...)` and `Err(...)` arms, or add a wildcard arm `_`"
                            .to_string(),
                    );
                }
            }
            _ => {
                self.error(format!(
                    "Non-exhaustive match on {}: add a wildcard arm `_`",
                    display_type(target_ty)
                ));
            }
        }
    }

    pub(super) fn check_match_pattern(
        &mut self,
        pat: &MatchPattern,
        target_ty: &TypeInfo,
        seen_literals: &mut std::collections::HashSet<String>,
    ) {
        match pat {
            MatchPattern::Wildcard => {}
            MatchPattern::Literal(lit) => {
                let lit_ty = match lit {
                    MatchLiteral::Int(_) => TypeInfo::Int,
                    MatchLiteral::Bool(_) => TypeInfo::Bool,
                    MatchLiteral::String(_) => TypeInfo::String,
                    MatchLiteral::Float(_) => TypeInfo::Float,
                };
                if *target_ty != TypeInfo::Unknown && *target_ty != lit_ty {
                    self.error(format!(
                        "Match pattern type mismatch: target {:?}, pattern {:?}",
                        target_ty, lit_ty
                    ));
                }
                if let Some((key, label)) = Self::match_pattern_literal_key_and_label(pat)
                    && !seen_literals.insert(key)
                {
                    self.error(format!("Duplicate match pattern {label}"));
                }
            }
            MatchPattern::Variant { name, binding } => {
                if !Self::match_variant_allowed(name, target_ty) {
                    self.error(format!(
                        "Match variant `{name}` is not valid for target type {}",
                        display_type(target_ty)
                    ));
                }
                if matches!(name.as_str(), "None" | "Some" | "Ok" | "Err")
                    && matches!(name.as_str(), "None")
                    && binding.is_some()
                {
                    self.error("Match variant `None` cannot bind a value".to_string());
                }
                if let Some((key, label)) = Self::match_pattern_literal_key_and_label(pat)
                    && !seen_literals.insert(key)
                {
                    self.error(format!("Duplicate match pattern {label}"));
                }
            }
            MatchPattern::Or(parts) => {
                if parts.is_empty() {
                    self.error(
                        "Match OR-pattern must contain at least one alternative".to_string(),
                    );
                    return;
                }
                for part in parts {
                    if matches!(part, MatchPattern::Wildcard | MatchPattern::Or(_)) {
                        self.error(
                            "Match OR-pattern alternatives must be literals or variants"
                                .to_string(),
                        );
                        continue;
                    }
                    if matches!(
                        part,
                        MatchPattern::Variant {
                            binding: Some(_),
                            ..
                        }
                    ) {
                        self.error(
                            "Match OR-pattern variant alternatives cannot bind values".to_string(),
                        );
                        continue;
                    }
                    self.check_match_pattern(part, target_ty, seen_literals);
                }
            }
        }
    }

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
                match base_ty {
                    TypeInfo::Named(struct_name) => {
                        if let Some(field_ty) = self.field_type(&struct_name, field) {
                            field_ty
                        } else {
                            self.error(format!(
                                "Unknown field `{}` on struct `{}`",
                                field, struct_name
                            ));
                            TypeInfo::Unknown
                        }
                    }
                    TypeInfo::Unknown => TypeInfo::Unknown,
                    other => {
                        self.error(format!(
                            "Field assignment requires struct value, got {:?}",
                            other
                        ));
                        TypeInfo::Unknown
                    }
                }
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
                        if Self::is_vec_new_call(value) {
                            match &declared {
                                TypeInfo::Vec { .. } => {}
                                _ => {
                                    self.error(format!(
                                        "Type mismatch in let `{name}`: declared {:?}, got vec.new()",
                                        declared
                                    ));
                                }
                            }
                            declared
                        } else if Self::is_map_new_call(value) {
                            match &declared {
                                TypeInfo::Map { .. } => {}
                                _ => {
                                    self.error(format!(
                                        "Type mismatch in let `{name}`: declared {:?}, got map.new()",
                                        declared
                                    ));
                                }
                            }
                            declared
                        } else if Self::is_task_channel_call(value) {
                            match &declared {
                                TypeInfo::Opaque(name)
                                    if crate::types::task_channel_value_type(name).is_some() => {}
                                _ => {
                                    self.error(format!(
                                        "Type mismatch in let `{name}`: declared {:?}, got task.channel()",
                                        declared
                                    ));
                                }
                            }
                            declared
                        } else {
                            let expr_ty =
                                Self::refine_result_type_from_expected(expr_ty, &declared);
                            if !Self::types_compatible(&expr_ty, &declared) {
                                self.error(format!(
                                    "Type mismatch in let `{name}`: declared {:?}, got {:?}",
                                    declared, expr_ty
                                ));
                            }
                            declared
                        }
                    }
                    None => {
                        if Self::is_vec_new_call(value) {
                            self.error(format!(
                                "Cannot infer vector element type for let `{name}`; annotate as `Vec[T]`"
                            ));
                            TypeInfo::Unknown
                        } else if Self::is_map_new_call(value) {
                            self.error(format!(
                                "Cannot infer map value type for let `{name}`; annotate as `Map[String, T]`"
                            ));
                            TypeInfo::Unknown
                        } else if Self::is_task_channel_call(value) {
                            self.error(format!(
                                "Cannot infer channel value type for let `{name}`; annotate as `task.Channel[T]`"
                            ));
                            TypeInfo::Unknown
                        } else {
                            expr_ty
                        }
                    }
                };
                if let Some(scope) = scopes.last_mut() {
                    if scope.contains_key(name) {
                        self.error(format!(
                            "Duplicate local binding `{name}` in the same scope"
                        ));
                    } else {
                        scope.insert(name.clone(), var_ty);
                    }
                }
            }
            Stmt::Assign { target, value } => {
                let target_ty = self.lookup_assignment_target(target, scopes);
                let value_ty = self.check_expr(value, scopes);
                if !Self::types_compatible(&value_ty, &target_ty) {
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
                let ret_ty = Self::refine_result_type_from_expected(ret_ty, expected_ret);
                if ret_ty != TypeInfo::Unknown
                    && &ret_ty != expected_ret
                    && !Self::types_compatible(&ret_ty, expected_ret)
                {
                    self.error(format!(
                        "Return type mismatch: expected {:?}, got {:?}",
                        expected_ret, ret_ty
                    ));
                }
            }
            Stmt::Match { expr, arms } => {
                let target_ty = self.check_expr(expr, scopes);
                let mut seen_wildcard = false;
                let mut seen_literals = std::collections::HashSet::<String>::new();

                if arms.is_empty() {
                    self.error("Match statement must have at least one arm".to_string());
                    return;
                }

                for (idx, arm) in arms.iter().enumerate() {
                    if Self::pattern_has_wildcard(&arm.pattern) {
                        if seen_wildcard {
                            self.error(
                                "Match statement can contain only one wildcard arm".to_string(),
                            );
                        }
                        if idx + 1 != arms.len() {
                            self.error("Wildcard match arm `_` must be last".to_string());
                        }
                        seen_wildcard = true;
                    }

                    self.check_match_pattern(&arm.pattern, &target_ty, &mut seen_literals);

                    scopes.push(HashMap::new());
                    if let MatchPattern::Variant {
                        binding: Some(binding),
                        ..
                    } = &arm.pattern
                        && let Some(binding_ty) =
                            Self::match_variant_binding_type(&arm.pattern, &target_ty)
                        && let Some(scope) = scopes.last_mut()
                    {
                        scope.insert(binding.clone(), binding_ty);
                    }
                    for s in &arm.body {
                        self.check_stmt(s, scopes, expected_ret);
                    }
                    scopes.pop();
                }

                Self::check_match_exhaustiveness(self, &target_ty, seen_wildcard, &seen_literals);
            }
        }
    }
}
