use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const STRING_PARAM: &[TypeInfo] = &[TypeInfo::String];
const OPAQUE_PARAM_SENTINEL: &[TypeInfo] = &[TypeInfo::Unknown];
const OPAQUE_AND_STRING_PARAM: &[TypeInfo] = &[TypeInfo::Unknown, TypeInfo::String];
const OPAQUE_AND_INT_PARAM: &[TypeInfo] = &[TypeInfo::Unknown, TypeInfo::Int];
const OPAQUE_AND_BYTES_PARAM: &[TypeInfo] = &[TypeInfo::Unknown, TypeInfo::Bytes];

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
    BuiltinSig {
        package: "ffi",
        name: "call0Int",
        params: OPAQUE_PARAM_SENTINEL,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "ffi",
        name: "call1Int",
        params: OPAQUE_AND_INT_PARAM,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "ffi",
        name: "call1IntVoid",
        params: OPAQUE_AND_INT_PARAM,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "ffi",
        name: "call1StringInt",
        params: OPAQUE_AND_STRING_PARAM,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "ffi",
        name: "call1StringVoid",
        params: OPAQUE_AND_STRING_PARAM,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "ffi",
        name: "call2StringInt",
        params: &[TypeInfo::Unknown, TypeInfo::String, TypeInfo::String],
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "ffi",
        name: "call2StringIntInt",
        params: &[TypeInfo::Unknown, TypeInfo::String, TypeInfo::Int],
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "ffi",
        name: "call1BytesInt",
        params: OPAQUE_AND_BYTES_PARAM,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
];
