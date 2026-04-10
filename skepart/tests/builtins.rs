mod common;

use common::RecordingHostBuilder;
use skepart::{builtins, RtBytes, RtErrorKind, RtFunctionRef, RtHost, RtResult, RtString, RtValue};

struct UnsupportedHost;

impl RtHost for UnsupportedHost {
    fn io_print(&mut self, _text: &str) -> RtResult<()> {
        Ok(())
    }
}

struct ImmediateRuntime;

impl builtins::BuiltinRuntime for ImmediateRuntime {
    fn call_function(&mut self, function: RtFunctionRef, _args: &[RtValue]) -> RtResult<RtValue> {
        match function.0 {
            7 => Ok(RtValue::Int(99)),
            _ => Err(skepart::RtError::new(
                skepart::RtErrorKind::InvalidArgument,
                format!("unknown runtime function id {}", function.0),
            )),
        }
    }
}

struct ThreadedRuntime;

impl builtins::BuiltinRuntime for ThreadedRuntime {
    fn call_function(&mut self, function: RtFunctionRef, _args: &[RtValue]) -> RtResult<RtValue> {
        match function.0 {
            7 => Ok(RtValue::Int(99)),
            _ => Err(skepart::RtError::new(
                skepart::RtErrorKind::InvalidArgument,
                format!("unknown runtime function id {}", function.0),
            )),
        }
    }

    fn spawn_function(
        &mut self,
        host: &mut dyn RtHost,
        function: RtFunctionRef,
        _args: &[RtValue],
    ) -> RtResult<skepart::RtHandle> {
        let task = std::thread::spawn(move || match function.0 {
            7 => Ok(RtValue::Int(123)),
            _ => Err(skepart::RtError::new(
                skepart::RtErrorKind::InvalidArgument,
                format!("unknown runtime function id {}", function.0),
            )),
        });
        host.task_store_running(task)
    }
}

fn string_vec(items: &[&str]) -> RtValue {
    let value = skepart::RtVec::new();
    for item in items {
        value.push(RtValue::String(RtString::from(*item)));
    }
    RtValue::Vec(value)
}

fn expect_ok_handle(
    value: RtValue,
    expected_kind: skepart::RtHandleKind,
    context: &str,
) -> skepart::RtHandle {
    let RtValue::Result(result) = value else {
        panic!("{context} should return a result");
    };
    let skepart::RtResultValue::Ok(value) = result else {
        panic!("{context} should return Ok(handle)");
    };
    let RtValue::Handle(handle) = *value else {
        panic!("{context} should return Ok(handle)");
    };
    assert_eq!(
        handle.kind, expected_kind,
        "{context} returned wrong handle kind"
    );
    handle
}

#[test]
fn builtins_dispatch_valid_core_families() {
    let mut host = RecordingHostBuilder::seeded().build();
    assert_eq!(
        builtins::call_with_host(&mut host, "datetime", "nowUnix", &[]).expect("datetime"),
        RtValue::Int(100)
    );
    assert_eq!(
        builtins::call(
            "bytes",
            "len",
            &[RtValue::Bytes(RtBytes::from("abc".as_bytes()))]
        )
        .expect("bytes.len"),
        RtValue::Int(3)
    );
    assert_eq!(
        builtins::call("str", "len", &[RtValue::String(RtString::from("abc"))]).expect("str.len"),
        RtValue::Int(3)
    );
    assert_eq!(
        builtins::call("vec", "new", &[])
            .expect("vec.new")
            .type_name(),
        "Vec"
    );
}

#[test]
fn builtins_cover_bytes_roundtrip_and_type_errors() {
    let bytes_value = builtins::call(
        "bytes",
        "fromString",
        &[RtValue::String(RtString::from("hello"))],
    )
    .expect("bytes.fromString");
    let RtValue::Bytes(raw) = bytes_value.clone() else {
        panic!("bytes.fromString should return Bytes");
    };
    assert_eq!(raw.as_slice(), b"hello");
    assert_eq!(
        builtins::call("bytes", "len", std::slice::from_ref(&bytes_value)).expect("bytes.len"),
        RtValue::Int(5)
    );
    assert_eq!(
        builtins::call("bytes", "toString", &[bytes_value]).expect("bytes.toString"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::String(RtString::from(
            "hello"
        ))))
    );
    assert_eq!(
        builtins::call(
            "bytes",
            "get",
            &[
                RtValue::Bytes(RtBytes::from("hello".as_bytes())),
                RtValue::Int(1)
            ],
        )
        .expect("bytes.get"),
        RtValue::Option(skepart::RtOption::some(RtValue::Int(101)))
    );
    assert_eq!(
        builtins::call(
            "bytes",
            "slice",
            &[
                RtValue::Bytes(RtBytes::from("hello".as_bytes())),
                RtValue::Int(1),
                RtValue::Int(4),
            ],
        )
        .expect("bytes.slice"),
        RtValue::Bytes(RtBytes::from("ell".as_bytes()))
    );
    assert_eq!(
        builtins::call(
            "bytes",
            "concat",
            &[
                RtValue::Bytes(RtBytes::from("hel".as_bytes())),
                RtValue::Bytes(RtBytes::from("lo".as_bytes())),
            ],
        )
        .expect("bytes.concat"),
        RtValue::Bytes(RtBytes::from("hello".as_bytes()))
    );
    assert_eq!(
        builtins::call(
            "bytes",
            "push",
            &[
                RtValue::Bytes(RtBytes::from("hell".as_bytes())),
                RtValue::Int(111)
            ],
        )
        .expect("bytes.push"),
        RtValue::Bytes(RtBytes::from("hello".as_bytes()))
    );
    assert_eq!(
        builtins::call(
            "bytes",
            "append",
            &[
                RtValue::Bytes(RtBytes::from("hel".as_bytes())),
                RtValue::Bytes(RtBytes::from("lo".as_bytes())),
            ],
        )
        .expect("bytes.append"),
        RtValue::Bytes(RtBytes::from("hello".as_bytes()))
    );
    assert_eq!(
        builtins::call("bytes", "fromString", &[RtValue::Int(1)])
            .expect_err("bytes.fromString type mismatch")
            .kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        builtins::call(
            "bytes",
            "toString",
            &[RtValue::String(RtString::from("abc"))]
        )
        .expect_err("bytes.toString type mismatch")
        .kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        builtins::call(
            "bytes",
            "toString",
            &[RtValue::Bytes(RtBytes::from(vec![0xFF_u8, 0xFE]))]
        )
        .expect("bytes.toString invalid utf8 should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("bytes.toString expected valid UTF-8 data")
        )))
    );
    assert_eq!(
        builtins::call(
            "bytes",
            "get",
            &[
                RtValue::Bytes(RtBytes::from("a".as_bytes())),
                RtValue::Int(-1)
            ],
        )
        .expect("bytes.get negative index should return none"),
        RtValue::Option(skepart::RtOption::none())
    );
    assert_eq!(
        builtins::call(
            "bytes",
            "slice",
            &[
                RtValue::Bytes(RtBytes::from("abc".as_bytes())),
                RtValue::Int(2),
                RtValue::Int(1),
            ],
        )
        .expect_err("bytes.slice reversed range")
        .kind,
        RtErrorKind::IndexOutOfBounds
    );
    assert_eq!(
        builtins::call(
            "bytes",
            "push",
            &[
                RtValue::Bytes(RtBytes::from("a".as_bytes())),
                RtValue::Int(256)
            ],
        )
        .expect_err("bytes.push invalid byte")
        .kind,
        RtErrorKind::InvalidArgument
    );
}

