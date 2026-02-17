use crate::types::TypeInfo;

mod arr;
mod datetime;
mod io;
mod str_pkg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinKind {
    FixedArity,
    FormatVariadic,
    ArrayOps,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinSig {
    pub package: &'static str,
    pub name: &'static str,
    pub params: &'static [TypeInfo],
    pub ret: TypeInfo,
    pub kind: BuiltinKind,
}

pub fn find_builtin_sig(package: &str, name: &str) -> Option<&'static BuiltinSig> {
    io::SIGS
        .iter()
        .chain(str_pkg::SIGS.iter())
        .chain(arr::SIGS.iter())
        .chain(datetime::SIGS.iter())
        .find(|s| s.package == package && s.name == name)
}
