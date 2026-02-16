mod query;
mod reduce;
mod transform;

use super::BuiltinRegistry;

pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("arr", "len", query::builtin_arr_len);
    r.register("arr", "isEmpty", query::builtin_arr_is_empty);
    r.register("arr", "contains", query::builtin_arr_contains);
    r.register("arr", "indexOf", query::builtin_arr_index_of);
    r.register("arr", "sum", reduce::builtin_arr_sum);
    r.register("arr", "count", query::builtin_arr_count);
    r.register("arr", "first", query::builtin_arr_first);
    r.register("arr", "last", query::builtin_arr_last);
    r.register("arr", "reverse", transform::builtin_arr_reverse);
    r.register("arr", "join", transform::builtin_arr_join);
    r.register("arr", "slice", transform::builtin_arr_slice);
    r.register("arr", "min", query::builtin_arr_min);
    r.register("arr", "max", query::builtin_arr_max);
    r.register("arr", "sort", transform::builtin_arr_sort);
}