#[test]
fn builtins_cover_map_roundtrip_and_errors() {
    let value = builtins::call("map", "new", &[]).expect("map.new");
    let RtValue::Map(raw) = value.clone() else {
        panic!("map.new should return Map");
    };
    assert_eq!(raw.len(), 0);
    assert_eq!(
        builtins::call("map", "len", std::slice::from_ref(&value)).expect("map.len"),
        RtValue::Int(0)
    );
    assert_eq!(
        builtins::call(
            "map",
            "has",
            &[value.clone(), RtValue::String(RtString::from("name"))],
        )
        .expect("map.has"),
        RtValue::Bool(false)
    );
    assert_eq!(
        builtins::call(
            "map",
            "insert",
            &[
                value.clone(),
                RtValue::String(RtString::from("name")),
                RtValue::Int(7),
            ],
        )
        .expect("map.insert"),
        RtValue::Unit
    );
    assert_eq!(
        builtins::call("map", "len", std::slice::from_ref(&value)).expect("map.len"),
        RtValue::Int(1)
    );
    assert_eq!(
        builtins::call(
            "map",
            "has",
            &[value.clone(), RtValue::String(RtString::from("name"))],
        )
        .expect("map.has"),
        RtValue::Bool(true)
    );
    assert_eq!(
        builtins::call(
            "map",
            "get",
            &[value.clone(), RtValue::String(RtString::from("name"))],
        )
        .expect("map.get"),
        RtValue::Option(skepart::RtOption::some(RtValue::Int(7)))
    );
    assert_eq!(
        builtins::call(
            "map",
            "remove",
            &[value.clone(), RtValue::String(RtString::from("name"))],
        )
        .expect("map.remove"),
        RtValue::Option(skepart::RtOption::some(RtValue::Int(7)))
    );
    assert_eq!(
        builtins::call("map", "len", std::slice::from_ref(&value)).expect("map.len"),
        RtValue::Int(0)
    );
    assert_eq!(
        builtins::call("map", "new", &[RtValue::Int(1)])
            .expect_err("map.new arity")
            .kind,
        RtErrorKind::UnsupportedBuiltin
    );
    assert_eq!(
        builtins::call("map", "len", &[RtValue::Int(1)])
            .expect_err("map.len type mismatch")
            .kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        builtins::call(
            "map",
            "get",
            &[value.clone(), RtValue::String(RtString::from("missing"))],
        )
        .expect("map.get missing key"),
        RtValue::Option(skepart::RtOption::none())
    );
    assert_eq!(
        builtins::call(
            "map",
            "remove",
            &[value, RtValue::String(RtString::from("missing"))],
        )
        .expect("map.remove missing key"),
        RtValue::Option(skepart::RtOption::none())
    );
}

#[test]
fn builtins_cover_net_bytes_roundtrip_and_errors() {
    let mut host = RecordingHostBuilder::seeded()
        .net_read_bytes_value(vec![0xDE, 0xAD, 0xBE, 0xEF])
        .build();
    let socket =
        builtins::call_with_host(&mut host, "net", "__testSocket", &[]).expect("allocate socket");

    assert_eq!(
        builtins::call_with_host(&mut host, "net", "readBytes", std::slice::from_ref(&socket))
            .expect("net.readBytes"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Bytes(RtBytes::from(
            vec![0xDE, 0xAD, 0xBE, 0xEF]
        ))))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "writeBytes",
            &[
                socket.clone(),
                RtValue::Bytes(RtBytes::from(vec![1_u8, 2, 3]))
            ],
        )
        .expect("net.writeBytes"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Unit))
    );
    assert!(
        host.output.contains("[netreadbytes 0]") && host.output.contains("[netwritebytes 0 len=3]"),
        "unexpected host output: {}",
        host.output
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "writeBytes",
            &[socket, RtValue::String(RtString::from("bad"))],
        )
        .expect_err("net.writeBytes type mismatch")
        .kind,
        RtErrorKind::TypeMismatch
    );
}

#[test]
fn builtins_cover_net_address_queries() {
    let mut host = RecordingHostBuilder::seeded()
        .net_local_addr_value("127.0.0.1:7000")
        .net_peer_addr_value("127.0.0.1:8000")
        .build();
    let socket =
        builtins::call_with_host(&mut host, "net", "__testSocket", &[]).expect("allocate socket");

    assert_eq!(
        builtins::call_with_host(&mut host, "net", "localAddr", std::slice::from_ref(&socket))
            .expect("net.localAddr"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::String(RtString::from(
            "127.0.0.1:7000"
        ))))
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "peerAddr", std::slice::from_ref(&socket))
            .expect("net.peerAddr"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::String(RtString::from(
            "127.0.0.1:8000"
        ))))
    );
    assert!(
        host.output.contains("[netlocaladdr 0]") && host.output.contains("[netpeeraddr 0]"),
        "unexpected host output: {}",
        host.output
    );
}

#[test]
fn builtins_cover_dummy_task_handles() {
    let mut host = RecordingHostBuilder::seeded().build();

    let task = builtins::call_with_host(&mut host, "task", "__testTask", &[RtValue::Int(7)])
        .expect("task.__testTask");
    let channel = builtins::call_with_host(&mut host, "task", "__testChannel", &[])
        .expect("task.__testChannel");

    let RtValue::Handle(task_handle) = task.clone() else {
        panic!("task.__testTask should return a handle");
    };
    let RtValue::Handle(channel_handle) = channel.clone() else {
        panic!("task.__testChannel should return a handle");
    };

    assert_eq!(task_handle.kind, skepart::RtHandleKind::Task);
    assert_eq!(channel_handle.kind, skepart::RtHandleKind::Channel);
    assert_eq!(
        builtins::call_with_host(&mut host, "task", "join", &[task]).expect("task.join"),
        RtValue::Int(7)
    );
}

#[test]
fn builtins_cover_typed_task_channel_roundtrip() {
    let mut host = RecordingHostBuilder::seeded().build();
    let channel =
        builtins::call_with_host(&mut host, "task", "channel", &[]).expect("task.channel");

    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "task",
            "send",
            &[channel.clone(), RtValue::Int(41)],
        )
        .expect("task.send"),
        RtValue::Unit
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "task", "recv", std::slice::from_ref(&channel))
            .expect("task.recv"),
        RtValue::Int(41)
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "task", "recv", &[channel])
            .expect_err("empty channel")
            .kind,
        RtErrorKind::InvalidArgument
    );
}

