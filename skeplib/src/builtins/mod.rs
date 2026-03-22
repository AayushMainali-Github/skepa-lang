use crate::types::TypeInfo;

mod arr;
mod datetime;
mod fs;
mod io;
mod os;
mod random;
mod str_pkg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinKind {
    FixedArity,
    FormatVariadic,
    ArrayOps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinPurity {
    Pure,
    HostStateful,
    HostEffectful,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinLowering {
    RuntimeCall,
    GenericDispatch,
    TypeDirected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinMeta {
    pub purity: BuiltinPurity,
    pub lowering: BuiltinLowering,
    pub can_const_fold: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinSig {
    pub package: &'static str,
    pub name: &'static str,
    pub params: &'static [TypeInfo],
    pub ret: TypeInfo,
    pub kind: BuiltinKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinSpec {
    pub sig: &'static BuiltinSig,
    pub meta: BuiltinMeta,
}

pub fn find_builtin_sig(package: &str, name: &str) -> Option<&'static BuiltinSig> {
    io::SIGS
        .iter()
        .chain(str_pkg::SIGS.iter())
        .chain(arr::SIGS.iter())
        .chain(datetime::SIGS.iter())
        .chain(fs::SIGS.iter())
        .chain(os::SIGS.iter())
        .chain(random::SIGS.iter())
        .find(|s| s.package == package && s.name == name)
}

pub fn find_builtin_spec(package: &str, name: &str) -> Option<BuiltinSpec> {
    let sig = find_builtin_sig(package, name)?;
    Some(BuiltinSpec {
        sig,
        meta: builtin_meta(sig.package, sig.name),
    })
}

pub fn all_builtin_specs() -> impl Iterator<Item = BuiltinSpec> {
    all_builtin_sigs().into_iter().map(|sig| BuiltinSpec {
        sig,
        meta: builtin_meta(sig.package, sig.name),
    })
}

fn all_builtin_sigs() -> Vec<&'static BuiltinSig> {
    io::SIGS
        .iter()
        .chain(str_pkg::SIGS.iter())
        .chain(arr::SIGS.iter())
        .chain(datetime::SIGS.iter())
        .chain(fs::SIGS.iter())
        .chain(os::SIGS.iter())
        .chain(random::SIGS.iter())
        .collect()
}

fn builtin_meta(package: &str, name: &str) -> BuiltinMeta {
    match (package, name) {
        ("str", "len")
        | ("str", "contains")
        | ("str", "startsWith")
        | ("str", "endsWith")
        | ("str", "trim")
        | ("str", "toLower")
        | ("str", "toUpper")
        | ("str", "indexOf")
        | ("str", "slice")
        | ("str", "isEmpty")
        | ("str", "lastIndexOf")
        | ("str", "replace")
        | ("str", "repeat") => BuiltinMeta {
            purity: BuiltinPurity::Pure,
            lowering: BuiltinLowering::RuntimeCall,
            can_const_fold: matches!(
                name,
                "len"
                    | "contains"
                    | "startsWith"
                    | "endsWith"
                    | "trim"
                    | "toLower"
                    | "toUpper"
                    | "indexOf"
                    | "slice"
                    | "isEmpty"
                    | "lastIndexOf"
                    | "replace"
                    | "repeat"
            ),
        },
        ("arr", _) => BuiltinMeta {
            purity: BuiltinPurity::Pure,
            lowering: BuiltinLowering::TypeDirected,
            can_const_fold: false,
        },
        ("io", "format") => BuiltinMeta {
            purity: BuiltinPurity::Pure,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
        },
        ("io", "print")
        | ("io", "println")
        | ("io", "printInt")
        | ("io", "printFloat")
        | ("io", "printBool")
        | ("io", "printString")
        | ("io", "printf")
        | ("io", "readLine") => BuiltinMeta {
            purity: BuiltinPurity::HostEffectful,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
        },
        ("datetime", "nowUnix") | ("datetime", "nowMillis") => BuiltinMeta {
            purity: BuiltinPurity::HostStateful,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
        },
        ("datetime", _) | ("fs", _) | ("os", _) | ("random", _) => BuiltinMeta {
            purity: BuiltinPurity::HostEffectful,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
        },
        _ => BuiltinMeta {
            purity: BuiltinPurity::HostEffectful,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BuiltinLowering, BuiltinPurity, all_builtin_specs, find_builtin_sig, find_builtin_spec,
    };

    #[test]
    fn builtin_registry_exposes_signature_and_metadata_for_known_entries() {
        let spec = find_builtin_spec("str", "slice").expect("string builtin should exist");
        assert_eq!(spec.sig.package, "str");
        assert_eq!(spec.sig.name, "slice");
        assert_eq!(spec.meta.purity, BuiltinPurity::Pure);
        assert_eq!(spec.meta.lowering, BuiltinLowering::RuntimeCall);
        assert!(spec.meta.can_const_fold);
    }

    #[test]
    fn builtin_registry_covers_all_known_signatures() {
        let sig_count = all_builtin_specs().count();
        let manual_count = [
            super::io::SIGS.len(),
            super::str_pkg::SIGS.len(),
            super::arr::SIGS.len(),
            super::datetime::SIGS.len(),
            super::fs::SIGS.len(),
            super::os::SIGS.len(),
            super::random::SIGS.len(),
        ]
        .into_iter()
        .sum::<usize>();
        assert_eq!(sig_count, manual_count);
        assert!(find_builtin_sig("datetime", "nowUnix").is_some());
        assert!(find_builtin_spec("missing", "name").is_none());
    }
}
