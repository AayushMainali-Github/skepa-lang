pub mod arr;
pub mod bytes;
pub mod datetime;
pub mod fs;
pub mod io;
pub mod map;
pub mod net;
pub mod os;
pub mod random;
pub mod str;
pub mod task;
pub mod vec;

use crate::{NoopHost, RtError, RtErrorKind, RtFunctionRef, RtHost, RtResult, RtValue};

pub trait BuiltinRuntime {
    fn call_function(&mut self, function: RtFunctionRef, args: &[RtValue]) -> RtResult<RtValue>;
}

struct NoopRuntime;

impl BuiltinRuntime for NoopRuntime {
    fn call_function(&mut self, _function: RtFunctionRef, _args: &[RtValue]) -> RtResult<RtValue> {
        Err(RtError::unsupported_builtin("task.spawn"))
    }
}

pub fn call(package: &str, name: &str, args: &[RtValue]) -> RtResult<RtValue> {
    let mut host = NoopHost::default();
    let mut runtime = NoopRuntime;
    call_with_host_runtime(&mut host, &mut runtime, package, name, args)
}

pub fn call_with_host(
    host: &mut dyn RtHost,
    package: &str,
    name: &str,
    args: &[RtValue],
) -> RtResult<RtValue> {
    let mut runtime = NoopRuntime;
    call_with_host_runtime(host, &mut runtime, package, name, args)
}

