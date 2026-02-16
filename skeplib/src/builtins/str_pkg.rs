use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const STR_ONE_STRING_PARAM: &[TypeInfo] = &[TypeInfo::String];
const STR_TWO_STRING_PARAMS: &[TypeInfo] = &[TypeInfo::String, TypeInfo::String];
const STR_SLICE_PARAMS: &[TypeInfo] = &[TypeInfo::String, TypeInfo::Int, TypeInfo::Int];

pub(super) const SIGS: &[BuiltinSig] = &[
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
];