#[test]
fn builtins_cover_spawn_and_join_roundtrip() {
    let mut host = RecordingHostBuilder::seeded().build();
    let mut runtime = ImmediateRuntime;
    let task = builtins::call_with_host_runtime(
        &mut host,
        &mut runtime,
        "task",
        "spawn",
        &[RtValue::Function(RtFunctionRef(7))],
    )
    .expect("task.spawn");

    assert_eq!(
        builtins::call_with_host(&mut host, "task", "join", &[task]).expect("task.join"),
        RtValue::Int(99)
    );
}

#[test]
fn builtins_spawn_uses_runtime_spawn_hook_when_available() {
    let mut host = skepart::NoopHost::default();
    let mut runtime = ThreadedRuntime;
    let task = builtins::call_with_host_runtime(
        &mut host,
        &mut runtime,
        "task",
        "spawn",
        &[RtValue::Function(RtFunctionRef(7))],
    )
    .expect("task.spawn");

    assert_eq!(
        builtins::call_with_host(&mut host, "task", "join", &[task]).expect("task.join"),
        RtValue::Int(123)
    );
}

#[test]
fn builtins_cover_net_flush() {
    let mut host = RecordingHostBuilder::seeded().build();
    let socket =
        builtins::call_with_host(&mut host, "net", "__testSocket", &[]).expect("allocate socket");

    assert_eq!(
        builtins::call_with_host(&mut host, "net", "flush", std::slice::from_ref(&socket))
            .expect("net.flush"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Unit))
    );
    assert!(
        host.output.contains("[netflush 0]"),
        "unexpected host output: {}",
        host.output
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "flush", &[RtValue::Int(1)])
            .expect_err("net.flush type mismatch")
            .kind,
        RtErrorKind::TypeMismatch
    );
}

#[test]
fn builtins_cover_net_timeout_setters() {
    let mut host = RecordingHostBuilder::seeded().build();
    let socket =
        builtins::call_with_host(&mut host, "net", "__testSocket", &[]).expect("allocate socket");

    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "setReadTimeout",
            &[socket.clone(), RtValue::Int(25)],
        )
        .expect("net.setReadTimeout"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Unit))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "setWriteTimeout",
            &[socket.clone(), RtValue::Int(0)],
        )
        .expect("net.setWriteTimeout"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Unit))
    );
    assert!(
        host.output.contains("[netsetreadtimeout 0=25]")
            && host.output.contains("[netsetwritetimeout 0=0]"),
        "unexpected host output: {}",
        host.output
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "setReadTimeout",
            &[socket.clone(), RtValue::Int(-1)],
        )
        .expect("negative read timeout should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("InvalidArgument: net.setReadTimeout millis must be non-negative")
        )))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "setWriteTimeout",
            &[RtValue::Int(1), RtValue::Int(10)],
        )
        .expect_err("setWriteTimeout type mismatch")
        .kind,
        RtErrorKind::TypeMismatch
    );
}

#[test]
fn builtins_cover_net_read_n() {
    let mut host = RecordingHostBuilder::seeded()
        .net_read_n_value(vec![9_u8, 8, 7, 6])
        .build();
    let socket =
        builtins::call_with_host(&mut host, "net", "__testSocket", &[]).expect("allocate socket");

    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "readN",
            &[socket.clone(), RtValue::Int(3)],
        )
        .expect("net.readN"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Bytes(RtBytes::from(
            vec![9_u8, 8, 7]
        ))))
    );
    assert!(
        host.output.contains("[netreadn 0 count=3]"),
        "unexpected host output: {}",
        host.output
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "readN", &[socket, RtValue::Int(-1)])
            .expect("negative readN count should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("InvalidArgument: net.readN count must be non-negative")
        )))
    );
}

#[test]
fn builtins_report_unknown_family_arity_and_type_errors() {
    let mut host = UnsupportedHost;
    assert_eq!(
        builtins::call("missing", "fn", &[])
            .expect_err("bad family")
            .kind,
        RtErrorKind::UnsupportedBuiltin
    );
    assert_eq!(
        builtins::call("str", "len", &[])
            .expect_err("bad arity")
            .kind,
        RtErrorKind::UnsupportedBuiltin
    );
    assert_eq!(
        builtins::call("str", "len", &[RtValue::Int(1)])
            .expect_err("bad type")
            .kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "datetime", "fromUnix", &[RtValue::Int(2)])
            .expect_err("unsupported host")
            .kind,
        RtErrorKind::UnsupportedBuiltin
    );
}

#[test]
fn builtins_map_host_backed_results_consistently() {
    let mut host = RecordingHostBuilder::seeded().build();
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "fs",
            "join",
            &[
                RtValue::String(RtString::from("tmp")),
                RtValue::String(RtString::from("x.txt")),
            ],
        )
        .expect("fs.join"),
        RtValue::String(RtString::from("tmp/x.txt"))
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "os", "platform", &[]).expect("os.platform"),
        RtValue::String(RtString::from("test-os"))
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "random", "float", &[]).expect("random.float"),
        RtValue::Float(0.25)
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "__testSocket", &[]).expect("net socket"),
        RtValue::Handle(skepart::RtHandle {
            id: 0,
            kind: skepart::RtHandleKind::Socket,
        })
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "listen",
            &[RtValue::String(RtString::from("127.0.0.1:0"))],
        )
        .expect("net listener"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Handle(
            skepart::RtHandle {
                id: 1,
                kind: skepart::RtHandleKind::Listener,
            }
        )))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "read",
            &[RtValue::Handle(skepart::RtHandle {
                id: 0,
                kind: skepart::RtHandleKind::Socket,
            })],
        )
        .expect("net read"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::String(RtString::from(
            "net-read"
        ))))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "write",
            &[
                RtValue::Handle(skepart::RtHandle {
                    id: 0,
                    kind: skepart::RtHandleKind::Socket,
                }),
                RtValue::String(RtString::from("ping")),
            ],
        )
        .expect("net write"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Unit))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "close",
            &[RtValue::Handle(skepart::RtHandle {
                id: 0,
                kind: skepart::RtHandleKind::Socket,
            })],
        )
        .expect("net close"),
        RtValue::Unit
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "closeListener",
            &[RtValue::Handle(skepart::RtHandle {
                id: 1,
                kind: skepart::RtHandleKind::Listener,
            })],
        )
        .expect("net closeListener"),
        RtValue::Unit
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "close",
            &[RtValue::Handle(skepart::RtHandle {
                id: 0,
                kind: skepart::RtHandleKind::Socket,
            })],
        )
        .expect_err("double close should fail")
        .kind,
        RtErrorKind::InvalidArgument
    );
    assert!(
        host.output.contains("[netread 0]") && host.output.contains("[netwrite 0=ping]"),
        "unexpected host output: {}",
        host.output
    );
}

