use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "vec",
        name: "new",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "vec",
        name: "len",
        params: &[],
        ret: TypeInfo::Int,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "vec",
        name: "push",
        params: &[],
        ret: TypeInfo::Void,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "vec",
        name: "get",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "vec",
        name: "set",
        params: &[],
        ret: TypeInfo::Void,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "vec",
        name: "delete",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
];
