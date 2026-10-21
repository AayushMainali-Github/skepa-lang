use crate::ast::TypeName;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeInfo {
    Int,
    Float,
    Bool,
    String,
    Bytes,
    Void,
    Option {
        value: Box<TypeInfo>,
    },
    Result {
        ok: Box<TypeInfo>,
        err: Box<TypeInfo>,
    },
    Named(String),
    Opaque(String),
    Array {
        elem: Box<TypeInfo>,
        size: usize,
    },
    Vec {
        elem: Box<TypeInfo>,
    },
    Map {
        value: Box<TypeInfo>,
    },
    Fn {
        params: Vec<TypeInfo>,
        ret: Box<TypeInfo>,
    },
    Unknown,
}

/// The source-level concurrency capability of a type.
///
/// `Shareable` is the only capability currently accepted by task and channel
/// boundaries. The other variants make the policy explicit for future
/// ownership-transfer and actor work rather than inferring it from Rust
/// representation details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcurrencyCapability {
    Shareable,
    TransferOnly,
    ActorLocal,
    LocalOnly,
    Unknown,
}

impl ConcurrencyCapability {
    /// Whether a value may cross a task or actor boundary under the target
    /// contract. Shareable values may be aliased safely; TransferOnly values
    /// are sendable only when ownership is transferred.
    pub const fn is_sendable(self) -> bool {
        matches!(self, Self::Shareable | Self::TransferOnly)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareabilityFailure {
    pub capability: ConcurrencyCapability,
    /// Field/container path from the checked value to the offending type.
    pub path: Vec<String>,
    pub ty: TypeInfo,
}

/// Return the first non-shareable type in a value's recursively contained
/// fields. Struct fields are visited in name order so diagnostics are stable.
pub fn shareability_failure(
    ty: &TypeInfo,
    struct_fields: &HashMap<String, HashMap<String, TypeInfo>>,
) -> Option<ShareabilityFailure> {
    shareability_failure_with(ty, struct_fields, &|name| {
        (struct_fields.contains_key(name) || is_builtin_opaque_type(name)).then(|| name.to_string())
    })
}

/// Classify a type using the current source-level concurrency rules.
pub fn type_concurrency_capability(
    ty: &TypeInfo,
    struct_fields: &HashMap<String, HashMap<String, TypeInfo>>,
) -> ConcurrencyCapability {
    type_concurrency_capability_with(ty, struct_fields, &|name| {
        (struct_fields.contains_key(name) || is_builtin_opaque_type(name)).then(|| name.to_string())
    })
}

/// Classify a type with frontend-specific name resolution for imported or
/// aliased structs.
pub fn type_concurrency_capability_with(
    ty: &TypeInfo,
    struct_fields: &HashMap<String, HashMap<String, TypeInfo>>,
    resolve_named: &dyn Fn(&str) -> Option<String>,
) -> ConcurrencyCapability {
    shareability_failure_with(ty, struct_fields, resolve_named)
        .map(|failure| failure.capability)
        .unwrap_or(ConcurrencyCapability::Shareable)
}

/// Variant of [`shareability_failure`] for frontends that resolve imported or
/// aliased struct names while traversing the type graph.
pub fn shareability_failure_with(
    ty: &TypeInfo,
    struct_fields: &HashMap<String, HashMap<String, TypeInfo>>,
    resolve_named: &dyn Fn(&str) -> Option<String>,
) -> Option<ShareabilityFailure> {
    fn failure(
        capability: ConcurrencyCapability,
        path: Vec<String>,
        ty: &TypeInfo,
    ) -> Option<ShareabilityFailure> {
        Some(ShareabilityFailure {
            capability,
            path,
            ty: ty.clone(),
        })
    }

    fn visit(
        ty: &TypeInfo,
        struct_fields: &HashMap<String, HashMap<String, TypeInfo>>,
        resolve_named: &dyn Fn(&str) -> Option<String>,
        path: Vec<String>,
        visiting: &mut HashSet<String>,
    ) -> Option<ShareabilityFailure> {
        match ty {
            TypeInfo::Int
            | TypeInfo::Float
            | TypeInfo::Bool
            | TypeInfo::String
            | TypeInfo::Bytes
            | TypeInfo::Void => None,
            TypeInfo::Option { value } => {
                let mut inner_path = path;
                inner_path.push("Some".to_string());
                visit(value, struct_fields, resolve_named, inner_path, visiting)
            }
            TypeInfo::Result { ok, err } => {
                let mut ok_path = path.clone();
                ok_path.push("Ok".to_string());
                visit(ok, struct_fields, resolve_named, ok_path, visiting).or_else(|| {
                    let mut err_path = path;
                    err_path.push("Err".to_string());
                    visit(err, struct_fields, resolve_named, err_path, visiting)
                })
            }
            TypeInfo::Array { elem, .. } => {
                let mut elem_path = path;
                elem_path.push("[]".to_string());
                visit(elem, struct_fields, resolve_named, elem_path, visiting)
            }
            TypeInfo::Named(name) => {
                let Some(resolved) = resolve_named(name) else {
                    return failure(ConcurrencyCapability::Unknown, path, ty);
                };
                let Some(fields) = struct_fields.get(&resolved) else {
                    return failure(
                        if is_builtin_opaque_type(&resolved) {
                            ConcurrencyCapability::LocalOnly
                        } else {
                            ConcurrencyCapability::Unknown
                        },
                        path,
                        ty,
                    );
                };
                if !visiting.insert(resolved.clone()) {
                    return failure(ConcurrencyCapability::Unknown, path, ty);
                }
                let mut names = fields.keys().collect::<Vec<_>>();
                names.sort();
                let nested_failure = names.into_iter().find_map(|name| {
                    let field_ty = fields.get(name)?;
                    let mut field_path = path.clone();
                    field_path.push(name.clone());
                    visit(field_ty, struct_fields, resolve_named, field_path, visiting)
                });
                visiting.remove(&resolved);
                nested_failure
            }
            TypeInfo::Opaque(name) => failure(
                if is_builtin_opaque_type(name) {
                    ConcurrencyCapability::LocalOnly
                } else {
                    ConcurrencyCapability::Unknown
                },
                path,
                ty,
            ),
            TypeInfo::Vec { .. } | TypeInfo::Map { .. } | TypeInfo::Fn { .. } => {
                failure(ConcurrencyCapability::LocalOnly, path, ty)
            }
            TypeInfo::Unknown => failure(ConcurrencyCapability::Unknown, path, ty),
        }
    }

    visit(
        ty,
        struct_fields,
        resolve_named,
        Vec::new(),
        &mut HashSet::new(),
    )
}

impl TypeInfo {
    pub fn from_ast(ty: &TypeName) -> Self {
        match ty {
            TypeName::Int => TypeInfo::Int,
            TypeName::Float => TypeInfo::Float,
            TypeName::Bool => TypeInfo::Bool,
            TypeName::String => TypeInfo::String,
            TypeName::Bytes => TypeInfo::Bytes,
            TypeName::Void => TypeInfo::Void,
            TypeName::Option { value } => TypeInfo::Option {
                value: Box::new(TypeInfo::from_ast(value)),
            },
            TypeName::Result { ok, err } => TypeInfo::Result {
                ok: Box::new(TypeInfo::from_ast(ok)),
                err: Box::new(TypeInfo::from_ast(err)),
            },
            TypeName::Named(name) => {
                if is_builtin_opaque_type(name) {
                    TypeInfo::Opaque(name.clone())
                } else {
                    TypeInfo::Named(name.clone())
                }
            }
            TypeName::Array { elem, size } => TypeInfo::Array {
                elem: Box::new(TypeInfo::from_ast(elem)),
                size: *size,
            },
            TypeName::Vec { elem } => TypeInfo::Vec {
                elem: Box::new(TypeInfo::from_ast(elem)),
            },
            TypeName::Map { value } => TypeInfo::Map {
                value: Box::new(TypeInfo::from_ast(value)),
            },
            TypeName::Fn { params, ret } => TypeInfo::Fn {
                params: params.iter().map(TypeInfo::from_ast).collect(),
                ret: Box::new(TypeInfo::from_ast(ret)),
            },
        }
    }
}

pub fn task_channel_type_name(value: &TypeInfo) -> String {
    format!("task.Channel[{}]", display_type(value))
}

pub fn task_task_type_name(value: &TypeInfo) -> String {
    format!("task.Task[{}]", display_type(value))
}

pub fn task_channel_type(value: &TypeInfo) -> TypeInfo {
    TypeInfo::Opaque(task_channel_type_name(value))
}

pub fn task_task_type(value: &TypeInfo) -> TypeInfo {
    TypeInfo::Opaque(task_task_type_name(value))
}

pub fn task_channel_value_type(name: &str) -> Option<TypeInfo> {
    let inner = name.strip_prefix("task.Channel[")?.strip_suffix(']')?;
    parse_display_type(inner)
}

pub fn task_task_value_type(name: &str) -> Option<TypeInfo> {
    let inner = name.strip_prefix("task.Task[")?.strip_suffix(']')?;
    parse_display_type(inner)
}

pub fn task_opaque_inner_type_name(name: &str) -> Option<&str> {
    name.strip_prefix("task.Channel[")
        .or_else(|| name.strip_prefix("task.Task["))?
        .strip_suffix(']')
}

pub fn display_type(value: &TypeInfo) -> String {
    match value {
        TypeInfo::Int => "Int".to_string(),
        TypeInfo::Float => "Float".to_string(),
        TypeInfo::Bool => "Bool".to_string(),
        TypeInfo::String => "String".to_string(),
        TypeInfo::Bytes => "Bytes".to_string(),
        TypeInfo::Void => "Void".to_string(),
        TypeInfo::Option { value } => format!("Option[{}]", display_type(value)),
        TypeInfo::Result { ok, err } => {
            format!("Result[{}, {}]", display_type(ok), display_type(err))
        }
        TypeInfo::Named(name) | TypeInfo::Opaque(name) => name.clone(),
        TypeInfo::Array { elem, size } => format!("[{}; {}]", display_type(elem), size),
        TypeInfo::Vec { elem } => format!("Vec[{}]", display_type(elem)),
        TypeInfo::Map { value } => format!("Map[String, {}]", display_type(value)),
        TypeInfo::Fn { params, ret } => format!(
            "Fn({}) -> {}",
            params
                .iter()
                .map(display_type)
                .collect::<Vec<_>>()
                .join(", "),
            display_type(ret)
        ),
        TypeInfo::Unknown => "Unknown".to_string(),
    }
}

fn parse_display_type(value: &str) -> Option<TypeInfo> {
    match value {
        "Int" => Some(TypeInfo::Int),
        "Float" => Some(TypeInfo::Float),
        "Bool" => Some(TypeInfo::Bool),
        "String" => Some(TypeInfo::String),
        "Bytes" => Some(TypeInfo::Bytes),
        "Void" => Some(TypeInfo::Void),
        _ => {
            if let Some(inner) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']'))
                && let Some((elem, size)) = split_top_level(inner, ';')
            {
                return Some(TypeInfo::Array {
                    elem: Box::new(parse_display_type(elem)?),
                    size: size.trim().parse().ok()?,
                });
            }
            if value.starts_with("Fn(")
                && let Some((params, ret)) = split_function_signature(value)
            {
                let params = split_top_level_items(params)?
                    .into_iter()
                    .map(parse_display_type)
                    .collect::<Option<Vec<_>>>()?;
                return Some(TypeInfo::Fn {
                    params,
                    ret: Box::new(parse_display_type(ret)?),
                });
            }
            if let Some(inner) = value.strip_prefix("Vec[").and_then(|v| v.strip_suffix(']')) {
                return Some(TypeInfo::Vec {
                    elem: Box::new(parse_display_type(inner)?),
                });
            }
            if let Some(inner) = value
                .strip_prefix("Option[")
                .and_then(|v| v.strip_suffix(']'))
            {
                return Some(TypeInfo::Option {
                    value: Box::new(parse_display_type(inner)?),
                });
            }
            if let Some(inner) = value
                .strip_prefix("Result[")
                .and_then(|v| v.strip_suffix(']'))
                && let Some((ok, err)) = split_top_level_comma(inner)
            {
                return Some(TypeInfo::Result {
                    ok: Box::new(parse_display_type(ok)?),
                    err: Box::new(parse_display_type(err)?),
                });
            }
            if let Some(inner) = value
                .strip_prefix("Map[String, ")
                .and_then(|v| v.strip_suffix(']'))
            {
                return Some(TypeInfo::Map {
                    value: Box::new(parse_display_type(inner)?),
                });
            }
            if (value.starts_with("task.Channel[") || value.starts_with("task.Task["))
                && value.ends_with(']')
            {
                return Some(TypeInfo::Opaque(value.to_string()));
            }
            if is_builtin_opaque_type(value) {
                return Some(TypeInfo::Opaque(value.to_string()));
            }
            Some(TypeInfo::Named(value.to_string()))
        }
    }
}

