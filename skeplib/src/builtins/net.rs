use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const NO_PARAMS: &[TypeInfo] = &[];
const STRING_PARAM: &[TypeInfo] = &[TypeInfo::String];
const STRING_AND_INT_PARAM: &[TypeInfo] = &[TypeInfo::String, TypeInfo::Int];
const OPAQUE_PARAM_SENTINEL: &[TypeInfo] = &[TypeInfo::Unknown];
const OPAQUE_AND_STRING_PARAM: &[TypeInfo] = &[TypeInfo::Unknown, TypeInfo::String];
const OPAQUE_AND_BYTES_PARAM: &[TypeInfo] = &[TypeInfo::Unknown, TypeInfo::Bytes];
const OPAQUE_AND_INT_PARAM: &[TypeInfo] = &[TypeInfo::Unknown, TypeInfo::Int];

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
        name: "tlsConnect",
        params: STRING_AND_INT_PARAM,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "resolve",
        params: STRING_PARAM,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "parseUrl",
        params: STRING_PARAM,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "accept",
        params: OPAQUE_PARAM_SENTINEL,
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "read",
        params: OPAQUE_PARAM_SENTINEL,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "write",
        params: OPAQUE_AND_STRING_PARAM,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "readBytes",
        params: OPAQUE_PARAM_SENTINEL,
        ret: TypeInfo::Bytes,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "writeBytes",
        params: OPAQUE_AND_BYTES_PARAM,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "readN",
        params: OPAQUE_AND_INT_PARAM,
        ret: TypeInfo::Bytes,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "localAddr",
        params: OPAQUE_PARAM_SENTINEL,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "peerAddr",
        params: OPAQUE_PARAM_SENTINEL,
        ret: TypeInfo::String,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "flush",
        params: OPAQUE_PARAM_SENTINEL,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "setReadTimeout",
        params: OPAQUE_AND_INT_PARAM,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "net",
        name: "setWriteTimeout",
        params: OPAQUE_AND_INT_PARAM,
        ret: TypeInfo::Void,
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
