use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const DATETIME_NOW_PARAMS: &[TypeInfo] = &[];
const DATETIME_UNIX_PARAMS: &[TypeInfo] = &[TypeInfo::Int];
const DATETIME_PARSE_PARAMS: &[TypeInfo] = &[TypeInfo::String];

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
    BuiltinSig {
        package: "datetime",
        name: "fromUnix",
        params: DATETIME_UNIX_PARAMS,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "datetime",
        name: "fromMillis",
        params: DATETIME_UNIX_PARAMS,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "datetime",
        name: "parseUnix",
        params: DATETIME_PARSE_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "datetime",
        name: "year",
        params: DATETIME_UNIX_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "datetime",
        name: "month",
        params: DATETIME_UNIX_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "datetime",
        name: "day",
        params: DATETIME_UNIX_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "datetime",
        name: "hour",
        params: DATETIME_UNIX_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "datetime",
        name: "minute",
        params: DATETIME_UNIX_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "datetime",
        name: "second",
        params: DATETIME_UNIX_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
];
