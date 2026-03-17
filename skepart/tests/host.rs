mod common;

use common::RecordingHost;
use skepart::{NoopHost, RtHost, RtString};

#[test]
fn noop_host_supports_print_and_time_defaults() {
    let mut host = NoopHost;
    host.io_print("hello").expect("print");
    host.io_println("world").expect("println");
    assert!(host.datetime_now_unix().expect("unix") > 0);
    assert!(host.datetime_now_millis().expect("millis") > 0);
}

#[test]
fn recording_host_captures_output_and_overrides_services() {
    let mut host = RecordingHost::seeded();
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
    assert_eq!(
        host.fs_read_text("file.txt").expect("read"),
        RtString::from("read:file.txt")
    );
}

#[test]
fn recording_host_tracks_fs_os_and_random_side_effects() {
    let mut host = RecordingHost::seeded();
    assert_eq!(host.random_int(1, 9).expect("rand int"), 5);
    assert_eq!(host.random_float().expect("rand float"), 0.25);
    assert!(host.fs_exists("exists.txt").expect("exists"));
    assert_eq!(host.fs_join("a", "b").expect("join"), RtString::from("a/b"));
    assert_eq!(host.os_cwd().expect("cwd"), RtString::from("tmp/work"));
    assert_eq!(host.os_exec_shell("echo hi").expect("shell"), 9);
    assert_eq!(
        host.os_exec_shell_out("echo hi").expect("shell out"),
        RtString::from("shell-out")
    );
    host.fs_write_text("f.txt", "x").expect("write");
    host.fs_append_text("f.txt", "y").expect("append");
    host.fs_mkdir_all("dir").expect("mkdir");
    host.fs_remove_file("f.txt").expect("rm file");
    host.fs_remove_dir_all("dir").expect("rm dir");
    host.os_sleep(12).expect("sleep");
    assert_eq!(
        host.output,
        "[sh echo hi][shout echo hi][write f.txt=x][append f.txt+=y][mkdir dir][rmfile f.txt][rmdir dir][sleep 12]"
    );
}
