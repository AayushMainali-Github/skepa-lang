use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const RANDOM_SEED_PARAMS: &[TypeInfo] = &[TypeInfo::Int];
const RANDOM_NO_PARAMS: &[TypeInfo] = &[];

pub(super) const SIGS: &[BuiltinSig] = &[
    BuiltinSig {
        package: "random",
        name: "seed",
        params: RANDOM_SEED_PARAMS,
        ret: TypeInfo::Void,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "random",
        name: "int",
        params: RANDOM_NO_PARAMS,
        ret: TypeInfo::Int,
        kind: BuiltinKind::FixedArity,
    },
    BuiltinSig {
        package: "random",
        name: "float",
        params: RANDOM_NO_PARAMS,
        ret: TypeInfo::Float,
        kind: BuiltinKind::FixedArity,
    },
];