fn split_top_level_comma(value: &str) -> Option<(&str, &str)> {
    split_top_level(value, ',')
}

fn split_top_level(value: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.checked_sub(1)?,
            _ if ch == delimiter && depth == 0 => {
                let (left, right) = value.split_at(index);
                return Some((left.trim_end(), right[1..].trim_start()));
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_items(value: &str) -> Option<Vec<&str>> {
    if value.trim().is_empty() {
        return Some(Vec::new());
    }
    let mut items = Vec::new();
    let mut remaining = value;
    while let Some((item, rest)) = split_top_level(remaining, ',') {
        items.push(item.trim());
        remaining = rest;
    }
    items.push(remaining.trim());
    Some(items)
}

fn split_function_signature(value: &str) -> Option<(&str, &str)> {
    let signature = value.strip_prefix("Fn(")?;
    let mut depth = 0usize;
    for (index, ch) in signature.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' if depth == 0 => {
                let (params, suffix) = signature.split_at(index);
                return Some((params, suffix.strip_prefix(") -> ")?));
            }
            ')' => depth = depth.checked_sub(1)?,
            _ => {}
        }
    }
    None
}

pub fn is_builtin_opaque_type(name: &str) -> bool {
    matches!(
        name,
        "net.Socket" | "net.Listener" | "task.Task" | "task.Channel" | "ffi.Library" | "ffi.Symbol"
    ) || ((name.starts_with("task.Channel[") || name.starts_with("task.Task["))
        && name.ends_with(']'))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<TypeInfo>,
    pub ret: TypeInfo,
}

