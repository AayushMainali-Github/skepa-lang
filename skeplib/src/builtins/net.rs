use crate::types::TypeInfo;

use super::{BuiltinKind, BuiltinSig};

const NO_PARAMS: &[TypeInfo] = &[];

pub(super) const SIGS: &[BuiltinSig] = &[BuiltinSig {
    package: "net",
    name: "__testSocket",
    params: NO_PARAMS,
    ret: TypeInfo::Unknown,
    kind: BuiltinKind::FixedArity,
}];
