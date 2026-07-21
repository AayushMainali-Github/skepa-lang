use std::sync::LazyLock;

use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const NO_PARAMS: &[TypeInfo] = &[];
const INT_PARAM: &[TypeInfo] = &[TypeInfo::Int];
const STRING_PARAM: &[TypeInfo] = &[TypeInfo::String];
const STRING2_PARAMS: &[TypeInfo] = &[TypeInfo::String, TypeInfo::String];

fn string_and_vec_string_params() -> &'static [TypeInfo] {
    Box::leak(Box::new([
        TypeInfo::String,
        TypeInfo::Vec {
            elem: Box::new(TypeInfo::String),
        },
    ]))
}

fn result_int_string() -> TypeInfo {
    TypeInfo::Result {
        ok: Box::new(TypeInfo::Int),
        err: Box::new(TypeInfo::String),
    }
}

fn result_string_string() -> TypeInfo {
    TypeInfo::Result {
        ok: Box::new(TypeInfo::String),
        err: Box::new(TypeInfo::String),
    }
}

pub(super) static SIGS: LazyLock<Vec<BuiltinSig>> = LazyLock::new(|| {
    let string_and_vec_string = string_and_vec_string_params();
    vec![
        BuiltinSig {
            package: "os",
            name: "platform",
            params: NO_PARAMS,
            ret: TypeInfo::String,
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "os",
            name: "arch",
            params: NO_PARAMS,
            ret: TypeInfo::String,
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "os",
            name: "arg",
            params: INT_PARAM,
            ret: TypeInfo::Unknown,
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "os",
            name: "envHas",
            params: STRING_PARAM,
            ret: TypeInfo::Bool,
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "os",
            name: "envGet",
            params: STRING_PARAM,
            ret: TypeInfo::Unknown,
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "os",
            name: "envSet",
            params: STRING2_PARAMS,
            ret: TypeInfo::Void,
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "os",
            name: "envRemove",
            params: STRING_PARAM,
            ret: TypeInfo::Void,
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "os",
            name: "sleep",
            params: INT_PARAM,
            ret: TypeInfo::Void,
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "os",
            name: "exit",
            params: INT_PARAM,
            ret: TypeInfo::Void,
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "os",
            name: "exec",
            params: string_and_vec_string,
            ret: result_int_string(),
            kind: BuiltinKind::FixedArity,
        },
        BuiltinSig {
            package: "os",
            name: "execOut",
            params: string_and_vec_string,
            ret: result_string_string(),
            kind: BuiltinKind::FixedArity,
        },
    ]
});
