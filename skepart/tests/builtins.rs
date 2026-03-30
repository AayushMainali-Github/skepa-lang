mod common;

use common::RecordingHostBuilder;
use skepart::{builtins, RtBytes, RtErrorKind, RtHost, RtResult, RtString, RtValue};

struct UnsupportedHost;

impl RtHost for UnsupportedHost {
    fn io_print(&mut self, _text: &str) -> RtResult<()> {
        Ok(())
    }
}

fn string_vec(items: &[&str]) -> RtValue {
    let value = skepart::RtVec::new();
    for item in items {
        value.push(RtValue::String(RtString::from(*item)));
    }
    RtValue::Vec(value)
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
        RtValue::String(RtString::from("hello"))
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
        RtValue::Int(101)
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
            "get",
            &[
                RtValue::Bytes(RtBytes::from("a".as_bytes())),
                RtValue::Int(-1)
            ],
        )
        .expect_err("bytes.get negative index")
        .kind,
        RtErrorKind::IndexOutOfBounds
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
fn builtins_cover_net_bytes_roundtrip_and_errors() {
    let mut host = RecordingHostBuilder::seeded()
        .net_read_bytes_value(vec![0xDE, 0xAD, 0xBE, 0xEF])
        .build();
    let socket =
        builtins::call_with_host(&mut host, "net", "__testSocket", &[]).expect("allocate socket");

    assert_eq!(
        builtins::call_with_host(&mut host, "net", "readBytes", std::slice::from_ref(&socket))
            .expect("net.readBytes"),
        RtValue::Bytes(RtBytes::from(vec![0xDE, 0xAD, 0xBE, 0xEF]))
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
        RtValue::Unit
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
        RtValue::String(RtString::from("127.0.0.1:7000"))
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "peerAddr", std::slice::from_ref(&socket))
            .expect("net.peerAddr"),
        RtValue::String(RtString::from("127.0.0.1:8000"))
    );
    assert!(
        host.output.contains("[netlocaladdr 0]") && host.output.contains("[netpeeraddr 0]"),
        "unexpected host output: {}",
        host.output
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
        RtValue::Handle(skepart::RtHandle {
            id: 1,
            kind: skepart::RtHandleKind::Listener,
        })
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
        RtValue::String(RtString::from("net-read"))
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
        builtins::call_with_host(
            &mut host,
            "net",
            "connect",
            &[RtValue::String(RtString::from("127.0.0.1:8080"))],
        )
        .expect("connect should return socket"),
        RtValue::Handle(skepart::RtHandle {
            id: 0,
            kind: skepart::RtHandleKind::Socket,
        })
    );

    let listener = builtins::call_with_host(
        &mut host,
        "net",
        "listen",
        &[RtValue::String(RtString::from("127.0.0.1:0"))],
    )
    .expect("listen should return listener");
    assert_eq!(
        listener,
        RtValue::Handle(skepart::RtHandle {
            id: 1,
            kind: skepart::RtHandleKind::Listener,
        })
    );

    assert_eq!(
        builtins::call_with_host(&mut host, "net", "accept", &[listener])
            .expect("accept should return socket"),
        RtValue::Handle(skepart::RtHandle {
            id: 2,
            kind: skepart::RtHandleKind::Socket,
        })
    );
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
        .expect_err("listen failure should surface")
        .kind,
        RtErrorKind::Io
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
        .expect_err("connect failure should surface")
        .kind,
        RtErrorKind::Io
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
        .expect_err("read failure should surface")
        .kind,
        RtErrorKind::Io
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
        .expect_err("write failure should surface")
        .kind,
        RtErrorKind::Io
    );
}