#[test]
fn builtins_preserve_net_handle_kinds_across_connect_listen_and_accept() {
    let mut host = RecordingHostBuilder::seeded().build();

    assert_eq!(
        expect_ok_handle(
            builtins::call_with_host(
                &mut host,
                "net",
                "connect",
                &[RtValue::String(RtString::from("127.0.0.1:8080"))],
            )
            .expect("connect should return socket"),
            skepart::RtHandleKind::Socket,
            "net.connect",
        ),
        skepart::RtHandle {
            id: 0,
            kind: skepart::RtHandleKind::Socket,
        }
    );

    assert_eq!(
        expect_ok_handle(
            builtins::call_with_host(
                &mut host,
                "net",
                "tlsConnect",
                &[
                    RtValue::String(RtString::from("example.com")),
                    RtValue::Int(443)
                ],
            )
            .expect("tlsConnect should return socket"),
            skepart::RtHandleKind::Socket,
            "net.tlsConnect",
        ),
        skepart::RtHandle {
            id: 1,
            kind: skepart::RtHandleKind::Socket,
        }
    );

    let listener = expect_ok_handle(
        builtins::call_with_host(
            &mut host,
            "net",
            "listen",
            &[RtValue::String(RtString::from("127.0.0.1:0"))],
        )
        .expect("listen should return listener"),
        skepart::RtHandleKind::Listener,
        "net.listen",
    );
    assert_eq!(
        listener,
        skepart::RtHandle {
            id: 2,
            kind: skepart::RtHandleKind::Listener,
        }
    );

    assert_eq!(
        expect_ok_handle(
            builtins::call_with_host(&mut host, "net", "accept", &[RtValue::Handle(listener)])
                .expect("accept should return socket"),
            skepart::RtHandleKind::Socket,
            "net.accept",
        ),
        skepart::RtHandle {
            id: 3,
            kind: skepart::RtHandleKind::Socket,
        }
    );
}

#[test]
fn builtins_cover_net_resolve_and_errors() {
    let mut host = RecordingHostBuilder::seeded()
        .net_resolve_value("203.0.113.7")
        .build();

    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "resolve",
            &[RtValue::String(RtString::from("example.com"))],
        )
        .expect("resolve should return result"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::String(RtString::from(
            "203.0.113.7",
        ))))
    );
    assert!(
        host.output.contains("[netresolve example.com]"),
        "unexpected host output: {}",
        host.output
    );

    let mut failing_host = RecordingHostBuilder::seeded()
        .net_resolve_error("dns failed")
        .build();
    let failed = builtins::call_with_host(
        &mut failing_host,
        "net",
        "resolve",
        &[RtValue::String(RtString::from("bad.host"))],
    )
    .expect("resolve failure should return Err(string)");
    let RtValue::Result(failed) = failed else {
        panic!("net.resolve should return a result");
    };
    let skepart::RtResultValue::Err(failed) = failed else {
        panic!("net.resolve should return Err(string) on failure");
    };
    let RtValue::String(failed) = *failed else {
        panic!("net.resolve should return Err(string)");
    };
    assert!(failed.as_str().contains("dns failed"));
}

#[test]
fn builtins_cover_net_parse_url_and_errors() {
    let mut host = RecordingHostBuilder::seeded()
        .net_parse_url_parts("https", "example.com", "8443", "/api", "x=1", "frag")
        .build();

    let parsed = builtins::call_with_host(
        &mut host,
        "net",
        "parseUrl",
        &[RtValue::String(RtString::from(
            "https://example.com:8443/api?x=1#frag",
        ))],
    )
    .expect("parseUrl should return result");

    let RtValue::Result(parsed) = parsed else {
        panic!("net.parseUrl should return a result");
    };
    let skepart::RtResultValue::Ok(parsed) = parsed else {
        panic!("net.parseUrl should return Ok(map) on success");
    };
    let RtValue::Map(parsed) = *parsed else {
        panic!("net.parseUrl should return Ok(map)");
    };
    assert_eq!(
        parsed.get("scheme").expect("scheme"),
        RtValue::String(RtString::from("https"))
    );
    assert_eq!(
        parsed.get("host").expect("host"),
        RtValue::String(RtString::from("example.com"))
    );
    assert_eq!(
        parsed.get("port").expect("port"),
        RtValue::String(RtString::from("8443"))
    );
    assert!(
        host.output
            .contains("[netparseurl https://example.com:8443/api?x=1#frag]"),
        "unexpected host output: {}",
        host.output
    );

    let mut failing_host = RecordingHostBuilder::seeded()
        .net_parse_url_error("bad url")
        .build();
    let failed = builtins::call_with_host(
        &mut failing_host,
        "net",
        "parseUrl",
        &[RtValue::String(RtString::from("bad"))],
    )
    .expect("parseUrl failure should return Err(string)");
    let RtValue::Result(failed) = failed else {
        panic!("net.parseUrl should return a result");
    };
    let skepart::RtResultValue::Err(failed) = failed else {
        panic!("net.parseUrl should return Err(string) on failure");
    };
    let RtValue::String(failed) = *failed else {
        panic!("net.parseUrl should return Err(string)");
    };
    assert!(failed.as_str().contains("bad url"));
}

#[test]
fn builtins_cover_ffi_open_bind_and_errors() {
    let mut host = RecordingHostBuilder::seeded().build();
    let library = builtins::call_with_host(
        &mut host,
        "ffi",
        "open",
        &[RtValue::String(RtString::from("test-lib"))],
    )
    .expect("ffi.open should return result");
    let RtValue::Result(library_result) = library.clone() else {
        panic!("ffi.open should return a result");
    };
    let skepart::RtResultValue::Ok(library) = library_result else {
        panic!("ffi.open should return Ok(handle)");
    };
    let RtValue::Handle(library) = *library else {
        panic!("ffi.open should return Ok(handle)");
    };
    assert_eq!(library.kind, skepart::RtHandleKind::Library);

    let symbol = builtins::call_with_host(
        &mut host,
        "ffi",
        "bind",
        &[
            RtValue::Handle(library),
            RtValue::String(RtString::from("puts")),
        ],
    )
    .expect("ffi.bind should return result");
    let RtValue::Result(symbol_result) = symbol.clone() else {
        panic!("ffi.bind should return a result");
    };
    let skepart::RtResultValue::Ok(symbol) = symbol_result else {
        panic!("ffi.bind should return Ok(handle)");
    };
    let RtValue::Handle(symbol) = *symbol else {
        panic!("ffi.bind should return Ok(handle)");
    };
    assert_eq!(symbol.kind, skepart::RtHandleKind::Symbol);
    assert!(
        host.output.contains("[ffiopen test-lib]") && host.output.contains("[ffibind 0:puts]"),
        "unexpected host output: {}",
        host.output
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "ffi", "closeSymbol", &[RtValue::Handle(symbol)])
            .expect("close symbol"),
        RtValue::Unit
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "closeLibrary",
            &[RtValue::Handle(library)]
        )
        .expect("close library"),
        RtValue::Unit
    );

    let mut failing_open = RecordingHostBuilder::seeded()
        .ffi_open_error("open failed")
        .build();
    assert_eq!(
        builtins::call_with_host(
            &mut failing_open,
            "ffi",
            "open",
            &[RtValue::String(RtString::from("bad-lib"))],
        )
        .expect("ffi.open failure should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("open failed")
        )))
    );

    let mut failing_bind = RecordingHostBuilder::seeded()
        .ffi_bind_error("bind failed")
        .build();
    let library = builtins::call_with_host(
        &mut failing_bind,
        "ffi",
        "open",
        &[RtValue::String(RtString::from("test-lib"))],
    )
    .expect("ffi.open");
    let RtValue::Result(library) = library else {
        panic!("ffi.open should return a result");
    };
    let skepart::RtResultValue::Ok(library) = library else {
        panic!("ffi.open should return Ok(handle)");
    };
    assert_eq!(
        builtins::call_with_host(
            &mut failing_bind,
            "ffi",
            "bind",
            &[*library, RtValue::String(RtString::from("puts"))],
        )
        .expect("ffi.bind failure should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("bind failed")
        )))
    );
}

