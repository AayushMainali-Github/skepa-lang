pub mod arr;
pub mod io;
pub mod str;
pub mod vec;

use crate::{NoopHost, RtError, RtErrorKind, RtHost, RtResult, RtValue};

pub fn call(package: &str, name: &str, args: &[RtValue]) -> RtResult<RtValue> {
    let mut host = NoopHost;
    call_with_host(&mut host, package, name, args)
}

pub fn call_with_host(
    host: &mut dyn RtHost,
    package: &str,
    name: &str,
    args: &[RtValue],
) -> RtResult<RtValue> {
    match (package, name, args) {
        ("str", "len", [value]) => Ok(RtValue::Int(str::len(&value.expect_string()?))),
        ("str", "contains", [haystack, needle]) => Ok(RtValue::Bool(str::contains(
            &haystack.expect_string()?,
            &needle.expect_string()?,
        ))),
        ("str", "indexOf", [haystack, needle]) => Ok(RtValue::Int(str::index_of(
            &haystack.expect_string()?,
            &needle.expect_string()?,
        ))),
        ("str", "slice", [value, start, end]) => Ok(RtValue::String(str::slice(
            &value.expect_string()?,
            usize::try_from(start.expect_int()?)
                .map_err(|_| RtError::new(RtErrorKind::IndexOutOfBounds, "negative slice start"))?,
            usize::try_from(end.expect_int()?)
                .map_err(|_| RtError::new(RtErrorKind::IndexOutOfBounds, "negative slice end"))?,
        )?)),
        ("arr", "len", [array]) => Ok(RtValue::Int(arr::len(&array.expect_array()?))),
        ("arr", "isEmpty", [array]) => Ok(RtValue::Bool(arr::is_empty(&array.expect_array()?))),
        ("arr", "first", [array]) => arr::first(&array.expect_array()?),
        ("arr", "last", [array]) => arr::last(&array.expect_array()?),
        ("arr", "join", [array, sep]) => Ok(RtValue::String(arr::join(
            &array.expect_array()?,
            &sep.expect_string()?,
        )?)),
        ("vec", "new", []) => Ok(RtValue::Vec(vec::new())),
        ("vec", "len", [value]) => Ok(RtValue::Int(vec::len(&value.expect_vec()?))),
        ("vec", "push", [vec_value, value]) => {
            vec::push(&vec_value.expect_vec()?, value.clone());
            Ok(RtValue::Unit)
        }
        ("vec", "get", [vec_value, index]) => vec::get(
            &vec_value.expect_vec()?,
            usize::try_from(index.expect_int()?)
                .map_err(|_| RtError::new(RtErrorKind::IndexOutOfBounds, "negative vec index"))?,
        ),
        ("vec", "set", [vec_value, index, value]) => {
            vec::set(
                &vec_value.expect_vec()?,
                usize::try_from(index.expect_int()?).map_err(|_| {
                    RtError::new(RtErrorKind::IndexOutOfBounds, "negative vec index")
                })?,
                value.clone(),
            )?;
            Ok(RtValue::Unit)
        }
        ("vec", "delete", [vec_value, index]) => vec::delete(
            &vec_value.expect_vec()?,
            usize::try_from(index.expect_int()?)
                .map_err(|_| RtError::new(RtErrorKind::IndexOutOfBounds, "negative vec index"))?,
        ),
        ("io", "print", [value]) => {
            io::print(host, value)?;
            Ok(RtValue::Unit)
        }
        ("io", "println", [value]) => {
            io::println(host, value)?;
            Ok(RtValue::Unit)
        }
        ("io", "readLine", []) => io::read_line(host),
        _ => Err(RtError::new(
            RtErrorKind::UnsupportedBuiltin,
            format!("unsupported builtin `{package}.{name}`"),
        )),
    }
}
