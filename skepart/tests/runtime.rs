use skepart::{
    builtins, NoopHost, RtArray, RtErrorKind, RtFunctionRegistry, RtHost, RtString, RtStruct,
    RtStructLayout, RtValue, RtVec,
};
use std::rc::Rc;

#[test]
fn arrays_use_copy_on_write_value_storage() {
    let mut first = RtArray::repeat(RtValue::Int(1), 3);
    let second = first.clone();

    first
        .set(1, RtValue::Int(9))
        .expect("array write should succeed");

    assert_eq!(first.get(1), Ok(RtValue::Int(9)));
    assert_eq!(second.get(1), Ok(RtValue::Int(1)));
}

#[test]
fn vecs_use_shared_handle_semantics() {
    let first = RtVec::new();
    let second = first.clone();

    first.push(RtValue::Int(7));

    assert_eq!(second.borrow().as_slice(), &[RtValue::Int(7)]);
}

#[test]
fn strings_track_character_length() {
    let value = RtString::from("naive");
    assert_eq!(value.len_chars(), 5);
}

#[test]
fn arrays_report_bounds_errors() {
    let array = RtArray::repeat(RtValue::Int(0), 2);
    let err = array.get(3).expect_err("index should be rejected");
    assert_eq!(err.kind, RtErrorKind::IndexOutOfBounds);
}

#[test]
fn vec_helpers_support_get_set_and_delete() {
    let vec = RtVec::new();
    vec.push(RtValue::Int(1));
    vec.push(RtValue::Int(2));
    vec.set(1, RtValue::Int(9)).expect("set should work");

    assert_eq!(vec.get(1), Ok(RtValue::Int(9)));
    assert_eq!(vec.delete(0), Ok(RtValue::Int(1)));
    assert_eq!(vec.get(0), Ok(RtValue::Int(9)));
}

#[test]
fn string_builtins_match_current_runtime_shape() {
    let value = RtString::from("skepa-language-benchmark");
    let needle = RtString::from("bench");

    assert_eq!(skepart::str_builtin::len(&value), 24);
    assert_eq!(skepart::str_builtin::index_of(&value, &needle), 15);
    assert!(skepart::str_builtin::contains(
        &RtString::from("language"),
        &RtString::from("gua")
    ));
    assert_eq!(
        skepart::str_builtin::slice(&value, 6, 18).expect("slice should work"),
        RtString::from("language-ben")
    );
}

#[test]
fn generic_builtin_dispatch_handles_core_runtime_helpers() {
    let array = RtArray::new(vec![
        RtValue::String(RtString::from("a")),
        RtValue::String(RtString::from("b")),
    ]);
    let vec = RtVec::new();

    assert_eq!(
        builtins::call(
            "arr",
            "join",
            &[RtValue::Array(array), RtValue::String(RtString::from("-"))]
        )
        .expect("arr.join should succeed"),
        RtValue::String(RtString::from("a-b"))
    );

    assert_eq!(
        builtins::call("vec", "new", &[])
            .expect("vec.new should succeed")
            .type_name(),
        "Vec"
    );

    builtins::call("vec", "push", &[RtValue::Vec(vec.clone()), RtValue::Int(4)])
        .expect("vec.push should succeed");
    assert_eq!(
        builtins::call("vec", "get", &[RtValue::Vec(vec), RtValue::Int(0)])
            .expect("vec.get should succeed"),
        RtValue::Int(4)
    );
}

#[test]
fn values_and_structs_expose_runtime_checked_accessors() {
    let value = RtValue::Struct(RtStruct {
        layout: Rc::new(RtStructLayout {
            name: "Pair".into(),
            field_names: vec!["a".into(), "b".into()],
        }),
        fields: vec![RtValue::Int(1), RtValue::Int(2)],
    });
    let mut strukt = value.expect_struct().expect("struct should match");

    assert_eq!(strukt.get_field(1), Ok(RtValue::Int(2)));
    assert_eq!(strukt.get_named_field("b"), Ok(RtValue::Int(2)));
    strukt
        .set_field(0, RtValue::Int(9))
        .expect("field write should work");
    assert_eq!(strukt.get_field(0), Ok(RtValue::Int(9)));
    assert_eq!(
        RtValue::Bool(true).expect_int().unwrap_err().kind,
        RtErrorKind::TypeMismatch
    );
}

