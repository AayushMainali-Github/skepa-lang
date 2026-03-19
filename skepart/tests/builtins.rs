mod common;

use common::RecordingHostBuilder;
use skepart::{builtins, RtErrorKind, RtHost, RtResult, RtString, RtValue};

struct UnsupportedHost;

impl RtHost for UnsupportedHost {
    fn io_print(&mut self, _text: &str) -> RtResult<()> {
        Ok(())
    }
}

#[test]
fn builtins_dispatch_valid_core_families() {
    let mut host = RecordingHostBuilder::seeded().build();
    assert_eq!(
        builtins::call_with_host(&mut host, "datetime", "nowUnix", &[]).expect("datetime"),
        RtValue::Int(100)
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
        builtins::call_with_host(&mut host, "os", "cwd", &[]).expect("cwd"),
        RtValue::String(RtString::from("tmp/work"))
    );
    builtins::call_with_host(&mut host, "os", "sleep", &[RtValue::Int(33)]).expect("sleep");
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "os",
            "execShell",
            &[RtValue::String(RtString::from("echo hi"))],
        )
        .expect("shell"),
        RtValue::Int(9)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "os",
            "execShellOut",
            &[RtValue::String(RtString::from("echo hi"))],
        )
        .expect("shell out"),
        RtValue::String(RtString::from("shell-out"))
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
        "[write a.txt=hello][append a.txt+=!][mkdir tmp/dir][rmfile a.txt][rmdir tmp/dir][sleep 33][sh echo hi][shout echo hi]"
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
