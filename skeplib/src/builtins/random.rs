use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const RANDOM_SEED_PARAMS: &[TypeInfo] = &[TypeInfo::Int];

pub(super) const SIGS: &[BuiltinSig] = &[BuiltinSig {
    package: "random",
    name: "seed",
    params: RANDOM_SEED_PARAMS,
    ret: TypeInfo::Void,
    kind: BuiltinKind::FixedArity,
}];

