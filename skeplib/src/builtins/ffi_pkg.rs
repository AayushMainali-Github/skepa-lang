use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const STRING_PARAM: &[TypeInfo] = &[TypeInfo::String];
const OPAQUE_PARAM_SENTINEL: &[TypeInfo] = &[TypeInfo::Unknown];
const OPAQUE_AND_STRING_PARAM: &[TypeInfo] = &[TypeInfo::Unknown, TypeInfo::String];

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "ffi",
        name: "open",
        params: STRING_PARAM,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "ffi",
        name: "bind",
        params: OPAQUE_AND_STRING_PARAM,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "ffi",
        name: "closeLibrary",
        params: OPAQUE_PARAM_SENTINEL,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "ffi",
        name: "closeSymbol",
        params: OPAQUE_PARAM_SENTINEL,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
];
