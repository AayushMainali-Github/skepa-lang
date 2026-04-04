use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "result",
        name: "ok",
        params: &[TypeInfo::Unknown],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "result",
        name: "err",
        params: &[TypeInfo::Unknown],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
];
