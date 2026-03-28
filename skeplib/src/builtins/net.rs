use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const NO_PARAMS: &[TypeInfo] = &[];
const STRING_PARAM: &[TypeInfo] = &[TypeInfo::String];
const OPAQUE_PARAM_SENTINEL: &[TypeInfo] = &[TypeInfo::Unknown];

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "net",
        name: "__testSocket",
        params: NO_PARAMS,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "listen",
        params: STRING_PARAM,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "connect",
        params: STRING_PARAM,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "accept",
        params: STRING_PARAM,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "close",
        params: OPAQUE_PARAM_SENTINEL,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "closeListener",
        params: OPAQUE_PARAM_SENTINEL,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
];
