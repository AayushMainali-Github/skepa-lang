use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const STRING1: &[TypeInfo] = &[TypeInfo::String];
const STRING2: &[TypeInfo] = &[TypeInfo::String, TypeInfo::String];

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "fs",
        name: "exists",
        params: STRING1,
        ret: TypeInfo::Bool,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "fs",
        name: "readText",
        params: STRING1,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "fs",
        name: "writeText",
        params: STRING2,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "fs",
        name: "appendText",
        params: STRING2,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "fs",
        name: "mkdirAll",
        params: STRING1,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "fs",
        name: "removeFile",
        params: STRING1,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "fs",
        name: "removeDirAll",
        params: STRING1,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "fs",
        name: "join",
        params: STRING2,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
];
