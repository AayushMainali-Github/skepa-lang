use crate::types::TypeInfo;

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

const IO_PRINT_PARAMS: &[TypeInfo] = &[TypeInfo::String];
const IO_PRINT_INT_PARAMS: &[TypeInfo] = &[TypeInfo::Int];
const IO_PRINT_FLOAT_PARAMS: &[TypeInfo] = &[TypeInfo::Float];
const IO_PRINT_BOOL_PARAMS: &[TypeInfo] = &[TypeInfo::Bool];
const IO_PRINT_STRING_PARAMS: &[TypeInfo] = &[TypeInfo::String];
const IO_READLINE_PARAMS: &[TypeInfo] = &[];
const STR_ONE_STRING_PARAM: &[TypeInfo] = &[TypeInfo::String];
const STR_TWO_STRING_PARAMS: &[TypeInfo] = &[TypeInfo::String, TypeInfo::String];
const STR_SLICE_PARAMS: &[TypeInfo] = &[TypeInfo::String, TypeInfo::Int, TypeInfo::Int];

pub const BUILTIN_SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "io",
        name: "print",
        params: IO_PRINT_PARAMS,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "io",
        name: "println",
        params: IO_PRINT_PARAMS,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "io",
        name: "printInt",
        params: IO_PRINT_INT_PARAMS,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "io",
        name: "printFloat",
        params: IO_PRINT_FLOAT_PARAMS,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "io",
        name: "printBool",
        params: IO_PRINT_BOOL_PARAMS,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "io",
        name: "printString",
        params: IO_PRINT_STRING_PARAMS,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "io",
        name: "format",
        params: IO_PRINT_PARAMS,
        ret: TypeInfo::String,
        kind: BuiltinKind::FormatVariadic,
    },
    BuiltinSig {
        package: "io",
        name: "printf",
        params: IO_PRINT_PARAMS,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FormatVariadic,
    },
    BuiltinSig {
        package: "io",
        name: "readLine",
        params: IO_READLINE_PARAMS,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "len",
        params: STR_ONE_STRING_PARAM,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "contains",
        params: STR_TWO_STRING_PARAMS,
        ret: TypeInfo::Bool,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "startsWith",
        params: STR_TWO_STRING_PARAMS,
        ret: TypeInfo::Bool,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "endsWith",
        params: STR_TWO_STRING_PARAMS,
        ret: TypeInfo::Bool,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "trim",
        params: STR_ONE_STRING_PARAM,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "toLower",
        params: STR_ONE_STRING_PARAM,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "toUpper",
        params: STR_ONE_STRING_PARAM,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "indexOf",
        params: STR_TWO_STRING_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "slice",
        params: STR_SLICE_PARAMS,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "isEmpty",
        params: STR_ONE_STRING_PARAM,
        ret: TypeInfo::Bool,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "lastIndexOf",
        params: STR_TWO_STRING_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "replace",
        params: &[TypeInfo::String, TypeInfo::String, TypeInfo::String],
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "str",
        name: "repeat",
        params: &[TypeInfo::String, TypeInfo::Int],
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "arr",
        name: "len",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "isEmpty",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "contains",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "indexOf",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "sum",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "count",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "first",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "last",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "reverse",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "join",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "slice",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "min",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "max",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "sort",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "distinct",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
];

pub fn find_builtin_sig(package: &str, name: &str) -> Option<&'static BuiltinSig> {
    BUILTIN_SIGS
        .iter()
        .find(|s| s.package == package && s.name == name)
}
