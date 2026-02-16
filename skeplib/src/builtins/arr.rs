use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "arr",
        name: "len",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "isEmpty",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "contains",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "indexOf",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "sum",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "count",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "first",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "last",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "reverse",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "join",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "slice",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "min",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "max",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
    BuiltinSig {
        package: "arr",
        name: "sort",
        params: &[],
        ret: TypeInfo::Unknown,
        kind: BuiltinKind::ArrayOps,
    },
];
