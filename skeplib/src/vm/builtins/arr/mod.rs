mod query;
mod transform;

use super::BuiltinRegistry;

pub(crate) use query::{
    builtin_arr_contains, builtin_arr_count, builtin_arr_first, builtin_arr_index_of,
    builtin_arr_is_empty, builtin_arr_last, builtin_arr_len,
};
pub(crate) use transform::builtin_arr_join;

#[allow(dead_code)]
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
