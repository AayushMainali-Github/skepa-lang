use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const NO_PARAMS: &[TypeInfo] = &[];

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "task",
        name: "__testTask",
        params: NO_PARAMS,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "task",
        name: "__testChannel",
        params: NO_PARAMS,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "task",
        name: "channel",
        params: NO_PARAMS,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "task",
        name: "send",
        params: &[TypeInfo::Unknown, TypeInfo::Unknown],
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "task",
        name: "recv",
        params: &[TypeInfo::Unknown],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
];
