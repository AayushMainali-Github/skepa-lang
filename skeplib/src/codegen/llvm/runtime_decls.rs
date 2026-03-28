const RUNTIME_DECLS: &[(&str, &str)] = &[
    (
        "skp_rt_string_from_utf8",
        "declare ptr @skp_rt_string_from_utf8(ptr, i64)",
    ),
    ("skp_rt_string_eq", "declare i1 @skp_rt_string_eq(ptr, ptr)"),
    (
        "skp_rt_builtin_str_len",
        "declare i64 @skp_rt_builtin_str_len(ptr)",
    ),
    (
        "skp_rt_builtin_str_contains",
        "declare i1 @skp_rt_builtin_str_contains(ptr, ptr)",
    ),
    (
        "skp_rt_builtin_str_index_of",
        "declare i64 @skp_rt_builtin_str_index_of(ptr, ptr)",
    ),
    (
        "skp_rt_builtin_str_slice",
        "declare ptr @skp_rt_builtin_str_slice(ptr, i64, i64)",
    ),
    (
        "skp_rt_call_builtin",
        "declare ptr @skp_rt_call_builtin(ptr, ptr, i64, ptr)",
    ),
    (
        "skp_rt_call_function",
        "declare ptr @skp_rt_call_function(ptr, i64, ptr)",
    ),
    (
        "skp_rt_abort_if_error",
        "declare void @skp_rt_abort_if_error()",
    ),
    (
        "skp_rt_raise_division_by_zero",
        "declare void @skp_rt_raise_division_by_zero()",
    ),
    (
        "skp_rt_value_from_int",
        "declare ptr @skp_rt_value_from_int(i64)",
    ),
    (
        "skp_rt_value_from_bool",
        "declare ptr @skp_rt_value_from_bool(i1)",
    ),
    (
        "skp_rt_value_from_float",
        "declare ptr @skp_rt_value_from_float(double)",
    ),
    (
        "skp_rt_value_from_unit",
        "declare ptr @skp_rt_value_from_unit()",
    ),
    (
        "skp_rt_value_from_string",
        "declare ptr @skp_rt_value_from_string(ptr)",
    ),
    (
        "skp_rt_value_from_array",
        "declare ptr @skp_rt_value_from_array(ptr)",
    ),
    (
        "skp_rt_value_from_vec",
        "declare ptr @skp_rt_value_from_vec(ptr)",
    ),
    (
        "skp_rt_value_from_struct",
        "declare ptr @skp_rt_value_from_struct(ptr)",
    ),
    (
        "skp_rt_value_from_function",
        "declare ptr @skp_rt_value_from_function(ptr)",
    ),
    ("skp_rt_value_free", "declare void @skp_rt_value_free(ptr)"),
    (
        "skp_rt_value_to_int",
        "declare i64 @skp_rt_value_to_int(ptr)",
    ),
    (
        "skp_rt_value_to_bool",
        "declare i1 @skp_rt_value_to_bool(ptr)",
    ),
    (
        "skp_rt_value_to_float",
        "declare double @skp_rt_value_to_float(ptr)",
    ),
    (
        "skp_rt_value_to_string",
        "declare ptr @skp_rt_value_to_string(ptr)",
    ),
    (
        "skp_rt_value_to_array",
        "declare ptr @skp_rt_value_to_array(ptr)",
    ),
    (
        "skp_rt_value_to_vec",
        "declare ptr @skp_rt_value_to_vec(ptr)",
    ),
    (
        "skp_rt_value_to_struct",
        "declare ptr @skp_rt_value_to_struct(ptr)",
    ),
    (
        "skp_rt_value_to_function",
        "declare ptr @skp_rt_value_to_function(ptr)",
    ),
    ("skp_rt_array_new", "declare ptr @skp_rt_array_new(i64)"),
    (
        "skp_rt_array_repeat",
        "declare ptr @skp_rt_array_repeat(ptr, i64)",
    ),
    (
        "skp_rt_array_get",
        "declare ptr @skp_rt_array_get(ptr, i64)",
    ),
    (
        "skp_rt_array_set",
        "declare void @skp_rt_array_set(ptr, i64, ptr)",
    ),
    ("skp_rt_vec_new", "declare ptr @skp_rt_vec_new()"),
    ("skp_rt_vec_len", "declare i64 @skp_rt_vec_len(ptr)"),
    ("skp_rt_vec_push", "declare void @skp_rt_vec_push(ptr, ptr)"),
    ("skp_rt_vec_get", "declare ptr @skp_rt_vec_get(ptr, i64)"),
    (
        "skp_rt_vec_set",
        "declare void @skp_rt_vec_set(ptr, i64, ptr)",
    ),
    (
        "skp_rt_vec_delete",
        "declare ptr @skp_rt_vec_delete(ptr, i64)",
    ),
    (
        "skp_rt_struct_new",
        "declare ptr @skp_rt_struct_new(i64, i64)",
    ),
    (
        "skp_rt_struct_get",
        "declare ptr @skp_rt_struct_get(ptr, i64)",
    ),
    (
        "skp_rt_struct_set",
        "declare void @skp_rt_struct_set(ptr, i64, ptr)",
    ),
];

pub fn runtime_declarations() -> &'static [(&'static str, &'static str)] {
    RUNTIME_DECLS
}

#[cfg(test)]
mod tests {
    use super::runtime_declarations;
    use std::collections::HashSet;

    #[test]
    fn runtime_declarations_are_unique_and_cover_core_abi_surface() {
        let decls = runtime_declarations();
        let names = decls.iter().map(|(name, _)| *name).collect::<Vec<_>>();
        let unique = names.iter().copied().collect::<HashSet<_>>();
        assert_eq!(names.len(), unique.len(), "duplicate runtime decl names");
        assert!(names.contains(&"skp_rt_call_builtin"));
        assert!(names.contains(&"skp_rt_call_function"));
        assert!(names.contains(&"skp_rt_value_free"));
        assert!(names.contains(&"skp_rt_abort_if_error"));
        assert!(names.contains(&"skp_rt_raise_division_by_zero"));
    }
}