#[test]
fn host_trait_is_callable_from_runtime_clients() {
    let mut host = NoopHost;
    host.io_print("hello").expect("print should succeed");
    host.io_println("world").expect("println should succeed");
}

#[derive(Default)]
struct RecordingHost {
    output: String,
}

impl RtHost for RecordingHost {
    fn io_print(&mut self, text: &str) -> skepart::RtResult<()> {
        self.output.push_str(text);
        Ok(())
    }

    fn datetime_now_unix(&mut self) -> skepart::RtResult<i64> {
        Ok(100)
    }

    fn random_int(&mut self, min: i64, max: i64) -> skepart::RtResult<i64> {
        Ok((min + max) / 2)
    }

    fn fs_join(&mut self, left: &str, right: &str) -> skepart::RtResult<RtString> {
        Ok(RtString::from(format!("{left}/{right}")))
    }

    fn os_platform(&mut self) -> skepart::RtResult<RtString> {
        Ok(RtString::from("test-os"))
    }
}

#[test]
fn io_builtins_dispatch_through_host() {
    let mut host = RecordingHost::default();
    builtins::call_with_host(&mut host, "io", "print", &[RtValue::Int(7)])
        .expect("io.print should succeed");
    builtins::call_with_host(
        &mut host,
        "io",
        "println",
        &[RtValue::String(RtString::from("done"))],
    )
    .expect("io.println should succeed");

    assert_eq!(host.output, "7done\n");
}

#[test]
fn host_backed_builtins_dispatch_for_runtime_services() {
    let mut host = RecordingHost::default();

    assert_eq!(
        builtins::call_with_host(&mut host, "datetime", "nowUnix", &[])
            .expect("datetime.nowUnix should succeed"),
        RtValue::Int(100)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "random",
            "int",
            &[RtValue::Int(2), RtValue::Int(8)]
        )
        .expect("random.int should succeed"),
        RtValue::Int(5)
    );
    assert_eq!(
        builtins::call_with_host(
            &mut host,
            "fs",
            "join",
            &[
                RtValue::String(RtString::from("tmp")),
                RtValue::String(RtString::from("file.txt"))
            ]
        )
        .expect("fs.join should succeed"),
        RtValue::String(RtString::from("tmp/file.txt"))
    );
    assert_eq!(
        builtins::call_with_host(&mut host, "os", "platform", &[])
            .expect("os.platform should succeed"),
        RtValue::String(RtString::from("test-os"))
    );
}

#[test]
fn io_format_and_printf_are_available_in_runtime() {
    let mut host = RecordingHost::default();
    assert_eq!(
        builtins::call(
            "io",
            "format",
            &[
                RtValue::String(RtString::from("%d %s")),
                RtValue::Int(3),
                RtValue::String(RtString::from("cats"))
            ]
        )
        .expect("io.format should succeed"),
        RtValue::String(RtString::from("3 cats"))
    );

    builtins::call_with_host(
        &mut host,
        "io",
        "printf",
        &[RtValue::String(RtString::from("%b")), RtValue::Bool(true)],
    )
    .expect("io.printf should succeed");
    assert_eq!(host.output, "true");
}

#[test]
fn runtime_function_registry_provides_native_call_surface() {
    fn add_one(_host: &mut dyn RtHost, args: &[RtValue]) -> skepart::RtResult<RtValue> {
        Ok(RtValue::Int(args[0].expect_int()? + 1))
    }

    let mut registry = RtFunctionRegistry::new();
    let func = registry.register(add_one);
    let mut host = NoopHost;

    assert_eq!(
        registry
            .call(&mut host, func, &[RtValue::Int(4)])
            .expect("registry call should succeed"),
        RtValue::Int(5)
    );
}
