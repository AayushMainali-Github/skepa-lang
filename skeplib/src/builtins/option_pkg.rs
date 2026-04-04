use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const NO_PARAMS: &[TypeInfo] = &[];

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "option",
        name: "some",
        params: &[TypeInfo::Unknown],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "option",
        name: "none",
        params: NO_PARAMS,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
];