#[cfg(test)]
mod tests {
    use super::{
        ConcurrencyCapability, TypeInfo, shareability_failure, task_channel_value_type,
        task_task_value_type, type_concurrency_capability,
    };
    use std::collections::HashMap;

    #[test]
    fn task_value_type_parser_handles_nested_result_parameters() {
        let expected = TypeInfo::Result {
            ok: Box::new(TypeInfo::Result {
                ok: Box::new(TypeInfo::Int),
                err: Box::new(TypeInfo::String),
            }),
            err: Box::new(TypeInfo::Bool),
        };

        assert_eq!(
            task_channel_value_type("task.Channel[Result[Result[Int, String], Bool]]"),
            Some(expected)
        );
    }

    #[test]
    fn task_value_type_parser_handles_nested_map_parameters() {
        let expected = TypeInfo::Map {
            value: Box::new(TypeInfo::Result {
                ok: Box::new(TypeInfo::Int),
                err: Box::new(TypeInfo::String),
            }),
        };

        assert_eq!(
            task_task_value_type("task.Task[Map[String, Result[Int, String]]]"),
            Some(expected)
        );
    }

    #[test]
    fn task_value_type_parser_handles_array_and_function_parameters() {
        let expected = TypeInfo::Fn {
            params: vec![
                TypeInfo::Int,
                TypeInfo::Array {
                    elem: Box::new(TypeInfo::String),
                    size: 2,
                },
                TypeInfo::Fn {
                    params: vec![TypeInfo::Bool],
                    ret: Box::new(TypeInfo::String),
                },
            ],
            ret: Box::new(TypeInfo::Result {
                ok: Box::new(TypeInfo::Bool),
                err: Box::new(TypeInfo::String),
            }),
        };

        assert_eq!(
            task_task_value_type(
                "task.Task[Fn(Int, [String; 2], Fn(Bool) -> String) -> Result[Bool, String]]"
            ),
            Some(expected)
        );
    }

