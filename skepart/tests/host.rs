mod common;

use common::RecordingHostBuilder;
use rcgen::generate_simple_self_signed;
use rustls::pki_types::PrivatePkcs8KeyDer;
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use skepart::{NoopHost, RtBytes, RtHandle, RtHandleKind, RtHost, RtString};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

#[cfg(windows)]
fn ffi_test_library_path() -> &'static str {
    "kernel32.dll"
}

#[cfg(windows)]
fn ffi_test_symbol_name() -> &'static str {
    "GetCurrentProcessId"
}

#[cfg(windows)]
fn ffi_test_call0_int_expected() -> i64 {
    assert!(std::process::id() <= i64::MAX as u32);
    std::process::id() as i64
}

#[cfg(windows)]
fn ffi_test_call1_int_symbol_name() -> &'static str {
    "lstrlenA"
}

#[cfg(windows)]
fn ffi_test_call1_int_value() -> i64 {
    0
}

#[cfg(windows)]
fn ffi_test_call1_int_expected() -> i64 {
    0
}

#[cfg(windows)]
fn ffi_test_call1_string_int_symbol_name() -> &'static str {
    "lstrlenA"
}

#[cfg(windows)]
fn ffi_test_call1_string_int_value() -> &'static str {
    "hello"
}

#[cfg(windows)]
fn ffi_test_call1_string_int_expected() -> i64 {
    5
}

#[cfg(windows)]
fn ffi_test_call1_string_void_library_path() -> &'static str {
    "kernel32.dll"
}

#[cfg(windows)]
fn ffi_test_call1_string_void_symbol_name() -> &'static str {
    "OutputDebugStringA"
}

#[cfg(windows)]
fn ffi_test_call1_int_void_library_path() -> &'static str {
    "ucrtbase.dll"
}

#[cfg(windows)]
fn ffi_test_call1_int_void_symbol_name() -> &'static str {
    "srand"
}

#[cfg(windows)]
fn ffi_test_call2_string_int_symbol_name() -> &'static str {
    "lstrcmpA"
}

#[cfg(windows)]
fn ffi_test_call2_string_int_int_library_path() -> &'static str {
    "ucrtbase.dll"
}

#[cfg(windows)]
fn ffi_test_call2_string_int_int_symbol_name() -> &'static str {
    "strnlen"
}

#[cfg(windows)]
fn ffi_test_call0_void_library_path() -> &'static str {
    "ucrtbase.dll"
}

#[cfg(windows)]
fn ffi_test_call0_void_symbol_name() -> &'static str {
    "_tzset"
}

#[cfg(windows)]
fn ffi_test_call1_bytes_library_path() -> &'static str {
    "ucrtbase.dll"
}

#[cfg(windows)]
fn ffi_test_call1_bytes_symbol_name() -> &'static str {
    "strnlen"
}

