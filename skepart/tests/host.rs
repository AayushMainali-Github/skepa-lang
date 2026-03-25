mod common;

use common::RecordingHostBuilder;
use skepart::{NoopHost, RtHost, RtString};

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
    assert_eq!(host.os_cwd().expect("cwd"), RtString::from("tmp/work"));
    assert_eq!(host.os_exec("hostname").expect("exec"), 9);
    assert_eq!(
        host.os_exec_out("hostname").expect("exec out"),
        RtString::from("shell-out")
    );
    assert_eq!(host.os_exec_shell("echo hi").expect("shell"), 9);
    assert_eq!(
        host.os_exec_shell_out("echo hi").expect("shell out"),
        RtString::from("shell-out")
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
        "[exec hostname][execout hostname][sh echo hi][shout echo hi][envset MODE=debug][envrm MODE][exit 7][write f.txt=x][append f.txt+=y][mkdir dir][rmfile f.txt][rmdir dir][sleep 12]"
    );
}