    #[test]
    fn shareability_is_derived_recursively_for_struct_fields() {
        let mut structs = HashMap::new();
        structs.insert(
            "Payload".to_string(),
            HashMap::from([
                (
                    "data".to_string(),
                    TypeInfo::Array {
                        elem: Box::new(TypeInfo::Option {
                            value: Box::new(TypeInfo::Result {
                                ok: Box::new(TypeInfo::Int),
                                err: Box::new(TypeInfo::String),
                            }),
                        }),
                        size: 2,
                    },
                ),
                ("tag".to_string(), TypeInfo::String),
            ]),
        );

        assert_eq!(
            shareability_failure(&TypeInfo::Named("Payload".to_string()), &structs),
            None
        );
    }

    #[test]
    fn shareability_failure_reports_the_first_nested_field_path() {
        let mut structs = HashMap::new();
        structs.insert(
            "Inner".to_string(),
            HashMap::from([(
                "mutable".to_string(),
                TypeInfo::Vec {
                    elem: Box::new(TypeInfo::Int),
                },
            )]),
        );
        structs.insert(
            "Envelope".to_string(),
            HashMap::from([("payload".to_string(), TypeInfo::Named("Inner".to_string()))]),
        );

        let failure = shareability_failure(&TypeInfo::Named("Envelope".to_string()), &structs)
            .expect("nested Vec should not be shareable");
        assert_eq!(failure.capability, ConcurrencyCapability::LocalOnly);
        assert_eq!(failure.path, vec!["payload", "mutable"]);
        assert_eq!(
            failure.ty,
            TypeInfo::Vec {
                elem: Box::new(TypeInfo::Int)
            }
        );
    }

