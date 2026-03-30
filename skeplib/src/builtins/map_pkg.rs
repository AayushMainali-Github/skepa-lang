use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const NO_PARAMS: &[TypeInfo] = &[];
const MAP_PARAM_SENTINEL: &[TypeInfo] = &[TypeInfo::Unknown];
const MAP_AND_STRING_PARAMS: &[TypeInfo] = &[TypeInfo::Unknown, TypeInfo::String];
const MAP_STRING_AND_UNKNOWN_PARAMS: &[TypeInfo] =
    &[TypeInfo::Unknown, TypeInfo::String, TypeInfo::Unknown];

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "map",
        name: "new",
        params: NO_PARAMS,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "map",
        name: "len",
        params: MAP_PARAM_SENTINEL,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "map",
        name: "has",
        params: MAP_AND_STRING_PARAMS,
        ret: TypeInfo::Bool,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "map",
        name: "get",
        params: MAP_AND_STRING_PARAMS,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "map",
        name: "insert",
        params: MAP_STRING_AND_UNKNOWN_PARAMS,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "map",
        name: "remove",
        params: MAP_AND_STRING_PARAMS,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
];
