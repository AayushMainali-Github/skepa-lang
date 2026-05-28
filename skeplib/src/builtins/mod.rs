use crate::types::TypeInfo;

mod arr;
mod bytes_pkg;
mod datetime;
mod ffi_pkg;
mod fs;
mod io;
mod map_pkg;
mod net;
mod option_pkg;
mod os;
mod random;
mod result_pkg;
mod str_pkg;
mod task;
mod vec_pkg;

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
pub enum BuiltinVisibility {
    Public,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinMeta {
    pub purity: BuiltinPurity,
    pub lowering: BuiltinLowering,
    pub can_const_fold: bool,
    pub runtime_helper: Option<&'static str>,
    pub visibility: BuiltinVisibility,
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
    let spec = find_builtin_spec(package, name)?;
    Some(spec.sig)
}

pub fn find_builtin_sig_any(package: &str, name: &str) -> Option<&'static BuiltinSig> {
    io::SIGS
        .iter()
        .chain(bytes_pkg::SIGS.iter())
        .chain(map_pkg::SIGS.iter())
        .chain(str_pkg::SIGS.iter())
        .chain(arr::SIGS.iter())
        .chain(datetime::SIGS.iter())
        .chain(ffi_pkg::SIGS.iter())
        .chain(fs::SIGS.iter())
        .chain(net::SIGS.iter())
        .chain(os::SIGS.iter())
        .chain(option_pkg::SIGS.iter())
        .chain(result_pkg::SIGS.iter())
        .chain(random::SIGS.iter())
        .chain(task::SIGS.iter())
        .chain(vec_pkg::SIGS.iter())
        .find(|s| s.package == package && s.name == name)
}

pub fn find_builtin_spec(package: &str, name: &str) -> Option<BuiltinSpec> {
    let spec = find_builtin_spec_any(package, name)?;
    (spec.meta.visibility == BuiltinVisibility::Public).then_some(spec)
}

pub fn find_builtin_spec_any(package: &str, name: &str) -> Option<BuiltinSpec> {
    let sig = find_builtin_sig_any(package, name)?;
    Some(BuiltinSpec {
        sig,
        meta: builtin_meta(sig.package, sig.name),
    })
}

pub fn all_builtin_specs() -> impl Iterator<Item = BuiltinSpec> {
    all_builtin_specs_any().filter(|spec| spec.meta.visibility == BuiltinVisibility::Public)
}

pub fn all_builtin_specs_any() -> impl Iterator<Item = BuiltinSpec> {
    all_builtin_sigs_any().into_iter().map(|sig| BuiltinSpec {
        sig,
        meta: builtin_meta(sig.package, sig.name),
    })
}

fn all_builtin_sigs_any() -> Vec<&'static BuiltinSig> {
    io::SIGS
        .iter()
        .chain(bytes_pkg::SIGS.iter())
        .chain(map_pkg::SIGS.iter())
        .chain(str_pkg::SIGS.iter())
        .chain(arr::SIGS.iter())
        .chain(datetime::SIGS.iter())
        .chain(ffi_pkg::SIGS.iter())
        .chain(fs::SIGS.iter())
        .chain(net::SIGS.iter())
        .chain(os::SIGS.iter())
        .chain(option_pkg::SIGS.iter())
        .chain(result_pkg::SIGS.iter())
        .chain(random::SIGS.iter())
        .chain(task::SIGS.iter())
        .chain(vec_pkg::SIGS.iter())
        .collect()
}