#[test]
fn builtins_cover_ffi_integer_calls_and_errors() {
    let mut host = RecordingHostBuilder::seeded()
        .ffi_call0_int_value(77)
        .ffi_call1_int_offset(5)
        .ffi_call1_string_offset(2)
        .ffi_call1_bytes_offset(3)
        .build();
    let library = builtins::call_with_host(
        &mut host,
        "ffi",
        "open",
        &[RtValue::String(RtString::from("test-lib"))],
    )
    .expect("ffi.open");
    let RtValue::Result(library) = library else {
        panic!("ffi.open should return a result");
    };
    let skepart::RtResultValue::Ok(library) = library else {
        panic!("ffi.open should return Ok(handle)");
    };
    let symbol = builtins::call_with_host(
        &mut host,
        "ffi",
        "bind",
        &[*library, RtValue::String(RtString::from("plus"))],
    )
    .expect("ffi.bind");
    let RtValue::Result(symbol) = symbol else {
        panic!("ffi.bind should return a result");
    };
    let skepart::RtResultValue::Ok(symbol) = symbol else {
        panic!("ffi.bind should return Ok(handle)");
    };
    let symbol = *symbol;

    assert_eq!(
        builtins::call_with_host(&mut host, "ffi", "call0Int", std::slice::from_ref(&symbol))
            .expect("ffi.call0Int"),
        RtValue::Int(77)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call",
            &[symbol.clone(), RtValue::String(RtString::from("->i64")),],
        )
        .expect("ffi.call generic zero-int"),
        RtValue::Int(77)
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "ffi", "call0Void", std::slice::from_ref(&symbol))
            .expect("ffi.call0Void"),
        RtValue::Unit
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "ffi", "call0Bool", std::slice::from_ref(&symbol))
            .expect("ffi.call0Bool"),
        RtValue::Bool(true)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call1Int",
            &[symbol.clone(), RtValue::Int(9)],
        )
        .expect("ffi.call1Int"),
        RtValue::Int(14)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call1IntBool",
            &[symbol.clone(), RtValue::Int(9)],
        )
        .expect("ffi.call1IntBool"),
        RtValue::Bool(true)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call1IntVoid",
            &[symbol.clone(), RtValue::Int(4)],
        )
        .expect("ffi.call1IntVoid"),
        RtValue::Unit
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call1StringInt",
            &[symbol.clone(), RtValue::String(RtString::from("hello"))],
        )
        .expect("ffi.call1StringInt"),
        RtValue::Int(7)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call1StringVoid",
            &[symbol.clone(), RtValue::String(RtString::from("trace"))],
        )
        .expect("ffi.call1StringVoid"),
        RtValue::Unit
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call2StringInt",
            &[
                symbol.clone(),
                RtValue::String(RtString::from("alpha")),
                RtValue::String(RtString::from("beta")),
            ],
        )
        .expect("ffi.call2StringInt"),
        RtValue::Int(1)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call2StringIntInt",
            &[
                symbol.clone(),
                RtValue::String(RtString::from("abc")),
                RtValue::Int(2),
            ],
        )
        .expect("ffi.call2StringIntInt"),
        RtValue::Int(5)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call2IntInt",
            &[symbol.clone(), RtValue::Int(2), RtValue::Int(5)],
        )
        .expect("ffi.call2IntInt"),
        RtValue::Int(7)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call1BytesInt",
            &[
                symbol.clone(),
                RtValue::Bytes(RtBytes::from(b"abc".to_vec()))
            ],
        )
        .expect("ffi.call1BytesInt"),
        RtValue::Int(6)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call2BytesIntInt",
            &[
                symbol.clone(),
                RtValue::Bytes(RtBytes::from(b"abc".to_vec())),
                RtValue::Int(4),
            ],
        )
        .expect("ffi.call2BytesIntInt"),
        RtValue::Int(7)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "ffi",
            "call",
            &[
                symbol.clone(),
                RtValue::String(RtString::from("bytes,usize->usize")),
                RtValue::Bytes(RtBytes::from(b"abc".to_vec())),
                RtValue::Int(4),
            ],
        )
        .expect("ffi.call generic bytes-int"),
        RtValue::Int(7)
    );
    assert!(
        host.output.contains("[fficall0int 1]")
            && host.output.contains("[fficall0void 1]")
            && host.output.contains("[fficall0bool 1]")
            && host.output.contains("[fficall1int 1=9]")
            && host.output.contains("[fficall1intbool 1=9]")
            && host.output.contains("[fficall1intvoid 1=4]")
            && host.output.contains("[fficall1stringint 1=hello]")
            && host.output.contains("[fficall1stringvoid 1=trace]")
            && host.output.contains("[fficall2stringint 1=alpha|beta]")
            && host.output.contains("[fficall2stringintint 1=abc|2]")
            && host.output.contains("[fficall2intint 1=2|5]"),
        "unexpected host output: {}",
        host.output
    );
    assert!(
        host.output.contains("[fficall1bytesint 1 len=3]")
            && host.output.contains("[fficall2bytesintint 1 len=3|4]"),
        "unexpected host output: {}",
        host.output
    );

    let mut failing_host = RecordingHostBuilder::seeded()
        .ffi_call_error("call failed")
        .build();
    let library = builtins::call_with_host(
        &mut failing_host,
        "ffi",
        "open",
        &[RtValue::String(RtString::from("test-lib"))],
    )
    .expect("ffi.open");
    let RtValue::Result(library) = library else {
        panic!("ffi.open should return a result");
    };
    let skepart::RtResultValue::Ok(library) = library else {
        panic!("ffi.open should return Ok(handle)");
    };
    let symbol = builtins::call_with_host(
        &mut failing_host,
        "ffi",
        "bind",
        &[*library, RtValue::String(RtString::from("plus"))],
    )
    .expect("ffi.bind");
    let RtValue::Result(symbol) = symbol else {
        panic!("ffi.bind should return a result");
    };
    let skepart::RtResultValue::Ok(symbol) = symbol else {
        panic!("ffi.bind should return Ok(handle)");
    };
    assert_eq!(
        builtins::call_with_host(
            &mut failing_host,
            "ffi",
            "call1BytesInt",
            &[*symbol, RtValue::Bytes(RtBytes::from(b"boom".to_vec()))],
        )
        .expect_err("ffi.call1BytesInt failure should surface")
        .kind,
        RtErrorKind::Io
    );
}

