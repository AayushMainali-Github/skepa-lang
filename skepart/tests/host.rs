mod common;

use common::RecordingHostBuilder;
use skepart::{NoopHost, RtHandle, RtHandleKind, RtHost, RtString};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

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