    #[test]
    fn shareability_classifies_current_local_only_and_unknown_families() {
        let no_structs = HashMap::new();
        let local_only = [
            TypeInfo::Vec {
                elem: Box::new(TypeInfo::Int),
            },
            TypeInfo::Map {
                value: Box::new(TypeInfo::String),
            },
            TypeInfo::Fn {
                params: vec![TypeInfo::Int],
                ret: Box::new(TypeInfo::Int),
            },
            TypeInfo::Opaque("net.Socket".to_string()),
            TypeInfo::Opaque("net.Listener".to_string()),
            TypeInfo::Opaque("task.Channel[Int]".to_string()),
            TypeInfo::Opaque("task.Task[Int]".to_string()),
            TypeInfo::Opaque("ffi.Library".to_string()),
            TypeInfo::Opaque("ffi.Symbol".to_string()),
        ];
        for ty in local_only {
            assert_eq!(
                shareability_failure(&ty, &no_structs)
                    .expect("local-only type should fail")
                    .capability,
                ConcurrencyCapability::LocalOnly,
                "{ty:?}"
            );
        }

        for ty in [
            TypeInfo::Int,
            TypeInfo::Float,
            TypeInfo::Bool,
            TypeInfo::String,
            TypeInfo::Bytes,
            TypeInfo::Void,
        ] {
            assert_eq!(
                type_concurrency_capability(&ty, &no_structs),
                ConcurrencyCapability::Shareable,
                "{ty:?}"
            );
        }

        assert!(ConcurrencyCapability::Shareable.is_sendable());
        assert!(ConcurrencyCapability::TransferOnly.is_sendable());
        assert!(!ConcurrencyCapability::ActorLocal.is_sendable());
        assert!(!ConcurrencyCapability::LocalOnly.is_sendable());

        for ty in [
            TypeInfo::Unknown,
            TypeInfo::Named("Missing".to_string()),
            TypeInfo::Opaque("custom.Resource".to_string()),
        ] {
            assert_eq!(
                shareability_failure(&ty, &no_structs)
                    .expect("unresolved type should fail")
                    .capability,
                ConcurrencyCapability::Unknown,
                "{ty:?}"
            );
        }
    }

    #[test]
    fn recursive_structs_are_rejected_as_unresolved_at_boundaries() {
        let mut structs = HashMap::new();
        structs.insert(
            "Node".to_string(),
            HashMap::from([("next".to_string(), TypeInfo::Named("Node".to_string()))]),
        );

        let failure = shareability_failure(&TypeInfo::Named("Node".to_string()), &structs)
            .expect("recursive value layout should be rejected");
        assert_eq!(failure.capability, ConcurrencyCapability::Unknown);
        assert_eq!(failure.path, vec!["next"]);
    }
}