#[test]
fn builtins_cover_net_fetch_and_errors() {
    let mut host = RecordingHostBuilder::seeded()
        .net_fetch_response("201", "fetch-ok", "application/json")
        .build();
    let options = skepart::RtMap::new();
    options.insert("method", RtValue::String(RtString::from("POST")));
    options.insert("body", RtValue::String(RtString::from("{\"ok\":true}")));
    options.insert(
        "contentType",
        RtValue::String(RtString::from("application/json")),
    );

    let response = builtins::call_with_host(
        &mut host,
        "net",
        "fetch",
        &[
            RtValue::String(RtString::from("https://example.com/api")),
            RtValue::Map(options),
        ],
    )
    .expect("fetch should return result");

    let RtValue::Result(response) = response else {
        panic!("net.fetch should return a result");
    };
    let skepart::RtResultValue::Ok(response) = response else {
        panic!("net.fetch should return Ok(map) on success");
    };
    let RtValue::Map(response) = *response else {
        panic!("net.fetch should return Ok(map)");
    };
    assert_eq!(
        response.get("status").expect("status"),
        RtValue::String(RtString::from("201"))
    );
    assert_eq!(
        response.get("body").expect("body"),
        RtValue::String(RtString::from("fetch-ok"))
    );
    assert_eq!(
        response.get("contentType").expect("contentType"),
        RtValue::String(RtString::from("application/json"))
    );
    assert!(
        host.output
            .contains("[netfetch https://example.com/api method=POST]"),
        "unexpected host output: {}",
        host.output
    );

    let mut failing_host = RecordingHostBuilder::seeded()
        .net_fetch_error("fetch failed")
        .build();
    let empty_options = skepart::RtMap::new();
    let failed = builtins::call_with_host(
        &mut failing_host,
        "net",
        "fetch",
        &[
            RtValue::String(RtString::from("https://bad/")),
            RtValue::Map(empty_options),
        ],
    )
    .expect("fetch failure should return Err(string)");
    let RtValue::Result(failed) = failed else {
        panic!("net.fetch should return a result");
    };
    let skepart::RtResultValue::Err(failed) = failed else {
        panic!("net.fetch should return Err(string) on failure");
    };
    let RtValue::String(failed) = *failed else {
        panic!("net.fetch should return Err(string)");
    };
    assert!(failed.as_str().contains("fetch failed"));
}

#[test]
fn builtins_enforce_net_close_lifetime_rules() {
    let mut host = RecordingHostBuilder::seeded().build();
    let socket = builtins::call_with_host(&mut host, "net", "__testSocket", &[])
        .expect("allocate socket through builtin");
    let alias = socket.clone();

    assert_eq!(
        builtins::call_with_host(&mut host, "net", "close", std::slice::from_ref(&socket))
            .expect("close through builtin"),
        RtValue::Unit
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "close", std::slice::from_ref(&alias))
            .expect_err("alias should see closed handle")
            .kind,
        RtErrorKind::InvalidArgument
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "closeListener",
            std::slice::from_ref(&socket),
        )
        .expect_err("wrong handle kind should fail")
        .kind,
        RtErrorKind::InvalidArgument
    );
}

#[test]
fn builtins_surface_net_runtime_errors_consistently() {
    let mut failing_listen = RecordingHostBuilder::seeded()
        .net_listen_error("bad listen address")
        .build();
    assert_eq!(
        builtins::call_with_host(
            &mut failing_listen,
            "net",
            "listen",
            &[RtValue::String(RtString::from("bad"))],
        )
        .expect("listen failure should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("Io: bad listen address")
        )))
    );

    let mut failing_connect = RecordingHostBuilder::seeded()
        .net_connect_error("connect failed")
        .build();
    assert_eq!(
        builtins::call_with_host(
            &mut failing_connect,
            "net",
            "connect",
            &[RtValue::String(RtString::from("127.0.0.1:1"))],
        )
        .expect("connect failure should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("Io: connect failed")
        )))
    );

    let mut failing_tls_connect = RecordingHostBuilder::seeded()
        .net_tls_connect_error("tls connect failed")
        .build();
    assert_eq!(
        builtins::call_with_host(
            &mut failing_tls_connect,
            "net",
            "tlsConnect",
            &[
                RtValue::String(RtString::from("example.com")),
                RtValue::Int(443)
            ],
        )
        .expect("tlsConnect failure should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("Io: tls connect failed")
        )))
    );

    let mut failing_accept = RecordingHostBuilder::seeded()
        .net_accept_error("accept failed")
        .build();
    let listener = expect_ok_handle(
        builtins::call_with_host(
            &mut failing_accept,
            "net",
            "listen",
            &[RtValue::String(RtString::from("127.0.0.1:0"))],
        )
        .expect("listen should return result"),
        skepart::RtHandleKind::Listener,
        "net.listen",
    );
    assert_eq!(
        builtins::call_with_host(
            &mut failing_accept,
            "net",
            "accept",
            &[RtValue::Handle(listener)],
        )
        .expect("accept failure should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("Io: accept failed")
        )))
    );

    let mut failing_read = RecordingHostBuilder::seeded()
        .net_read_error("read failed")
        .build();
    let socket = builtins::call_with_host(&mut failing_read, "net", "__testSocket", &[])
        .expect("allocate socket");
    assert_eq!(
        builtins::call_with_host(
            &mut failing_read,
            "net",
            "read",
            std::slice::from_ref(&socket)
        )
        .expect("read failure should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("Io: read failed")
        )))
    );

    let mut failing_write = RecordingHostBuilder::seeded()
        .net_write_error("write failed")
        .build();
    let socket = builtins::call_with_host(&mut failing_write, "net", "__testSocket", &[])
        .expect("allocate socket");
    assert_eq!(
        builtins::call_with_host(
            &mut failing_write,
            "net",
            "write",
            &[socket, RtValue::String(RtString::from("ping"))],
        )
        .expect("write failure should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("Io: write failed")
        )))
    );

    let mut failing_flush = RecordingHostBuilder::seeded()
        .net_flush_error("flush failed")
        .build();
    let socket = builtins::call_with_host(&mut failing_flush, "net", "__testSocket", &[])
        .expect("allocate socket");
    assert_eq!(
        builtins::call_with_host(
            &mut failing_flush,
            "net",
            "flush",
            std::slice::from_ref(&socket),
        )
        .expect("flush failure should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("Io: flush failed")
        )))
    );

    let mut failing_read_timeout = RecordingHostBuilder::seeded()
        .net_set_read_timeout_error("set read timeout failed")
        .build();
    let socket = builtins::call_with_host(&mut failing_read_timeout, "net", "__testSocket", &[])
        .expect("allocate socket");
    assert_eq!(
        builtins::call_with_host(
            &mut failing_read_timeout,
            "net",
            "setReadTimeout",
            &[socket, RtValue::Int(1)],
        )
        .expect("setReadTimeout failure should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("Io: set read timeout failed")
        )))
    );

    let mut failing_write_timeout = RecordingHostBuilder::seeded()
        .net_set_write_timeout_error("set write timeout failed")
        .build();
    let socket = builtins::call_with_host(&mut failing_write_timeout, "net", "__testSocket", &[])
        .expect("allocate socket");
    assert_eq!(
        builtins::call_with_host(
            &mut failing_write_timeout,
            "net",
            "setWriteTimeout",
            &[socket, RtValue::Int(1)],
        )
        .expect("setWriteTimeout failure should return result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("Io: set write timeout failed")
        )))
    );
}

