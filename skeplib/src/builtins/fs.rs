use std::sync::LazyLock;

use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const STRING1: &[TypeInfo] = &[TypeInfo::String];
const STRING2: &[TypeInfo] = &[TypeInfo::String, TypeInfo::String];

fn result_bool_string() -> TypeInfo {
    TypeInfo::Result {
        ok: Box::new(TypeInfo::Bool),
        err: Box::new(TypeInfo::String),
    }
}

fn result_string_string() -> TypeInfo {
    TypeInfo::Result {
        ok: Box::new(TypeInfo::String),
        err: Box::new(TypeInfo::String),
    }
}

fn result_void_string() -> TypeInfo {
    TypeInfo::Result {
        ok: Box::new(TypeInfo::Void),
        err: Box::new(TypeInfo::String),
    }
}

pub(super) static SIGS: LazyLock<Vec<BuiltinSig>> = LazyLock::new(|| {
    vec![
        BuiltinSig {
            package: "fs",
            name: "exists",
            params: STRING1,
            ret: result_bool_string(),
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "fs",
            name: "readText",
            params: STRING1,
            ret: result_string_string(),
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "fs",
            name: "writeText",
            params: STRING2,
            ret: result_void_string(),
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "fs",
            name: "appendText",
            params: STRING2,
            ret: result_void_string(),
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "fs",
            name: "mkdirAll",
            params: STRING1,
            ret: result_void_string(),
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "fs",
            name: "removeFile",
            params: STRING1,
            ret: result_void_string(),
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "fs",
            name: "removeDirAll",
            params: STRING1,
            ret: result_void_string(),
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "fs",
            name: "join",
            params: STRING2,
            ret: TypeInfo::String,
            kind: BuiltinKind::FixedArity,
        },
    ]
});
