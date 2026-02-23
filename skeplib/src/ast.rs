#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Program {
    pub imports: Vec<ImportDecl>,
    pub exports: Vec<ExportDecl>,
    pub globals: Vec<GlobalLetDecl>,
    pub structs: Vec<StructDecl>,
    pub impls: Vec<ImplDecl>,
    pub functions: Vec<FnDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalLetDecl {
    pub name: String,
    pub ty: Option<TypeName>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportDecl {
    ImportModule {
        path: Vec<String>,
        alias: Option<String>,
    },
    ImportFrom {
        path: Vec<String>,
        wildcard: bool,
        items: Vec<ImportItem>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportDecl {
    Local {
        items: Vec<ExportItem>,
    },
    From {
        path: Vec<String>,
        items: Vec<ExportItem>,
    },
    FromAll {
        path: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportItem {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeName>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplDecl {
    pub target: String,
    pub methods: Vec<MethodDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeName>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<TypeName>,
        value: Expr,
    },
    Assign {
        target: AssignTarget,
        value: Expr,
    },
    Expr(Expr),
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        step: Option<Box<Stmt>>,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    Return(Option<Expr>),
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchPattern {
    Wildcard,
    Literal(MatchLiteral),
    Or(Vec<MatchPattern>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchLiteral {
    Int(i64),
    Bool(bool),
    String(String),
    Float(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignTarget {
    Ident(String),
    Path(Vec<String>),
    Index { base: Box<Expr>, index: Expr },
    Field { base: Box<Expr>, field: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    IntLit(i64),
    FloatLit(String),
    Ident(String),
    BoolLit(bool),
    StringLit(String),
    Path(Vec<String>),
    ArrayLit(Vec<Expr>),
    ArrayRepeat {
        value: Box<Expr>,
        size: usize,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        base: Box<Expr>,
        field: String,
    },
    StructLit {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    FnLit {
        params: Vec<Param>,
        return_type: TypeName,
        body: Vec<Stmt>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Group(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeName {
    Int,
    Float,
    Bool,
    String,
    Void,
    Named(String),
    Array {
        elem: Box<TypeName>,
        size: usize,
    },
    Fn {
        params: Vec<TypeName>,
        ret: Box<TypeName>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    EqEq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    AndAnd,
    OrOr,
}

impl Program {
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        for import in &self.imports {
            match import {
                ImportDecl::ImportModule { path, alias } => {
                    out.push_str(&format!("import {}", path.join(".")));
                    if let Some(alias) = alias {
                        out.push_str(&format!(" as {alias}"));
                    }
                    out.push('\n');
                }
                ImportDecl::ImportFrom {
                    path,
                    wildcard,
                    items,
                } => {
                    if *wildcard {
                        out.push_str(&format!("from {} import *\n", path.join(".")));
                    } else {
                        let items = items
                            .iter()
                            .map(|item| match &item.alias {
                                Some(alias) => format!("{} as {}", item.name, alias),
                                None => item.name.clone(),
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        out.push_str(&format!("from {} import {items}\n", path.join(".")));
                    }
                }
            }
        }
        for export in &self.exports {
            match export {
                ExportDecl::Local { items } => {
                    let items = items
                        .iter()
                        .map(|item| match &item.alias {
                            Some(alias) => format!("{} as {}", item.name, alias),
                            None => item.name.clone(),
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    out.push_str(&format!("export {{ {items} }}\n"));
                }
                ExportDecl::From { path, items } => {
                    let items = items
                        .iter()
                        .map(|item| match &item.alias {
                            Some(alias) => format!("{} as {}", item.name, alias),
                            None => item.name.clone(),
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    out.push_str(&format!("export {{ {items} }} from {}\n", path.join(".")));
                }
                ExportDecl::FromAll { path } => {
                    out.push_str(&format!("export * from {}\n", path.join(".")));
                }
            }
        }
        for g in &self.globals {
            if let Some(ty) = &g.ty {
                out.push_str(&format!(
                    "let {}: {} = {};\n",
                    g.name,
                    ty.as_str(),
                    pretty_expr(&g.value)
                ));
            } else {
                out.push_str(&format!("let {} = {};\n", g.name, pretty_expr(&g.value)));
            }
        }
        for s in &self.structs {
            pretty_struct(s, 0, &mut out);
        }
        for i in &self.impls {
            pretty_impl(i, 0, &mut out);
        }
        for func in &self.functions {
            pretty_fn(func, 0, &mut out);
        }
        out
    }
}

fn pretty_struct(s: &StructDecl, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    out.push_str(&format!("{pad}struct {} {{\n", s.name));
    for f in &s.fields {
        out.push_str(&format!("{pad}  {}: {}\n", f.name, f.ty.as_str()));
    }
    out.push_str(&format!("{pad}}}\n"));
}

fn pretty_impl(i: &ImplDecl, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    out.push_str(&format!("{pad}impl {} {{\n", i.target));
    for m in &i.methods {
        pretty_method(m, indent + 2, out);
    }
    out.push_str(&format!("{pad}}}\n"));
}

fn pretty_method(method: &MethodDecl, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    let params = method
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    let ret = method
        .return_type
        .as_ref()
        .map(TypeName::as_str)
        .unwrap_or_else(|| "Void".to_string());
    out.push_str(&format!("{pad}fn {}({}) -> {}\n", method.name, params, ret));
    for stmt in &method.body {
        pretty_stmt(stmt, indent + 2, out);
    }
}

fn pretty_fn(func: &FnDecl, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    let params = func
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    let ret = func
        .return_type
        .as_ref()
        .map(TypeName::as_str)
        .unwrap_or_else(|| "Void".to_string());
    out.push_str(&format!("{pad}fn {}({}) -> {}\n", func.name, params, ret));
    for stmt in &func.body {
        pretty_stmt(stmt, indent + 2, out);
    }
}

fn pretty_stmt(stmt: &Stmt, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    match stmt {
        Stmt::Let { name, ty, value } => {
            if let Some(ty) = ty {
                out.push_str(&format!(
                    "{pad}let {}: {} = {}\n",
                    name,
                    ty.as_str(),
                    pretty_expr(value)
                ));
            } else {
                out.push_str(&format!("{pad}let {} = {}\n", name, pretty_expr(value)));
            }
        }
        Stmt::Assign { target, value } => {
            let target = match target {
                AssignTarget::Ident(n) => n.clone(),
                AssignTarget::Path(parts) => parts.join("."),
                AssignTarget::Index { base, index } => {
                    format!("{}[{}]", pretty_expr(base), pretty_expr(index))
                }
                AssignTarget::Field { base, field } => format!("{}.{}", pretty_expr(base), field),
            };
            out.push_str(&format!(
                "{pad}assign {} = {}\n",
                target,
                pretty_expr(value)
            ));
        }
        Stmt::Expr(expr) => {
            out.push_str(&format!("{pad}expr {}\n", pretty_expr(expr)));
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
        } => {
            out.push_str(&format!("{pad}if {}\n", pretty_expr(cond)));
            for s in then_body {
                pretty_stmt(s, indent + 2, out);
            }
            if !else_body.is_empty() {
                out.push_str(&format!("{pad}else\n"));
                for s in else_body {
                    pretty_stmt(s, indent + 2, out);
                }
            }
        }
        Stmt::While { cond, body } => {
            out.push_str(&format!("{pad}while {}\n", pretty_expr(cond)));
            for s in body {
                pretty_stmt(s, indent + 2, out);
            }
        }
        Stmt::For {
            init,
            cond,
            step,
            body,
        } => {
            let init = init
                .as_ref()
                .map(|s| pretty_for_clause_stmt(s))
                .unwrap_or_default();
            let cond = cond.as_ref().map(pretty_expr).unwrap_or_default();
            let step = step
                .as_ref()
                .map(|s| pretty_for_clause_stmt(s))
                .unwrap_or_default();
            out.push_str(&format!("{pad}for ({init}; {cond}; {step})\n"));
            for s in body {
                pretty_stmt(s, indent + 2, out);
            }
        }
        Stmt::Return(expr) => {
            if let Some(expr) = expr {
                out.push_str(&format!("{pad}return {}\n", pretty_expr(expr)));
            } else {
                out.push_str(&format!("{pad}return\n"));
            }
        }
        Stmt::Break => out.push_str(&format!("{pad}break\n")),
        Stmt::Continue => out.push_str(&format!("{pad}continue\n")),
        Stmt::Match { expr, arms } => {
            out.push_str(&format!("{pad}match {}\n", pretty_expr(expr)));
            for arm in arms {
                out.push_str(&format!(
                    "{pad}  arm {}\n",
                    pretty_match_pattern(&arm.pattern)
                ));
                for s in &arm.body {
                    pretty_stmt(s, indent + 4, out);
                }
            }
        }
    }
}

fn pretty_for_clause_stmt(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Let { name, ty, value } => {
            if let Some(ty) = ty {
                format!("let {}: {} = {}", name, ty.as_str(), pretty_expr(value))
            } else {
                format!("let {} = {}", name, pretty_expr(value))
            }
        }
        Stmt::Assign { target, value } => {
            let target = match target {
                AssignTarget::Ident(n) => n.clone(),
                AssignTarget::Path(parts) => parts.join("."),
                AssignTarget::Index { base, index } => {
                    format!("{}[{}]", pretty_expr(base), pretty_expr(index))
                }
                AssignTarget::Field { base, field } => format!("{}.{}", pretty_expr(base), field),
            };
            format!("{target} = {}", pretty_expr(value))
        }
        Stmt::Expr(expr) => pretty_expr(expr),
        _ => "<invalid-for-clause>".to_string(),
    }
}

fn pretty_expr(expr: &Expr) -> String {
    match expr {
        Expr::IntLit(v) => v.to_string(),
        Expr::FloatLit(v) => v.clone(),
        Expr::Ident(n) => n.clone(),
        Expr::BoolLit(v) => v.to_string(),
        Expr::StringLit(s) => format!("\"{}\"", s.replace('"', "\\\"")),
        Expr::Path(parts) => parts.join("."),
        Expr::ArrayLit(items) => {
            let items = items.iter().map(pretty_expr).collect::<Vec<_>>().join(", ");
            format!("[{items}]")
        }
        Expr::ArrayRepeat { value, size } => format!("[{}; {}]", pretty_expr(value), size),
        Expr::Index { base, index } => format!("{}[{}]", pretty_expr(base), pretty_expr(index)),
        Expr::Field { base, field } => format!("{}.{}", pretty_expr(base), field),
        Expr::StructLit { name, fields } => {
            let fields = fields
                .iter()
                .map(|(n, v)| format!("{n}: {}", pretty_expr(v)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name} {{ {fields} }}")
        }
        Expr::FnLit {
            params,
            return_type,
            ..
        } => {
            let params = params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.ty.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            format!("fn({params}) -> {}", return_type.as_str())
        }
        Expr::Unary { op, expr } => {
            let symbol = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Pos => "+",
                UnaryOp::Not => "!",
            };
            format!("({}{})", symbol, pretty_expr(expr))
        }
        Expr::Binary { left, op, right } => {
            let symbol = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Mod => "%",
                BinaryOp::EqEq => "==",
                BinaryOp::Neq => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::Lte => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Gte => ">=",
                BinaryOp::AndAnd => "&&",
                BinaryOp::OrOr => "||",
            };
            format!("({} {} {})", pretty_expr(left), symbol, pretty_expr(right))
        }
        Expr::Call { callee, args } => {
            let args = args.iter().map(pretty_expr).collect::<Vec<_>>().join(", ");
            format!("{}({})", pretty_expr(callee), args)
        }
        Expr::Group(inner) => format!("({})", pretty_expr(inner)),
    }
}

fn pretty_match_pattern(pat: &MatchPattern) -> String {
    match pat {
        MatchPattern::Wildcard => "_".to_string(),
        MatchPattern::Literal(MatchLiteral::Int(v)) => v.to_string(),
        MatchPattern::Literal(MatchLiteral::Bool(v)) => v.to_string(),
        MatchPattern::Literal(MatchLiteral::String(s)) => format!("\"{}\"", s.replace('"', "\\\"")),
        MatchPattern::Literal(MatchLiteral::Float(v)) => v.clone(),
        MatchPattern::Or(parts) => parts
            .iter()
            .map(pretty_match_pattern)
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

impl TypeName {
    pub fn as_str(&self) -> String {
        match self {
            TypeName::Int => "Int".to_string(),
            TypeName::Float => "Float".to_string(),
            TypeName::Bool => "Bool".to_string(),
            TypeName::String => "String".to_string(),
            TypeName::Void => "Void".to_string(),
            TypeName::Named(name) => name.clone(),
            TypeName::Array { elem, size } => format!("[{}; {}]", elem.as_str(), size),
            TypeName::Fn { params, ret } => {
                let params = params
                    .iter()
                    .map(TypeName::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Fn({params}) -> {}", ret.as_str())
            }
        }
    }
}
