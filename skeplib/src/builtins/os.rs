use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const NO_PARAMS: &[TypeInfo] = &[];
const INT_PARAM: &[TypeInfo] = &[TypeInfo::Int];
const STRING_PARAM: &[TypeInfo] = &[TypeInfo::String];

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "os",
        name: "cwd",
        params: NO_PARAMS,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "platform",
        params: NO_PARAMS,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "sleep",
        params: INT_PARAM,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "execShell",
        params: STRING_PARAM,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "execShellOut",
        params: STRING_PARAM,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
];
