use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "bytes",
        name: "fromString",
        params: &[TypeInfo::String],
        ret: TypeInfo::Bytes,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "bytes",
        name: "toString",
        params: &[TypeInfo::Bytes],
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "bytes",
        name: "len",
        params: &[TypeInfo::Bytes],
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
];