#[test]
fn builtins_reject_wrong_or_closed_net_handles_for_io() {
    let mut host = RecordingHostBuilder::seeded().build();
    let listener = builtins::call_with_host(
        &mut host,
        "net",
        "listen",
        &[RtValue::String(RtString::from("127.0.0.1:0"))],
    )
    .expect("listener");
    let socket = builtins::call_with_host(&mut host, "net", "__testSocket", &[]).expect("socket");

    assert_eq!(
        builtins::call_with_host(&mut host, "net", "read", std::slice::from_ref(&listener))
            .expect_err("listener passed to read should fail")
            .kind,
        RtErrorKind::InvalidArgument
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "write",
            &[listener.clone(), RtValue::String(RtString::from("ping"))],
        )
        .expect_err("listener passed to write should fail")
        .kind,
        RtErrorKind::InvalidArgument
    );

    builtins::call_with_host(&mut host, "net", "close", std::slice::from_ref(&socket))
        .expect("close socket");
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "read", std::slice::from_ref(&socket))
            .expect_err("closed socket read should fail")
            .kind,
        RtErrorKind::InvalidArgument
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "net",
            "write",
            &[socket, RtValue::String(RtString::from("ping"))],
        )
        .expect_err("closed socket write should fail")
        .kind,
        RtErrorKind::InvalidArgument
    );
}

#[test]
fn builtins_reject_wrong_or_closed_net_handles_for_accept() {
    let mut host = RecordingHostBuilder::seeded().build();
    let socket = builtins::call_with_host(&mut host, "net", "__testSocket", &[]).expect("socket");
    let listener = builtins::call_with_host(
        &mut host,
        "net",
        "listen",
        &[RtValue::String(RtString::from("127.0.0.1:0"))],
    )
    .expect("listener");

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
        std::slice::from_ref(&listener),
    )
    .expect("close listener");
    assert_eq!(
        builtins::call_with_host(&mut host, "net", "accept", std::slice::from_ref(&listener))
            .expect_err("closed listener passed to accept should fail")
            .kind,
        RtErrorKind::InvalidArgument
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
        RtValue::Bool(true)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "fs",
            "readText",
            &[RtValue::String(RtString::from("note.txt"))],
        )
        .expect("fs read"),
        RtValue::String(RtString::from("seed-note"))
    );
    builtins::call_with_host(
        &mut host,
        "fs",
        "writeText",
        &[
            RtValue::String(RtString::from("a.txt")),
            RtValue::String(RtString::from("hello")),
        ],
    )
    .expect("fs write");
    builtins::call_with_host(
        &mut host,
        "fs",
        "appendText",
        &[
            RtValue::String(RtString::from("a.txt")),
            RtValue::String(RtString::from("!")),
        ],
    )
    .expect("fs append");
    builtins::call_with_host(
        &mut host,
        "fs",
        "mkdirAll",
        &[RtValue::String(RtString::from("tmp/dir"))],
    )
    .expect("mkdir");
    builtins::call_with_host(
        &mut host,
        "fs",
        "removeFile",
        &[RtValue::String(RtString::from("a.txt"))],
    )
    .expect("rm file");
    builtins::call_with_host(
        &mut host,
        "fs",
        "removeDirAll",
        &[RtValue::String(RtString::from("tmp/dir"))],
    )
    .expect("rm dir");

    assert_eq!(
        builtins::call_with_host(&mut host, "os", "arch", &[]).expect("arch"),
        RtValue::String(RtString::from("test-arch"))
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "os", "arg", &[RtValue::Int(1)]).expect("arg"),
        RtValue::String(RtString::from("--flag"))
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
        RtValue::String(RtString::from("/tmp/home"))
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
        RtValue::Int(9)
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
        RtValue::String(RtString::from("exec-out"))
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
        RtValue::Int(10)
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
            .expect_err("negative arg index")
            .kind,
        RtErrorKind::InvalidArgument
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "os", "arg", &[RtValue::Int(99)],)
            .expect_err("oob arg index")
            .kind,
        RtErrorKind::IndexOutOfBounds
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "os",
            "envGet",
            &[RtValue::String(RtString::from("MISSING"))],
        )
        .expect_err("missing env")
        .kind,
        RtErrorKind::InvalidArgument
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
