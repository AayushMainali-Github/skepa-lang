#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Program {
    pub imports: Vec<ImportDecl>,
    pub functions: Vec<FnDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDecl {
    pub module: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnDecl {
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
    Return(Option<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignTarget {
    Ident(String),
    Path(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    IntLit(i64),
    Ident(String),
    BoolLit(bool),
    StringLit(String),
    Path(Vec<String>),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeName {
    Int,
    Float,
    Bool,
    String,
    Void,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
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
            out.push_str(&format!("import {}\n", import.module));
        }
        for func in &self.functions {
            pretty_fn(func, 0, &mut out);
        }
        out
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
    let ret = func.return_type.map(|t| t.as_str()).unwrap_or("Void");
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
            };
            out.push_str(&format!("{pad}assign {} = {}\n", target, pretty_expr(value)));
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
        Stmt::Return(expr) => {
            if let Some(expr) = expr {
                out.push_str(&format!("{pad}return {}\n", pretty_expr(expr)));
            } else {
                out.push_str(&format!("{pad}return\n"));
            }
        }
    }
}

fn pretty_expr(expr: &Expr) -> String {
    match expr {
        Expr::IntLit(v) => v.to_string(),
        Expr::Ident(n) => n.clone(),
        Expr::BoolLit(v) => v.to_string(),
        Expr::StringLit(s) => format!("\"{}\"", s.replace('"', "\\\"")),
        Expr::Path(parts) => parts.join("."),
        Expr::Unary { op, expr } => {
            let symbol = match op {
                UnaryOp::Neg => "-",
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

impl TypeName {
    pub fn as_str(self) -> &'static str {
        match self {
            TypeName::Int => "Int",
            TypeName::Float => "Float",
            TypeName::Bool => "Bool",
            TypeName::String => "String",
            TypeName::Void => "Void",
        }
    }
}