#[test]
fn builtins_reject_wrong_or_closed_net_handles_for_io() {
    let mut host = RecordingHostBuilder::seeded().build();
    let listener = expect_ok_handle(
        builtins::call_with_host(
            &mut host,
            "net",
            "listen",
            &[RtValue::String(RtString::from("127.0.0.1:0"))],
        )
        .expect("listener"),
        skepart::RtHandleKind::Listener,
        "net.listen",
    );
    let socket = builtins::call_with_host(&mut host, "net", "__testSocket", &[]).expect("socket");

    assert_eq!(
        builtins::call_with_host(&mut host, "net", "read", &[RtValue::Handle(listener)],)
            .expect_err("listener passed to read should fail")
            .kind,
        RtErrorKind::InvalidArgument
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "write",
            &[
                RtValue::Handle(listener),
                RtValue::String(RtString::from("ping"))
            ],
        )
        .expect_err("listener passed to write should fail")
        .kind,
        RtErrorKind::InvalidArgument
    );

    builtins::call_with_host(&mut host, "net", "close", std::slice::from_ref(&socket))
        .expect("close socket");
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "read", std::slice::from_ref(&socket))
            .expect("closed socket read should now return Err result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("InvalidArgument: unknown handle id 1")
        )))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "write",
            &[socket.clone(), RtValue::String(RtString::from("ping"))],
        )
        .expect("closed socket write should now return Err result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("InvalidArgument: unknown handle id 1")
        )))
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "flush", &[RtValue::Handle(listener)])
            .expect_err("listener passed to flush should fail")
            .kind,
        RtErrorKind::InvalidArgument
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "flush", std::slice::from_ref(&socket))
            .expect("closed socket flush should return Err result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("InvalidArgument: unknown handle id 1")
        )))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "setReadTimeout",
            &[RtValue::Handle(listener), RtValue::Int(5)],
        )
        .expect_err("listener passed to setReadTimeout should fail")
        .kind,
        RtErrorKind::InvalidArgument
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "setWriteTimeout",
            &[socket, RtValue::Int(5)],
        )
        .expect("closed socket setWriteTimeout should return Err result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("InvalidArgument: unknown handle id 1")
        )))
    );
}

#[test]
fn builtins_reject_wrong_or_closed_net_handles_for_accept() {
    let mut host = RecordingHostBuilder::seeded().build();
    let socket = builtins::call_with_host(&mut host, "net", "__testSocket", &[]).expect("socket");
    let listener = expect_ok_handle(
        builtins::call_with_host(
            &mut host,
            "net",
            "listen",
            &[RtValue::String(RtString::from("127.0.0.1:0"))],
        )
        .expect("listener"),
        skepart::RtHandleKind::Listener,
        "net.listen",
    );

    assert_eq!(
        builtins::call_with_host(&mut host, "net", "accept", std::slice::from_ref(&socket))
            .expect_err("socket passed to accept should fail")
            .kind,
        RtErrorKind::InvalidArgument
    );

    builtins::call_with_host(
        &mut host,
        "net",
        "closeListener",
        &[RtValue::Handle(listener)],
    )
    .expect("close listener");
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "accept", &[RtValue::Handle(listener)])
            .expect("closed listener should return Err result"),
        RtValue::Result(skepart::RtResultValue::err(RtValue::String(
            RtString::from("InvalidArgument: unknown handle id 1")
        )))
    );
}

#[test]
fn builtins_cover_more_io_arr_and_vec_edge_shapes() {
    let mut host = RecordingHostBuilder::seeded().build();
    assert_eq!(
        builtins::call(
            "arr",
            "join",
            &[
                RtValue::Array(skepart::RtArray::new(vec![
                    RtValue::String(RtString::from("a")),
                    RtValue::String(RtString::from("b")),
                ])),
                RtValue::String(RtString::from(",")),
            ],
        )
        .expect("arr.join"),
        RtValue::String(RtString::from("a,b"))
    );
    assert_eq!(
        builtins::call("io", "format", &[RtValue::String(RtString::from("%%"))]).expect("percent"),
        RtValue::String(RtString::from("%"))
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "io", "readLine", &[]).expect("read line"),
        RtValue::String(RtString::from("typed line"))
    );
}