pub fn call_with_host_runtime(
    host: &mut dyn RtHost,
    _runtime: &mut dyn BuiltinRuntime,
    package: &str,
    name: &str,
    args: &[RtValue],
) -> RtResult<RtValue> {
    match (package, name, args) {
        ("bytes", "fromString", [value]) => bytes::from_string(value.expect_string()?.as_str()),
        ("bytes", "toString", [value]) => bytes::to_string(&value.expect_bytes()?),
        ("bytes", "len", [value]) => Ok(bytes::len(&value.expect_bytes()?)),
        ("bytes", "get", [value, index]) => bytes::get(&value.expect_bytes()?, index.expect_int()?),
        ("bytes", "slice", [value, start, end]) => bytes::slice(
            &value.expect_bytes()?,
            start.expect_int()?,
            end.expect_int()?,
        ),
        ("bytes", "concat", [left, right]) => {
            Ok(bytes::concat(&left.expect_bytes()?, &right.expect_bytes()?))
        }
        ("bytes", "push", [value, byte]) => bytes::push(&value.expect_bytes()?, byte.expect_int()?),
        ("bytes", "append", [left, right]) => {
            Ok(bytes::append(&left.expect_bytes()?, &right.expect_bytes()?))
        }
        ("map", "new", []) => Ok(RtValue::Map(map::new())),
        ("map", "len", [value]) => Ok(map::len(&value.expect_map()?)),
        ("map", "has", [value, key]) => Ok(map::has(
            &value.expect_map()?,
            key.expect_string()?.as_str(),
        )),
        ("map", "get", [value, key]) => {
            map::get(&value.expect_map()?, key.expect_string()?.as_str())
        }
        ("map", "insert", [value, key, item]) => {
            map::insert(
                &value.expect_map()?,
                key.expect_string()?.as_str(),
                item.clone(),
            );
            Ok(RtValue::Unit)
        }
        ("map", "remove", [value, key]) => {
            map::remove(&value.expect_map()?, key.expect_string()?.as_str())
        }
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
        ("io", "printInt", [value]) => {
            io::print(host, &RtValue::Int(value.expect_int()?))?;
            Ok(RtValue::Unit)
        }
        ("io", "printFloat", [value]) => {
            io::print(host, &RtValue::Float(value.expect_float()?))?;
            Ok(RtValue::Unit)
        }
        ("io", "printBool", [value]) => {
            io::print(host, &RtValue::Bool(value.expect_bool()?))?;
            Ok(RtValue::Unit)
        }
        ("io", "printString", [value]) => {
            io::print(host, &RtValue::String(value.expect_string()?))?;
            Ok(RtValue::Unit)
        }
        ("io", "format", args) => io::format(args),
        ("io", "printf", args) => io::printf(host, args),
        ("io", "readLine", []) => io::read_line(host),
        ("datetime", "nowUnix", []) => datetime::now_unix(host),
        ("datetime", "nowMillis", []) => datetime::now_millis(host),
        ("datetime", "fromUnix", [value]) => datetime::from_unix(host, value.expect_int()?),
        ("datetime", "fromMillis", [value]) => datetime::from_millis(host, value.expect_int()?),
        ("datetime", "parseUnix", [value]) => {
            datetime::parse_unix(host, value.expect_string()?.as_str())
        }
        ("datetime", "year", [value]) => datetime::component(host, "year", value.expect_int()?),
        ("datetime", "month", [value]) => datetime::component(host, "month", value.expect_int()?),
        ("datetime", "day", [value]) => datetime::component(host, "day", value.expect_int()?),
        ("datetime", "hour", [value]) => datetime::component(host, "hour", value.expect_int()?),
        ("datetime", "minute", [value]) => datetime::component(host, "minute", value.expect_int()?),
        ("datetime", "second", [value]) => datetime::component(host, "second", value.expect_int()?),
        ("random", "seed", [value]) => random::seed(host, value.expect_int()?),
        ("random", "int", [min, max]) => random::int(host, min.expect_int()?, max.expect_int()?),
        ("random", "float", []) => random::float(host),
        ("fs", "exists", [path]) => fs::exists(host, path.expect_string()?.as_str()),
        ("fs", "readText", [path]) => fs::read_text(host, path.expect_string()?.as_str()),
        ("fs", "writeText", [path, text]) => fs::write_text(
            host,
            path.expect_string()?.as_str(),
            text.expect_string()?.as_str(),
        ),
        ("fs", "appendText", [path, text]) => fs::append_text(
            host,
            path.expect_string()?.as_str(),
            text.expect_string()?.as_str(),
        ),
        ("fs", "mkdirAll", [path]) => fs::mkdir_all(host, path.expect_string()?.as_str()),
        ("fs", "removeFile", [path]) => fs::remove_file(host, path.expect_string()?.as_str()),
        ("fs", "removeDirAll", [path]) => fs::remove_dir_all(host, path.expect_string()?.as_str()),
        ("fs", "join", [left, right]) => fs::join(
            host,
            left.expect_string()?.as_str(),
            right.expect_string()?.as_str(),
        ),
        ("net", "__testSocket", []) => net::test_socket(host),
        ("net", "listen", [address]) => net::listen(host, address.expect_string()?.as_str()),
        ("net", "connect", [address]) => net::connect(host, address.expect_string()?.as_str()),
        ("net", "tlsConnect", [host_name, port]) => net::tls_connect(
            host,
            host_name.expect_string()?.as_str(),
            port.expect_int()?,
        ),
        ("net", "accept", [listener]) => net::accept(
            host,
            listener.expect_handle_kind(crate::RtHandleKind::Listener)?,
        ),
        ("net", "read", [socket]) => net::read(
            host,
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "write", [socket, data]) => net::write(
            host,
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
            data.expect_string()?.as_str(),
        ),
        ("net", "readBytes", [socket]) => net::read_bytes(
            host,
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "writeBytes", [socket, data]) => net::write_bytes(
            host,
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
            &data.expect_bytes()?,
        ),
        ("net", "readN", [socket, count]) => net::read_n(
            host,
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
            count.expect_int()?,
        ),
        ("net", "localAddr", [socket]) => net::local_addr(
            host,
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "peerAddr", [socket]) => net::peer_addr(
            host,
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "flush", [socket]) => net::flush(
            host,
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "setReadTimeout", [socket, millis]) => net::set_read_timeout(
            host,
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
            millis.expect_int()?,
        ),
        ("net", "setWriteTimeout", [socket, millis]) => net::set_write_timeout(
            host,
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
            millis.expect_int()?,
        ),
        ("net", "close", [socket]) => net::close(
            host,
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "closeListener", [listener]) => net::close_listener(
            host,
            listener.expect_handle_kind(crate::RtHandleKind::Listener)?,
        ),
        ("task", "__testTask", [value]) => task::test_task(host, value),
        ("task", "__testChannel", []) => task::test_channel(host),
        ("task", "channel", []) => task::channel(host),
        ("task", "send", [channel, value]) => task::send(
            host,
            channel.expect_handle_kind(crate::RtHandleKind::Channel)?,
            value,
        ),
        ("task", "recv", [channel]) => task::recv(
            host,
            channel.expect_handle_kind(crate::RtHandleKind::Channel)?,
        ),
        ("task", "join", [task]) => {
            task::join(host, task.expect_handle_kind(crate::RtHandleKind::Task)?)
        }
        ("os", "platform", []) => os::platform(host),
        ("os", "arch", []) => os::arch(host),
        ("os", "arg", [value]) => os::arg(host, value.expect_int()?),
        ("os", "envHas", [value]) => os::env_has(host, value.expect_string()?.as_str()),
        ("os", "envGet", [value]) => os::env_get(host, value.expect_string()?.as_str()),
        ("os", "envSet", [name, value]) => os::env_set(
            host,
            name.expect_string()?.as_str(),
            value.expect_string()?.as_str(),
        ),
        ("os", "envRemove", [value]) => os::env_remove(host, value.expect_string()?.as_str()),
        ("os", "sleep", [value]) => os::sleep(host, value.expect_int()?),
        ("os", "exit", [value]) => os::exit(host, value.expect_int()?),
        ("os", "exec", [program, args]) => os::exec(
            host,
            program.expect_string()?.as_str(),
            &args.expect_string_vec()?,
        ),
        ("os", "execOut", [program, args]) => os::exec_out(
            host,
            program.expect_string()?.as_str(),
            &args.expect_string_vec()?,
        ),
        _ => Err(RtError::new(
            RtErrorKind::UnsupportedBuiltin,
            format!("unsupported builtin `{package}.{name}`"),
        )),
    }
}
