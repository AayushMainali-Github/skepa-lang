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
}
