use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const NO_PARAMS: &[TypeInfo] = &[];
const INT_PARAM: &[TypeInfo] = &[TypeInfo::Int];
const STRING_PARAM: &[TypeInfo] = &[TypeInfo::String];
const STRING2_PARAMS: &[TypeInfo] = &[TypeInfo::String, TypeInfo::String];

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "os",
        name: "platform",
        params: NO_PARAMS,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "arch",
        params: NO_PARAMS,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "arg",
        params: INT_PARAM,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "envHas",
        params: STRING_PARAM,
        ret: TypeInfo::Bool,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "envGet",
        params: STRING_PARAM,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "envSet",
        params: STRING2_PARAMS,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "envRemove",
        params: STRING_PARAM,
        ret: TypeInfo::Void,
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
        name: "exit",
        params: INT_PARAM,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "exec",
        params: STRING2_PARAMS,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "os",
        name: "execOut",
        params: STRING2_PARAMS,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
];
