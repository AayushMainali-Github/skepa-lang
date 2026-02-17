mod query;
mod transform;

use super::BuiltinRegistry;

pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("arr", "len", query::builtin_arr_len);
    r.register("arr", "isEmpty", query::builtin_arr_is_empty);
    r.register("arr", "contains", query::builtin_arr_contains);
    r.register("arr", "indexOf", query::builtin_arr_index_of);
    r.register("arr", "count", query::builtin_arr_count);
    r.register("arr", "first", query::builtin_arr_first);
    r.register("arr", "last", query::builtin_arr_last);
    r.register("arr", "join", transform::builtin_arr_join);
}
