pub mod arr;
pub mod bytes;
pub mod datetime;
pub mod ffi;
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

    fn spawn_function(
        &mut self,
        host: &mut dyn RtHost,
        function: RtFunctionRef,
        args: &[RtValue],
    ) -> RtResult<crate::RtHandle> {
        let value = self.call_function(function, args)?;
        host.task_store_completed(value)
    }
}

pub trait BuiltinContext {
    fn host(&mut self) -> &mut dyn RtHost;
    fn call_function(&mut self, function: RtFunctionRef, args: &[RtValue]) -> RtResult<RtValue>;
    fn spawn_function(
        &mut self,
        function: RtFunctionRef,
        args: &[RtValue],
    ) -> RtResult<crate::RtHandle>;
}

struct NoopRuntime;

impl BuiltinRuntime for NoopRuntime {
    fn call_function(&mut self, _function: RtFunctionRef, _args: &[RtValue]) -> RtResult<RtValue> {
        Err(RtError::unsupported_builtin("task.spawn"))
    }
}

struct SplitContext<'a> {
    host: &'a mut dyn RtHost,
    runtime: &'a mut dyn BuiltinRuntime,
}

impl BuiltinContext for SplitContext<'_> {
    fn host(&mut self) -> &mut dyn RtHost {
        self.host
    }

    fn call_function(&mut self, function: RtFunctionRef, args: &[RtValue]) -> RtResult<RtValue> {
        self.runtime.call_function(function, args)
    }

    fn spawn_function(
        &mut self,
        function: RtFunctionRef,
        args: &[RtValue],
    ) -> RtResult<crate::RtHandle> {
        self.runtime.spawn_function(self.host, function, args)
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
    runtime: &mut dyn BuiltinRuntime,
    package: &str,
    name: &str,
    args: &[RtValue],
) -> RtResult<RtValue> {
    let mut ctx = SplitContext { host, runtime };
    call_with_context(&mut ctx, package, name, args)
}

