use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const DATETIME_NOW_PARAMS: &[TypeInfo] = &[];

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "datetime",
        name: "nowUnix",
        params: DATETIME_NOW_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "datetime",
        name: "nowMillis",
        params: DATETIME_NOW_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
];