#[test]
fn builtins_cover_host_backed_fs_os_and_random_families_more_thoroughly() {
    let mut host = RecordingHostBuilder::seeded()
        .file("note.txt", "seed-note")
        .build();

    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "fs",
            "exists",
            &[RtValue::String(RtString::from("exists.txt"))],
        )
        .expect("fs exists"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Bool(true)))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "fs",
            "readText",
            &[RtValue::String(RtString::from("note.txt"))],
        )
        .expect("fs read"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::String(RtString::from(
            "seed-note",
        ))))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "fs",
            "writeText",
            &[
                RtValue::String(RtString::from("a.txt")),
                RtValue::String(RtString::from("hello")),
            ],
        )
        .expect("fs write"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Unit))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "fs",
            "appendText",
            &[
                RtValue::String(RtString::from("a.txt")),
                RtValue::String(RtString::from("!")),
            ],
        )
        .expect("fs append"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Unit))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "fs",
            "mkdirAll",
            &[RtValue::String(RtString::from("tmp/dir"))],
        )
        .expect("mkdir"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Unit))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "fs",
            "removeFile",
            &[RtValue::String(RtString::from("a.txt"))],
        )
        .expect("rm file"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Unit))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "fs",
            "removeDirAll",
            &[RtValue::String(RtString::from("tmp/dir"))],
        )
        .expect("rm dir"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Unit))
    );

    assert_eq!(
        builtins::call_with_host(&mut host, "os", "arch", &[]).expect("arch"),
        RtValue::String(RtString::from("test-arch"))
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "os", "arg", &[RtValue::Int(1)]).expect("arg"),
        RtValue::Option(skepart::RtOption::some(RtValue::String(RtString::from(
            "--flag",
        ))))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "os",
            "envHas",
            &[RtValue::String(RtString::from("HOME"))],
        )
        .expect("envHas"),
        RtValue::Bool(true)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "os",
            "envGet",
            &[RtValue::String(RtString::from("HOME"))],
        )
        .expect("envGet"),
        RtValue::Option(skepart::RtOption::some(RtValue::String(RtString::from(
            "/tmp/home",
        ))))
    );
    builtins::call_with_host(
        &mut host,
        "os",
        "envSet",
        &[
            RtValue::String(RtString::from("MODE")),
            RtValue::String(RtString::from("debug")),
        ],
    )
    .expect("envSet");
    builtins::call_with_host(
        &mut host,
        "os",
        "envRemove",
        &[RtValue::String(RtString::from("HOME"))],
    )
    .expect("envRemove");
    builtins::call_with_host(&mut host, "os", "sleep", &[RtValue::Int(33)]).expect("sleep");
    builtins::call_with_host(&mut host, "os", "exit", &[RtValue::Int(7)]).expect("exit");
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "os",
            "exec",
            &[
                RtValue::String(RtString::from("git")),
                string_vec(&["status"])
            ],
        )
        .expect("exec"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Int(9)))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "os",
            "execOut",
            &[
                RtValue::String(RtString::from("git")),
                string_vec(&["rev-parse"])
            ],
        )
        .expect("execOut"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::String(RtString::from(
            "exec-out"
        ),)))
    );

    builtins::call_with_host(&mut host, "random", "seed", &[RtValue::Int(123)]).expect("seed");
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "random",
            "int",
            &[RtValue::Int(1), RtValue::Int(10)],
        )
        .expect("rand int"),
        RtValue::Int(5)
    );

    assert_eq!(
        host.output,
        "[write a.txt=hello][append a.txt+=!][mkdir tmp/dir][rmfile a.txt][rmdir tmp/dir][envset MODE=debug][envrm HOME][sleep 33][exit 7][exec git status][execout git rev-parse]"
    );
}

#[test]
fn builtins_cover_datetime_component_and_parse_shapes() {
    let mut host = RecordingHostBuilder::seeded()
        .unix_now(111)
        .millis_now(222)
        .build();

    assert_eq!(
        builtins::call_with_host(&mut host, "datetime", "fromUnix", &[RtValue::Int(5)],)
            .expect("from unix"),
        RtValue::String(RtString::from("unix:5"))
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "datetime", "fromMillis", &[RtValue::Int(5)],)
            .expect("from millis"),
        RtValue::String(RtString::from("millis:5"))
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "datetime",
            "parseUnix",
            &[RtValue::String(RtString::from("2025-03-17"))],
        )
        .expect("parse unix"),
        RtValue::Result(skepart::RtResultValue::ok(RtValue::Int(10)))
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "datetime", "year", &[RtValue::Int(100)])
            .expect("year"),
        RtValue::Int(104)
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "datetime", "second", &[RtValue::Int(100)])
            .expect("second"),
        RtValue::Int(106)
    );
}

#[test]
fn builtins_reject_typed_io_print_mismatches_and_format_extra_args() {
    let mut host = RecordingHostBuilder::seeded().build();
    assert_eq!(
        builtins::call_with_host(&mut host, "io", "printInt", &[RtValue::Bool(true)])
            .expect_err("typed print mismatch")
            .kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        builtins::call(
            "io",
            "format",
            &[
                RtValue::String(RtString::from("%d")),
                RtValue::Int(1),
                RtValue::Int(2),
            ],
        )
        .expect_err("extra args")
        .kind,
        RtErrorKind::InvalidArgument
    );
}

#[test]
fn builtins_reject_new_os_invalid_argument_shapes() {
    let mut host = RecordingHostBuilder::seeded().build();
    assert_eq!(
        builtins::call_with_host(&mut host, "os", "arg", &[RtValue::Int(-1)])
            .expect("negative arg index"),
        RtValue::Option(skepart::RtOption::none())
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "os", "arg", &[RtValue::Int(99)],)
            .expect("oob arg index"),
        RtValue::Option(skepart::RtOption::none())
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "os",
            "envGet",
            &[RtValue::String(RtString::from("MISSING"))],
        )
        .expect("missing env"),
        RtValue::Option(skepart::RtOption::none())
    );
    let bad_args = skepart::RtVec::new();
    bad_args.push(RtValue::Int(1));
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "os",
            "exec",
            &[
                RtValue::String(RtString::from("git")),
                RtValue::Vec(bad_args)
            ],
        )
        .expect_err("non-string exec arg")
        .kind,
        RtErrorKind::TypeMismatch
    );
}

#[test]
fn builtins_cover_option_and_result_inspection_helpers() {
    let some = builtins::call("option", "some", &[RtValue::Int(7)]).expect("option.some");
    let none = builtins::call("option", "none", &[]).expect("option.none");
    let ok = builtins::call("result", "ok", &[RtValue::String(RtString::from("ok"))])
        .expect("result.ok");
    let err = builtins::call("result", "err", &[RtValue::String(RtString::from("bad"))])
        .expect("result.err");

    assert_eq!(
        builtins::call("option", "isSome", std::slice::from_ref(&some)).expect("option.isSome"),
        RtValue::Bool(true)
    );
    assert_eq!(
        builtins::call("option", "isNone", std::slice::from_ref(&some)).expect("option.isNone"),
        RtValue::Bool(false)
    );
    assert_eq!(
        builtins::call("option", "isSome", std::slice::from_ref(&none)).expect("option.isSome"),
        RtValue::Bool(false)
    );
    assert_eq!(
        builtins::call("option", "isNone", std::slice::from_ref(&none)).expect("option.isNone"),
        RtValue::Bool(true)
    );
    assert_eq!(
        builtins::call("result", "isOk", std::slice::from_ref(&ok)).expect("result.isOk"),
        RtValue::Bool(true)
    );
    assert_eq!(
        builtins::call("result", "isErr", std::slice::from_ref(&ok)).expect("result.isErr"),
        RtValue::Bool(false)
    );
    assert_eq!(
        builtins::call("result", "isOk", std::slice::from_ref(&err)).expect("result.isOk"),
        RtValue::Bool(false)
    );
    assert_eq!(
        builtins::call("result", "isErr", std::slice::from_ref(&err)).expect("result.isErr"),
        RtValue::Bool(true)
    );
    assert_eq!(
        builtins::call("option", "isSome", &[RtValue::Int(1)])
            .expect_err("option.isSome type mismatch")
            .kind,
        RtErrorKind::TypeMismatch
    );
    assert_eq!(
        builtins::call("result", "isOk", &[RtValue::Int(1)])
            .expect_err("result.isOk type mismatch")
            .kind,
        RtErrorKind::TypeMismatch
    );
}