pub fn call_with_context(
    ctx: &mut dyn BuiltinContext,
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
            io::print(ctx.host(), value)?;
            Ok(RtValue::Unit)
        }
        ("io", "println", [value]) => {
            io::println(ctx.host(), value)?;
            Ok(RtValue::Unit)
        }
        ("io", "printInt", [value]) => {
            io::print(ctx.host(), &RtValue::Int(value.expect_int()?))?;
            Ok(RtValue::Unit)
        }
        ("io", "printFloat", [value]) => {
            io::print(ctx.host(), &RtValue::Float(value.expect_float()?))?;
            Ok(RtValue::Unit)
        }
        ("io", "printBool", [value]) => {
            io::print(ctx.host(), &RtValue::Bool(value.expect_bool()?))?;
            Ok(RtValue::Unit)
        }
        ("io", "printString", [value]) => {
            io::print(ctx.host(), &RtValue::String(value.expect_string()?))?;
            Ok(RtValue::Unit)
        }
        ("io", "format", args) => io::format(args),
        ("io", "printf", args) => io::printf(ctx.host(), args),
        ("io", "readLine", []) => io::read_line(ctx.host()),
        ("datetime", "nowUnix", []) => datetime::now_unix(ctx.host()),
        ("datetime", "nowMillis", []) => datetime::now_millis(ctx.host()),
        ("datetime", "fromUnix", [value]) => datetime::from_unix(ctx.host(), value.expect_int()?),
        ("datetime", "fromMillis", [value]) => {
            datetime::from_millis(ctx.host(), value.expect_int()?)
        }
        ("datetime", "parseUnix", [value]) => {
            datetime::parse_unix(ctx.host(), value.expect_string()?.as_str())
        }
        ("datetime", "year", [value]) => {
            datetime::component(ctx.host(), "year", value.expect_int()?)
        }
        ("datetime", "month", [value]) => {
            datetime::component(ctx.host(), "month", value.expect_int()?)
        }
        ("datetime", "day", [value]) => datetime::component(ctx.host(), "day", value.expect_int()?),
        ("datetime", "hour", [value]) => {
            datetime::component(ctx.host(), "hour", value.expect_int()?)
        }
        ("datetime", "minute", [value]) => {
            datetime::component(ctx.host(), "minute", value.expect_int()?)
        }
        ("datetime", "second", [value]) => {
            datetime::component(ctx.host(), "second", value.expect_int()?)
        }
        ("random", "seed", [value]) => random::seed(ctx.host(), value.expect_int()?),
        ("random", "int", [min, max]) => {
            random::int(ctx.host(), min.expect_int()?, max.expect_int()?)
        }
        ("random", "float", []) => random::float(ctx.host()),
        ("fs", "exists", [path]) => fs::exists(ctx.host(), path.expect_string()?.as_str()),
        ("fs", "readText", [path]) => fs::read_text(ctx.host(), path.expect_string()?.as_str()),
        ("fs", "writeText", [path, text]) => fs::write_text(
            ctx.host(),
            path.expect_string()?.as_str(),
            text.expect_string()?.as_str(),
        ),
        ("fs", "appendText", [path, text]) => fs::append_text(
            ctx.host(),
            path.expect_string()?.as_str(),
            text.expect_string()?.as_str(),
        ),
        ("fs", "mkdirAll", [path]) => fs::mkdir_all(ctx.host(), path.expect_string()?.as_str()),
        ("fs", "removeFile", [path]) => fs::remove_file(ctx.host(), path.expect_string()?.as_str()),
        ("fs", "removeDirAll", [path]) => {
            fs::remove_dir_all(ctx.host(), path.expect_string()?.as_str())
        }
        ("fs", "join", [left, right]) => fs::join(
            ctx.host(),
            left.expect_string()?.as_str(),
            right.expect_string()?.as_str(),
        ),
        ("ffi", "open", [path]) => ffi::open(ctx.host(), path.expect_string()?.as_str()),
        ("ffi", "bind", [library, symbol]) => ffi::bind(
            ctx.host(),
            library.expect_handle_kind(crate::RtHandleKind::Library)?,
            symbol.expect_string()?.as_str(),
        ),
        ("ffi", "closeLibrary", [library]) => ffi::close_library(
            ctx.host(),
            library.expect_handle_kind(crate::RtHandleKind::Library)?,
        ),
        ("ffi", "closeSymbol", [symbol]) => ffi::close_symbol(
            ctx.host(),
            symbol.expect_handle_kind(crate::RtHandleKind::Symbol)?,
        ),
        ("ffi", "call0Int", [symbol]) => ffi::call_0_int(
            ctx.host(),
            symbol.expect_handle_kind(crate::RtHandleKind::Symbol)?,
        ),
        ("ffi", "call1Int", [symbol, value]) => ffi::call_1_int(
            ctx.host(),
            symbol.expect_handle_kind(crate::RtHandleKind::Symbol)?,
            value.expect_int()?,
        ),
        ("ffi", "call1StringInt", [symbol, value]) => ffi::call_1_string_int(
            ctx.host(),
            symbol.expect_handle_kind(crate::RtHandleKind::Symbol)?,
            value.expect_string()?.as_str(),
        ),
        ("net", "__testSocket", []) => net::test_socket(ctx.host()),
        ("net", "listen", [address]) => net::listen(ctx.host(), address.expect_string()?.as_str()),
        ("net", "connect", [address]) => {
            net::connect(ctx.host(), address.expect_string()?.as_str())
        }
        ("net", "tlsConnect", [host_name, port]) => net::tls_connect(
            ctx.host(),
            host_name.expect_string()?.as_str(),
            port.expect_int()?,
        ),
        ("net", "resolve", [host_name]) => {
            net::resolve(ctx.host(), host_name.expect_string()?.as_str())
        }
        ("net", "parseUrl", [url]) => net::parse_url(ctx.host(), url.expect_string()?.as_str()),
        ("net", "fetch", [url, options]) => net::fetch(
            ctx.host(),
            url.expect_string()?.as_str(),
            &options.expect_map()?,
        ),
        ("net", "accept", [listener]) => net::accept(
            ctx.host(),
            listener.expect_handle_kind(crate::RtHandleKind::Listener)?,
        ),
        ("net", "read", [socket]) => net::read(
            ctx.host(),
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "write", [socket, data]) => net::write(
            ctx.host(),
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
            data.expect_string()?.as_str(),
        ),
        ("net", "readBytes", [socket]) => net::read_bytes(
            ctx.host(),
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "writeBytes", [socket, data]) => net::write_bytes(
            ctx.host(),
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
            &data.expect_bytes()?,
        ),
        ("net", "readN", [socket, count]) => net::read_n(
            ctx.host(),
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
            count.expect_int()?,
        ),
        ("net", "localAddr", [socket]) => net::local_addr(
            ctx.host(),
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "peerAddr", [socket]) => net::peer_addr(
            ctx.host(),
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "flush", [socket]) => net::flush(
            ctx.host(),
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "setReadTimeout", [socket, millis]) => net::set_read_timeout(
            ctx.host(),
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
            millis.expect_int()?,
        ),
        ("net", "setWriteTimeout", [socket, millis]) => net::set_write_timeout(
            ctx.host(),
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
            millis.expect_int()?,
        ),
        ("net", "close", [socket]) => net::close(
            ctx.host(),
            socket.expect_handle_kind(crate::RtHandleKind::Socket)?,
        ),
        ("net", "closeListener", [listener]) => net::close_listener(
            ctx.host(),
            listener.expect_handle_kind(crate::RtHandleKind::Listener)?,
        ),
        ("task", "__testTask", [value]) => task::test_task(ctx.host(), value),
        ("task", "__testChannel", []) => task::test_channel(ctx.host()),
        ("task", "channel", []) => task::channel(ctx.host()),
        ("task", "send", [channel, value]) => task::send(
            ctx.host(),
            channel.expect_handle_kind(crate::RtHandleKind::Channel)?,
            value,
        ),
        ("task", "recv", [channel]) => task::recv(
            ctx.host(),
            channel.expect_handle_kind(crate::RtHandleKind::Channel)?,
        ),
        ("task", "spawn", [function]) => task::spawn(ctx, function.expect_function()?),
        ("task", "join", [task]) => task::join(
            ctx.host(),
            task.expect_handle_kind(crate::RtHandleKind::Task)?,
        ),
        ("os", "platform", []) => os::platform(ctx.host()),
        ("os", "arch", []) => os::arch(ctx.host()),
        ("os", "arg", [value]) => os::arg(ctx.host(), value.expect_int()?),
        ("os", "envHas", [value]) => os::env_has(ctx.host(), value.expect_string()?.as_str()),
        ("os", "envGet", [value]) => os::env_get(ctx.host(), value.expect_string()?.as_str()),
        ("os", "envSet", [name, value]) => os::env_set(
            ctx.host(),
            name.expect_string()?.as_str(),
            value.expect_string()?.as_str(),
        ),
        ("os", "envRemove", [value]) => os::env_remove(ctx.host(), value.expect_string()?.as_str()),
        ("os", "sleep", [value]) => os::sleep(ctx.host(), value.expect_int()?),
        ("os", "exit", [value]) => os::exit(ctx.host(), value.expect_int()?),
        ("os", "exec", [program, args]) => os::exec(
            ctx.host(),
            program.expect_string()?.as_str(),
            &args.expect_string_vec()?,
        ),
        ("os", "execOut", [program, args]) => os::exec_out(
            ctx.host(),
            program.expect_string()?.as_str(),
            &args.expect_string_vec()?,
        ),
        _ => Err(RtError::new(
            RtErrorKind::UnsupportedBuiltin,
            format!("unsupported builtin `{package}.{name}`"),
        )),
    }
}