fn builtin_meta(package: &str, name: &str) -> BuiltinMeta {
    match (package, name) {
        ("str", "len") | ("str", "contains") | ("str", "indexOf") => BuiltinMeta {
            purity: BuiltinPurity::Pure,
            lowering: BuiltinLowering::RuntimeCall,
            can_const_fold: true,
            runtime_helper: match name {
                "len" => Some("skp_rt_builtin_str_len"),
                "contains" => Some("skp_rt_builtin_str_contains"),
                "indexOf" => Some("skp_rt_builtin_str_index_of"),
                _ => None,
            },
            visibility: BuiltinVisibility::Public,
        },
        ("str", "startsWith")
        | ("str", "endsWith")
        | ("str", "trim")
        | ("str", "toLower")
        | ("str", "toUpper")
        | ("str", "slice")
        | ("str", "isEmpty")
        | ("str", "lastIndexOf")
        | ("str", "replace")
        | ("str", "repeat") => BuiltinMeta {
            purity: BuiltinPurity::Pure,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: true,
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
        },
        ("arr", _) => BuiltinMeta {
            purity: BuiltinPurity::Pure,
            lowering: BuiltinLowering::TypeDirected,
            can_const_fold: false,
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
        },
        ("vec", "len") | ("vec", "get") => BuiltinMeta {
            purity: BuiltinPurity::Pure,
            lowering: BuiltinLowering::TypeDirected,
            can_const_fold: false,
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
        },
        ("vec", "new") | ("vec", "push") | ("vec", "set") | ("vec", "delete") => BuiltinMeta {
            purity: BuiltinPurity::HostEffectful,
            lowering: BuiltinLowering::TypeDirected,
            can_const_fold: false,
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
        },
        ("io", "format") => BuiltinMeta {
            purity: BuiltinPurity::Pure,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
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
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
        },
        ("datetime", "nowUnix") | ("datetime", "nowMillis") => BuiltinMeta {
            purity: BuiltinPurity::HostStateful,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
        },
        ("option", _) => BuiltinMeta {
            purity: BuiltinPurity::Pure,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
        },
        ("result", _) => BuiltinMeta {
            purity: BuiltinPurity::Pure,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
        },
        ("datetime", _)
        | ("ffi", _)
        | ("fs", _)
        | ("net", _)
        | ("os", _)
        | ("random", _)
        | ("task", _) => BuiltinMeta {
            purity: BuiltinPurity::HostEffectful,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
            runtime_helper: None,
            visibility: if (package == "ffi" && name.starts_with("call"))
                || (package == "task" && name.starts_with("__test"))
                || (package == "net" && name.starts_with("__test"))
            {
                BuiltinVisibility::Internal
            } else {
                BuiltinVisibility::Public
            },
        },
        ("bytes", "len") | ("map", "len") | ("map", "has") => BuiltinMeta {
            purity: BuiltinPurity::Pure,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
        },
        ("map", "new") | ("map", "insert") | ("map", "remove") | ("map", "get") => BuiltinMeta {
            purity: BuiltinPurity::HostEffectful,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
        },
        _ => BuiltinMeta {
            purity: BuiltinPurity::HostEffectful,
            lowering: BuiltinLowering::GenericDispatch,
            can_const_fold: false,
            runtime_helper: None,
            visibility: BuiltinVisibility::Public,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BuiltinLowering, BuiltinPurity, BuiltinVisibility, all_builtin_specs,
        all_builtin_specs_any, find_builtin_sig, find_builtin_sig_any, find_builtin_spec,
        find_builtin_spec_any,
    };

    #[test]
    fn builtin_registry_exposes_signature_and_metadata_for_known_entries() {
        let spec = find_builtin_spec("str", "slice").expect("string builtin should exist");
        assert_eq!(spec.sig.package, "str");
        assert_eq!(spec.sig.name, "slice");
        assert_eq!(spec.meta.purity, BuiltinPurity::Pure);
        assert_eq!(spec.meta.lowering, BuiltinLowering::GenericDispatch);
        assert!(spec.meta.can_const_fold);
        assert_eq!(spec.meta.runtime_helper, None);
        assert_eq!(spec.meta.visibility, BuiltinVisibility::Public);
    }

    #[test]
    fn builtin_registry_covers_all_known_signatures() {
        let sig_count = all_builtin_specs_any().count();
        let manual_count = [
            super::io::SIGS.len(),
            super::bytes_pkg::SIGS.len(),
            super::map_pkg::SIGS.len(),
            super::str_pkg::SIGS.len(),
            super::arr::SIGS.len(),
            super::datetime::SIGS.len(),
            super::ffi_pkg::SIGS.len(),
            super::fs::SIGS.len(),
            super::net::SIGS.len(),
            super::os::SIGS.len(),
            super::option_pkg::SIGS.len(),
            super::result_pkg::SIGS.len(),
            super::random::SIGS.len(),
            super::task::SIGS.len(),
            super::vec_pkg::SIGS.len(),
        ]
        .into_iter()
        .sum::<usize>();
        assert_eq!(sig_count, manual_count);
        assert!(find_builtin_sig("datetime", "nowUnix").is_some());
        assert!(find_builtin_spec("vec", "push").is_some());
        assert!(find_builtin_spec("bytes", "fromString").is_some());
        assert!(find_builtin_spec("map", "new").is_some());
        assert!(find_builtin_spec("task", "__testTask").is_none());
        assert!(find_builtin_sig("task", "__testTask").is_none());
        assert!(find_builtin_spec_any("task", "__testTask").is_some());
        assert!(find_builtin_sig_any("task", "__testTask").is_some());
        assert!(find_builtin_spec("ffi", "call0Int").is_none());
        assert!(find_builtin_spec_any("ffi", "call0Int").is_some());
        assert!(find_builtin_spec("missing", "name").is_none());
    }

    #[test]
    fn public_builtin_registry_hides_internal_helpers() {
        let public = all_builtin_specs()
            .map(|spec| (spec.sig.package, spec.sig.name))
            .collect::<Vec<_>>();
        assert!(!public.contains(&("ffi", "call0Int")));
        assert!(!public.contains(&("task", "__testTask")));
        assert!(!public.contains(&("net", "__testSocket")));
        assert!(public.contains(&("ffi", "open")));
        assert!(public.contains(&("task", "spawn")));
        assert!(public.contains(&("net", "connect")));
    }
}
