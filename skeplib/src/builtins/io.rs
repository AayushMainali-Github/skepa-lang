use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const IO_PRINT_PARAMS: &[TypeInfo] = &[TypeInfo::String];
const IO_PRINT_INT_PARAMS: &[TypeInfo] = &[TypeInfo::Int];
const IO_PRINT_FLOAT_PARAMS: &[TypeInfo] = &[TypeInfo::Float];
const IO_PRINT_BOOL_PARAMS: &[TypeInfo] = &[TypeInfo::Bool];
const IO_PRINT_STRING_PARAMS: &[TypeInfo] = &[TypeInfo::String];
const IO_READLINE_PARAMS: &[TypeInfo] = &[];

pub(super) const SIGS: &[BuiltinSig] = &[
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
];