#[cfg(windows)]
fn ffi_test_call1_bytes_expected() -> i64 {
    5
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_library_path() -> &'static str {
    "libc.so.6"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_symbol_name() -> &'static str {
    "getpid"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call0_int_expected() -> i64 {
    std::process::id() as i64
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_int_symbol_name() -> &'static str {
    "abs"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_int_value() -> i64 {
    -9
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_int_expected() -> i64 {
    9
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_string_int_symbol_name() -> &'static str {
    "strlen"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_string_int_value() -> &'static str {
    "hello"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_string_int_expected() -> i64 {
    5
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_string_void_library_path() -> &'static str {
    "libc.so.6"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_string_void_symbol_name() -> &'static str {
    "perror"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_int_void_library_path() -> &'static str {
    "libc.so.6"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_int_void_symbol_name() -> &'static str {
    "srand"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call2_string_int_symbol_name() -> &'static str {
    "strcmp"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call2_string_int_int_library_path() -> &'static str {
    "libc.so.6"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call2_string_int_int_symbol_name() -> &'static str {
    "strnlen"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call0_void_library_path() -> &'static str {
    "libc.so.6"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call0_void_symbol_name() -> &'static str {
    "tzset"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_bytes_library_path() -> &'static str {
    "libc.so.6"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_bytes_symbol_name() -> &'static str {
    "strnlen"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ffi_test_call1_bytes_expected() -> i64 {
    5
}

#[cfg(target_os = "macos")]
fn ffi_test_library_path() -> &'static str {
    "/usr/lib/libSystem.B.dylib"
}

#[cfg(target_os = "macos")]
fn ffi_test_symbol_name() -> &'static str {
    "getpid"
}

#[cfg(target_os = "macos")]
fn ffi_test_call0_int_expected() -> i64 {
    std::process::id() as i64
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_int_symbol_name() -> &'static str {
    "abs"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_int_value() -> i64 {
    -9
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_int_expected() -> i64 {
    9
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_string_int_symbol_name() -> &'static str {
    "strlen"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_string_int_value() -> &'static str {
    "hello"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_string_int_expected() -> i64 {
    5
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_string_void_library_path() -> &'static str {
    "/usr/lib/libSystem.B.dylib"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_string_void_symbol_name() -> &'static str {
    "perror"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_int_void_library_path() -> &'static str {
    "/usr/lib/libSystem.B.dylib"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_int_void_symbol_name() -> &'static str {
    "srand"
}

#[cfg(target_os = "macos")]
fn ffi_test_call2_string_int_symbol_name() -> &'static str {
    "strcmp"
}

#[cfg(target_os = "macos")]
fn ffi_test_call2_string_int_int_library_path() -> &'static str {
    "/usr/lib/libSystem.B.dylib"
}

#[cfg(target_os = "macos")]
fn ffi_test_call2_string_int_int_symbol_name() -> &'static str {
    "strnlen"
}

#[cfg(target_os = "macos")]
fn ffi_test_call0_void_library_path() -> &'static str {
    "/usr/lib/libSystem.B.dylib"
}

#[cfg(target_os = "macos")]
fn ffi_test_call0_void_symbol_name() -> &'static str {
    "tzset"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_bytes_library_path() -> &'static str {
    "/usr/lib/libSystem.B.dylib"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_bytes_symbol_name() -> &'static str {
    "strnlen"
}

#[cfg(target_os = "macos")]
fn ffi_test_call1_bytes_expected() -> i64 {
    5
}

#[test]
fn noop_host_supports_print_and_time_defaults() {
    let mut host = NoopHost::default();
    host.io_print("hello").expect("print");
    host.io_println("world").expect("println");
    assert!(host.datetime_now_unix().expect("unix") > 0);
    assert!(host.datetime_now_millis().expect("millis") > 0);
    assert_eq!(
        host.datetime_from_unix(0).expect("from unix"),
        RtString::from("1970-01-01T00:00:00Z")
    );
    assert_eq!(
        host.datetime_from_millis(1234).expect("from millis"),
        RtString::from("1970-01-01T00:00:01.234Z")
    );
    assert_eq!(
        host.datetime_parse_unix("1970-01-01T00:00:00Z")
            .expect("parse unix"),
        0
    );
    assert_eq!(host.datetime_component("year", 0).expect("year"), 1970);
}

#[test]
fn recording_host_captures_output_and_overrides_services() {
    let mut host = RecordingHostBuilder::seeded().build();
    host.io_print("a").expect("print");
    host.io_println("b").expect("println");
    assert_eq!(host.output, "ab\n");
    assert_eq!(
        host.io_read_line().expect("read line"),
        RtString::from("typed line")
    );
    assert_eq!(
        host.os_platform().expect("platform"),
        RtString::from("test-os")
    );
    assert_eq!(host.os_arch().expect("arch"), RtString::from("test-arch"));
    assert_eq!(host.os_arg(0).expect("arg0"), RtString::from("skepa"));
    assert!(host.os_env_has("HOME").expect("env has"));
    assert_eq!(
        host.os_env_get("HOME").expect("env get"),
        RtString::from("/tmp/home")
    );
    assert_eq!(
        host.fs_read_text("file.txt").expect("read"),
        RtString::from("read:file.txt")
    );
}

#[test]
fn recording_host_tracks_fs_os_and_random_side_effects() {
    let mut host = RecordingHostBuilder::seeded()
        .file("f.txt", "seed")
        .existing_path("dir", true)
        .build();
    assert_eq!(host.random_int(1, 9).expect("rand int"), 5);
    assert_eq!(host.random_float().expect("rand float"), 0.25);
    assert!(host.fs_exists("exists.txt").expect("exists"));
    assert_eq!(host.fs_join("a", "b").expect("join"), RtString::from("a/b"));
    assert_eq!(
        host.os_exec("hostname", &["--help".into()]).expect("exec"),
        9
    );
    assert_eq!(
        host.os_exec_out("hostname", &["--help".into()])
            .expect("exec out"),
        RtString::from("exec-out")
    );
    host.os_env_set("MODE", "debug").expect("env set");
    host.os_env_remove("MODE").expect("env remove");
    host.os_exit(7).expect("exit");
    host.fs_write_text("f.txt", "x").expect("write");
    host.fs_append_text("f.txt", "y").expect("append");
    host.fs_mkdir_all("dir").expect("mkdir");
    host.fs_remove_file("f.txt").expect("rm file");
    host.fs_remove_dir_all("dir").expect("rm dir");
    host.os_sleep(12).expect("sleep");
    assert_eq!(
        host.output,
        "[exec hostname --help][execout hostname --help][envset MODE=debug][envrm MODE][exit 7][write f.txt=x][append f.txt+=y][mkdir dir][rmfile f.txt][rmdir dir][sleep 12]"
    );
}

#[test]
fn hosts_can_construct_typed_placeholder_net_handles() {
    let mut noop = NoopHost::default();
    assert_eq!(
        noop.net_make_socket_handle(1).expect("socket handle"),
        RtHandle {
            id: 1,
            kind: RtHandleKind::Socket,
        }
    );
    assert_eq!(
        noop.net_make_listener_handle(2).expect("listener handle"),
        RtHandle {
            id: 2,
            kind: RtHandleKind::Listener,
        }
    );
}

#[test]
fn noop_host_opens_and_binds_real_shared_library_symbols() {
    let mut host = NoopHost::default();
    let library = host
        .ffi_open_library(ffi_test_library_path())
        .expect("open shared library");
    assert_eq!(library.kind, RtHandleKind::Library);
    let symbol = host
        .ffi_bind_symbol(library, ffi_test_symbol_name())
        .expect("bind symbol");
    assert_eq!(symbol.kind, RtHandleKind::Symbol);
    assert_eq!(
        host.net_lookup_handle_kind(library).expect("library kind"),
        RtHandleKind::Library
    );
    assert_eq!(
        host.net_lookup_handle_kind(symbol).expect("symbol kind"),
        RtHandleKind::Symbol
    );
    host.net_close_handle(symbol).expect("close symbol");
    host.net_close_handle(library).expect("close library");
}

#[test]
fn noop_host_calls_integer_ffi_symbols() {
    let mut host = NoopHost::default();
    let library = host
        .ffi_open_library(ffi_test_library_path())
        .expect("open shared library");
    let call0 = host
        .ffi_bind_symbol(library, ffi_test_symbol_name())
        .expect("bind zero-arg int symbol");
    let call1 = host
        .ffi_bind_symbol(library, ffi_test_call1_int_symbol_name())
        .expect("bind one-arg int symbol");

    assert_eq!(
        host.ffi_call_0_int(call0).expect("call0"),
        ffi_test_call0_int_expected()
    );
    assert_eq!(
        host.ffi_call_1_int(call1, ffi_test_call1_int_value())
            .expect("call1"),
        ffi_test_call1_int_expected()
    );
    host.net_close_handle(call0).expect("close call0");
    host.net_close_handle(call1).expect("close call1");
    host.net_close_handle(library).expect("close library");
}

#[test]
fn noop_host_calls_zero_void_ffi_symbols() {
    let mut host = NoopHost::default();
    let library = host
        .ffi_open_library(ffi_test_call0_void_library_path())
        .expect("open shared library");
    let symbol = host
        .ffi_bind_symbol(library, ffi_test_call0_void_symbol_name())
        .expect("bind zero-void symbol");

    host.ffi_call_0_void(symbol).expect("call0Void");

    host.net_close_handle(symbol).expect("close symbol");
    host.net_close_handle(library).expect("close library");
}

#[test]
fn noop_host_calls_borrowed_string_ffi_symbols() {
    let mut host = NoopHost::default();
    let library = host
        .ffi_open_library(ffi_test_library_path())
        .expect("open shared library");
    let symbol = host
        .ffi_bind_symbol(library, ffi_test_call1_string_int_symbol_name())
        .expect("bind one-string int symbol");

    assert_eq!(
        host.ffi_call_1_string_int(symbol, ffi_test_call1_string_int_value())
            .expect("call1StringInt"),
        ffi_test_call1_string_int_expected()
    );

    host.net_close_handle(symbol).expect("close symbol");
    host.net_close_handle(library).expect("close library");
}

#[test]
fn noop_host_calls_borrowed_string_void_ffi_symbols() {
    let mut host = NoopHost::default();
    let library = host
        .ffi_open_library(ffi_test_call1_string_void_library_path())
        .expect("open shared library");
    let symbol = host
        .ffi_bind_symbol(library, ffi_test_call1_string_void_symbol_name())
        .expect("bind one-string void symbol");

    host.ffi_call_1_string_void(symbol, "hello")
        .expect("call1StringVoid");

    host.net_close_handle(symbol).expect("close symbol");
    host.net_close_handle(library).expect("close library");
}

#[test]
fn noop_host_calls_int_void_ffi_symbols() {
    let mut host = NoopHost::default();
    let library = host
        .ffi_open_library(ffi_test_call1_int_void_library_path())
        .expect("open shared library");
    let symbol = host
        .ffi_bind_symbol(library, ffi_test_call1_int_void_symbol_name())
        .expect("bind one-int void symbol");

    host.ffi_call_1_int_void(symbol, 123).expect("call1IntVoid");

    host.net_close_handle(symbol).expect("close symbol");
    host.net_close_handle(library).expect("close library");
}

#[test]
fn noop_host_calls_two_borrowed_string_ffi_symbols() {
    let mut host = NoopHost::default();
    let library = host
        .ffi_open_library(ffi_test_library_path())
        .expect("open shared library");
    let symbol = host
        .ffi_bind_symbol(library, ffi_test_call2_string_int_symbol_name())
        .expect("bind two-string int symbol");

    assert_eq!(
        host.ffi_call_2_string_int(symbol, "same", "same")
            .expect("call2StringInt"),
        0
    );

    host.net_close_handle(symbol).expect("close symbol");
    host.net_close_handle(library).expect("close library");
}

#[test]
fn noop_host_calls_borrowed_string_int_ffi_symbols() {
    let mut host = NoopHost::default();
    let library = host
        .ffi_open_library(ffi_test_call2_string_int_int_library_path())
        .expect("open shared library");
    let symbol = host
        .ffi_bind_symbol(library, ffi_test_call2_string_int_int_symbol_name())
        .expect("bind string-int int symbol");

    assert_eq!(
        host.ffi_call_2_string_int_int(symbol, "hello", 3)
            .expect("call2StringIntInt"),
        3
    );

    host.net_close_handle(symbol).expect("close symbol");
    host.net_close_handle(library).expect("close library");
}

#[test]
fn noop_host_calls_borrowed_bytes_ffi_symbols() {
    let mut host = NoopHost::default();
    let library = host
        .ffi_open_library(ffi_test_call1_bytes_library_path())
        .expect("open shared library");
    let symbol = host
        .ffi_bind_symbol(library, ffi_test_call1_bytes_symbol_name())
        .expect("bind one-bytes int symbol");

    assert_eq!(
        host.ffi_call_1_bytes_int(symbol, &RtBytes::from(b"hello".to_vec()))
            .expect("call1BytesInt"),
        ffi_test_call1_bytes_expected()
    );

    host.net_close_handle(symbol).expect("close symbol");
    host.net_close_handle(library).expect("close library");
}

#[test]
fn recording_host_covers_new_bool_and_mixed_ffi_shapes() {
    let mut host = RecordingHostBuilder::seeded().build();
    let library = host.ffi_open_library("test-lib").expect("open");
    let symbol = host.ffi_bind_symbol(library, "plus").expect("bind");

    assert!(host.ffi_call_0_bool(symbol).expect("call0Bool"));
    assert!(host.ffi_call_1_int_bool(symbol, 3).expect("call1IntBool"));
    assert_eq!(
        host.ffi_call_2_int_int(symbol, 2, 5).expect("call2IntInt"),
        7
    );
    assert_eq!(
        host.ffi_call_2_bytes_int_int(symbol, &RtBytes::from(b"abc".to_vec()), 4)
            .expect("call2BytesIntInt"),
        7
    );
}

#[test]
fn noop_host_tracks_placeholder_net_handle_lifetimes() {
    let mut host = NoopHost::default();
    let socket = host
        .net_alloc_handle(RtHandleKind::Socket)
        .expect("allocate socket");
    let listener = host
        .net_alloc_handle(RtHandleKind::Listener)
        .expect("allocate listener");

    assert_eq!(socket.id, 0);
    assert_eq!(listener.id, 1);
    assert_eq!(
        host.net_lookup_handle_kind(socket).expect("lookup socket"),
        RtHandleKind::Socket
    );
    assert_eq!(
        host.net_lookup_handle_kind(listener)
            .expect("lookup listener"),
        RtHandleKind::Listener
    );

    host.net_close_handle(socket).expect("close socket");
    assert_eq!(
        host.net_lookup_handle_kind(socket)
            .expect_err("closed handle should be gone")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
    assert_eq!(
        host.net_close_handle(socket)
            .expect_err("double close should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
}

#[test]
fn noop_host_stores_and_recovers_live_tcp_resources_by_handle() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let client = TcpStream::connect(addr).expect("connect client");
    let (server, _) = listener.accept().expect("accept server");

    let mut host = NoopHost::default();
    let listener_handle = host
        .net_store_tcp_listener(listener)
        .expect("store listener");
    let socket_handle = host.net_store_tcp_stream(server).expect("store server");

    assert_eq!(
        host.net_lookup_handle_kind(listener_handle)
            .expect("listener kind"),
        RtHandleKind::Listener
    );
    assert_eq!(
        host.net_lookup_handle_kind(socket_handle)
            .expect("socket kind"),
        RtHandleKind::Socket
    );
    assert_eq!(
        host.net_tcp_listener(listener_handle)
            .expect("typed listener lookup")
            .local_addr()
            .expect("stored listener addr"),
        addr
    );

    host.net_tcp_stream(socket_handle)
        .expect("typed socket lookup")
        .write_all(b"ping")
        .expect("write server->client");
    let mut buf = [0_u8; 4];
    let mut client = client;
    client.read_exact(&mut buf).expect("read client");
    assert_eq!(&buf, b"ping");

    host.net_close_handle(socket_handle)
        .expect("close stored socket");
    assert_eq!(
        host.net_tcp_stream(socket_handle)
            .expect_err("closed socket should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
    assert_eq!(
        host.net_close_handle(socket_handle)
            .expect_err("double close should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
}

#[test]
fn noop_host_closing_any_handle_alias_closes_the_underlying_resource() {
    let mut host = NoopHost::default();
    let socket = host
        .net_alloc_handle(RtHandleKind::Socket)
        .expect("allocate socket");
    let alias = socket;

    host.net_close_handle(socket)
        .expect("close through first alias");

    assert_eq!(
        host.net_lookup_handle_kind(alias)
            .expect_err("alias should see closed resource")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
}

#[test]
fn noop_host_joins_running_tasks_once() {
    let mut host = NoopHost::default();
    let task = host
        .task_store_running(std::thread::spawn(|| Ok(skepart::RtValue::Int(55))))
        .expect("store running task");

    assert_eq!(
        host.task_join(task).expect("join task"),
        skepart::RtValue::Int(55)
    );
    assert_eq!(
        host.task_join(task)
            .expect_err("double join should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
}

#[test]
fn noop_host_rejects_using_socket_handle_as_listener_handle() {
    let mut host = NoopHost::default();
    let socket = host
        .net_alloc_handle(RtHandleKind::Socket)
        .expect("allocate socket");

    assert_eq!(
        host.net_tcp_listener(socket)
            .expect_err("wrong handle kind should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
}

#[test]
fn noop_host_rejects_using_listener_handle_as_socket_handle() {
    let mut host = NoopHost::default();
    let listener = host
        .net_alloc_handle(RtHandleKind::Listener)
        .expect("allocate listener");

    assert_eq!(
        host.net_tcp_stream(listener)
            .expect_err("wrong handle kind should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
}

#[test]
fn noop_host_supports_loopback_connect_accept_write_and_read() {
    let mut host = NoopHost::default();
    let listener = host.net_listen("127.0.0.1:0").expect("listen");
    let addr = host
        .net_tcp_listener(listener)
        .expect("listener lookup")
        .local_addr()
        .expect("listener addr");

    let client = host
        .net_connect(&addr.to_string())
        .expect("connect client socket");
    let server = host.net_accept(listener).expect("accept server socket");

    host.net_write(server, "ping")
        .expect("write server->client");
    assert_eq!(
        host.net_read(client).expect("read client"),
        RtString::from("ping")
    );

    host.net_close_handle(server).expect("close server");
    host.net_close_handle(client).expect("close client");
    host.net_close_handle(listener).expect("close listener");
}

#[test]
fn noop_host_supports_loopback_connect_accept_write_and_read_bytes() {
    let mut host = NoopHost::default();
    let listener = host.net_listen("127.0.0.1:0").expect("listen");
    let addr = host
        .net_tcp_listener(listener)
        .expect("listener lookup")
        .local_addr()
        .expect("listener addr");

    let client = host
        .net_connect(&addr.to_string())
        .expect("connect client socket");
    let server = host.net_accept(listener).expect("accept server socket");

    host.net_write_bytes(server, &RtBytes::from(vec![1_u8, 2, 3, 4]))
        .expect("write bytes server->client");
    assert_eq!(
        host.net_read_bytes(client).expect("read client bytes"),
        RtBytes::from(vec![1_u8, 2, 3, 4])
    );

    host.net_close_handle(server).expect("close server");
    host.net_close_handle(client).expect("close client");
    host.net_close_handle(listener).expect("close listener");
}

#[test]
fn noop_host_reports_local_and_peer_addrs_for_connected_socket() {
    let mut host = NoopHost::default();
    let listener = host.net_listen("127.0.0.1:0").expect("listen");
    let listener_addr = host
        .net_tcp_listener(listener)
        .expect("listener lookup")
        .local_addr()
        .expect("listener addr");

    let client = host
        .net_connect(&listener_addr.to_string())
        .expect("connect client socket");
    let server = host.net_accept(listener).expect("accept server socket");

    let client_local = host.net_local_addr(client).expect("client local");
    let client_peer = host.net_peer_addr(client).expect("client peer");
    let server_local = host.net_local_addr(server).expect("server local");
    let server_peer = host.net_peer_addr(server).expect("server peer");

    assert_eq!(client_peer.as_str(), listener_addr.to_string());
    assert_eq!(server_local.as_str(), listener_addr.to_string());
    assert_eq!(client_local.as_str(), server_peer.as_str());

    host.net_close_handle(server).expect("close server");
    host.net_close_handle(client).expect("close client");
    host.net_close_handle(listener).expect("close listener");
}

#[test]
fn noop_host_supports_exact_byte_reads_with_read_n() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener addr");
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        stream.write_all(&[7_u8, 8, 9, 10]).expect("write bytes");
    });

    let mut host = NoopHost::default();
    let client = host
        .net_connect(&addr.to_string())
        .expect("connect client socket");

    assert_eq!(
        host.net_read_n(client, 3).expect("read exact bytes"),
        RtBytes::from(vec![7_u8, 8, 9])
    );
    peer.join().expect("join peer");
    host.net_close_handle(client).expect("close client");
}

#[test]
fn noop_host_supports_flushing_connected_socket() {
    let mut host = NoopHost::default();
    let listener = host.net_listen("127.0.0.1:0").expect("listen");
    let addr = host
        .net_tcp_listener(listener)
        .expect("listener lookup")
        .local_addr()
        .expect("listener addr");

    let client = host
        .net_connect(&addr.to_string())
        .expect("connect client socket");
    let server = host.net_accept(listener).expect("accept server socket");

    host.net_write(server, "ping")
        .expect("write server->client");
    host.net_flush(server).expect("flush server");
    assert_eq!(
        host.net_read(client).expect("read client"),
        RtString::from("ping")
    );

    host.net_close_handle(server).expect("close server");
    host.net_close_handle(client).expect("close client");
    host.net_close_handle(listener).expect("close listener");
}

#[test]
fn noop_host_supports_setting_socket_timeouts() {
    let mut host = NoopHost::default();
    let listener = host.net_listen("127.0.0.1:0").expect("listen");
    let addr = host
        .net_tcp_listener(listener)
        .expect("listener lookup")
        .local_addr()
        .expect("listener addr");

    let client = host
        .net_connect(&addr.to_string())
        .expect("connect client socket");
    let server = host.net_accept(listener).expect("accept server socket");

    host.net_set_read_timeout(client, 25)
        .expect("set read timeout");
    host.net_set_write_timeout(client, 50)
        .expect("set write timeout");
    assert_eq!(
        host.net_tcp_stream(client)
            .expect("socket lookup")
            .read_timeout()
            .expect("read timeout"),
        Some(std::time::Duration::from_millis(25))
    );
    assert_eq!(
        host.net_tcp_stream(client)
            .expect("socket lookup")
            .write_timeout()
            .expect("write timeout"),
        Some(std::time::Duration::from_millis(50))
    );

    host.net_set_read_timeout(client, 0)
        .expect("clear read timeout");
    host.net_set_write_timeout(client, 0)
        .expect("clear write timeout");
    assert_eq!(
        host.net_tcp_stream(client)
            .expect("socket lookup")
            .read_timeout()
            .expect("read timeout"),
        None
    );
    assert_eq!(
        host.net_tcp_stream(client)
            .expect("socket lookup")
            .write_timeout()
            .expect("write timeout"),
        None
    );

    host.net_close_handle(server).expect("close server");
    host.net_close_handle(client).expect("close client");
    host.net_close_handle(listener).expect("close listener");
}

#[test]
fn noop_host_supports_tls_connect_with_trusted_local_root() {
    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("generate cert");
    let cert_der = cert.cert.der().clone();
    let key_der = PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der.clone()], key_der.into())
        .expect("server config");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind tls listener");
    let addr = listener.local_addr().expect("listener addr");
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept tls client");
        let conn = ServerConnection::new(Arc::new(server_config)).expect("server connection");
        let mut tls = StreamOwned::new(conn, stream);
        let mut buf = [0_u8; 4];
        tls.read_exact(&mut buf).expect("read tls payload");
        assert_eq!(&buf, b"ping");
        tls.write_all(b"pong").expect("write tls response");
        tls.flush().expect("flush tls response");
    });

    let mut host = NoopHost::default();
    host.add_tls_root_certificate(cert_der);
    let socket = host
        .net_tls_connect("localhost", addr.port() as i64)
        .expect("tls connect");
    host.net_write(socket, "ping").expect("write tls");
    host.net_flush(socket).expect("flush tls");
    assert_eq!(
        host.net_read(socket).expect("read tls"),
        RtString::from("pong")
    );
    host.net_close_handle(socket).expect("close tls socket");
    server.join().expect("join tls server");
}

#[test]
fn noop_host_resolves_hostnames_to_ip_text() {
    let mut host = NoopHost::default();
    assert_eq!(
        host.net_resolve("127.0.0.1").expect("resolve ip literal"),
        RtString::from("127.0.0.1")
    );
}

#[test]
fn noop_host_parses_urls_into_string_maps() {
    let mut host = NoopHost::default();
    let parsed = host
        .net_parse_url("https://example.com:8443/api/v1?q=ok#frag")
        .expect("parse url");

    assert_eq!(
        parsed.get("scheme").expect("scheme"),
        skepart::RtValue::String(RtString::from("https"))
    );
    assert_eq!(
        parsed.get("host").expect("host"),
        skepart::RtValue::String(RtString::from("example.com"))
    );
    assert_eq!(
        parsed.get("port").expect("port"),
        skepart::RtValue::String(RtString::from("8443"))
    );
    assert_eq!(
        parsed.get("path").expect("path"),
        skepart::RtValue::String(RtString::from("/api/v1"))
    );
    assert_eq!(
        parsed.get("query").expect("query"),
        skepart::RtValue::String(RtString::from("q=ok"))
    );
    assert_eq!(
        parsed.get("fragment").expect("fragment"),
        skepart::RtValue::String(RtString::from("frag"))
    );
}

#[test]
fn noop_host_rejects_invalid_urls() {
    let mut host = NoopHost::default();
    assert_eq!(
        host.net_parse_url("not-a-url")
            .expect_err("invalid url should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
}

#[test]
fn noop_host_supports_basic_fetch_over_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind http listener");
    let addr = listener.local_addr().expect("listener addr");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept http client");
        let mut buf = [0_u8; 512];
        let read = stream.read(&mut buf).expect("read request");
        let request = String::from_utf8_lossy(&buf[..read]);
        assert!(request.contains("POST /fetch HTTP/1.0"));
        assert!(request.contains("Content-Type: application/json"));
        assert!(request.ends_with("\r\n\r\n{\"ok\":true}"));
        stream
            .write_all(
                b"HTTP/1.0 201 Created\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"done\":true}\n",
            )
            .expect("write response");
    });

    let mut host = NoopHost::default();
    let options = skepart::RtMap::new();
    options.insert("method", skepart::RtValue::String(RtString::from("POST")));
    options.insert(
        "body",
        skepart::RtValue::String(RtString::from("{\"ok\":true}")),
    );
    options.insert(
        "contentType",
        skepart::RtValue::String(RtString::from("application/json")),
    );
    let response = host
        .net_fetch(&format!("http://{addr}/fetch"), &options)
        .expect("fetch");
    server.join().expect("server thread");

    assert_eq!(
        response.get("status").expect("status"),
        skepart::RtValue::String(RtString::from("201"))
    );
    assert_eq!(
        response.get("contentType").expect("content type"),
        skepart::RtValue::String(RtString::from("application/json"))
    );
}

#[test]
fn noop_host_rejects_tls_connect_with_untrusted_certificate() {
    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("generate cert");
    let cert_der = cert.cert.der().clone();
    let key_der = PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der.into())
        .expect("server config");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind tls listener");
    let addr = listener.local_addr().expect("listener addr");
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept tls client");
        let conn = ServerConnection::new(Arc::new(server_config)).expect("server connection");
        let mut tls = StreamOwned::new(conn, stream);
        let _ = tls.flush();
    });

    let mut host = NoopHost::default();
    assert_eq!(
        host.net_tls_connect("localhost", addr.port() as i64)
            .expect_err("untrusted tls cert should fail")
            .kind,
        skepart::RtErrorKind::Io
    );
    server.join().expect("join tls server");
}

#[test]
fn noop_host_surfaces_invalid_address_and_closed_socket_errors() {
    let mut host = NoopHost::default();
    assert_eq!(
        host.net_connect("not-a-valid-address")
            .expect_err("invalid address should fail")
            .kind,
        skepart::RtErrorKind::Io
    );

    let socket = host
        .net_alloc_handle(RtHandleKind::Socket)
        .expect("allocate placeholder socket");
    host.net_close_handle(socket)
        .expect("close placeholder socket");
    assert_eq!(
        host.net_read(socket)
            .expect_err("closed socket read should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
    assert_eq!(
        host.net_write(socket, "ping")
            .expect_err("closed socket write should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
    assert_eq!(
        host.net_flush(socket)
            .expect_err("closed socket flush should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
    assert_eq!(
        host.net_set_read_timeout(socket, -1)
            .expect_err("negative timeout should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
    assert_eq!(
        host.net_set_write_timeout(socket, 5)
            .expect_err("closed socket timeout should fail")
            .kind,
        skepart::RtErrorKind::InvalidArgument
    );
}

#[test]
fn noop_host_rejects_non_utf8_net_reads() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener addr");
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        stream
            .write_all(&[0xFF, 0xFE, 0xFD])
            .expect("write invalid utf8");
    });

    let mut host = NoopHost::default();
    let client = host
        .net_connect(&addr.to_string())
        .expect("connect client socket");

    let err = host.net_read(client).expect_err("invalid utf8 should fail");
    assert_eq!(err.kind, skepart::RtErrorKind::InvalidArgument);
    assert!(
        err.message.contains("valid UTF-8"),
        "unexpected error: {err:?}"
    );

    peer.join().expect("peer thread should finish");
    host.net_close_handle(client).expect("close client");
}
