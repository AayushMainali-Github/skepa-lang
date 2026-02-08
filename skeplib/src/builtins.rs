use crate::types::TypeInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinSig {
    pub package: &'static str,
    pub name: &'static str,
    pub params: &'static [TypeInfo],
    pub ret: TypeInfo,
}

const IO_PRINT_PARAMS: &[TypeInfo] = &[TypeInfo::String];
const IO_READLINE_PARAMS: &[TypeInfo] = &[];

pub const BUILTIN_SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "io",
        name: "print",
        params: IO_PRINT_PARAMS,
        ret: TypeInfo::Void,
    },
    BuiltinSig {
        package: "io",
        name: "println",
        params: IO_PRINT_PARAMS,
        ret: TypeInfo::Void,
    },
    BuiltinSig {
        package: "io",
        name: "readLine",
        params: IO_READLINE_PARAMS,
        ret: TypeInfo::String,
    },
];

pub fn find_builtin_sig(package: &str, name: &str) -> Option<&'static BuiltinSig> {
    BUILTIN_SIGS
        .iter()
        .find(|s| s.package == package && s.name == name)
}
