mod common;

use common::RecordingHostBuilder;
use skepart::{RtErrorKind, RtFunctionRef, RtFunctionRegistry, RtValue};

fn add_one(_host: &mut dyn skepart::RtHost, args: &[RtValue]) -> skepart::RtResult<RtValue> {
    Ok(RtValue::Int(args[0].expect_int()? + 1))
}

fn sum_two(_host: &mut dyn skepart::RtHost, args: &[RtValue]) -> skepart::RtResult<RtValue> {
    Ok(RtValue::Int(args[0].expect_int()? + args[1].expect_int()?))
}

#[test]
fn function_registry_calls_registered_functions() {
    let mut registry = RtFunctionRegistry::new();
    let f = registry.register(add_one);
    let mut host = RecordingHostBuilder::seeded().build();
    assert_eq!(
        registry
            .call(&mut host, f, &[RtValue::Int(4)])
            .expect("call"),
        RtValue::Int(5)
    );
}

#[test]
fn function_registry_reports_missing_function_id() {
    let registry = RtFunctionRegistry::new();
    let mut host = RecordingHostBuilder::seeded().build();
    assert_eq!(
        registry
            .call(&mut host, RtFunctionRef(99), &[RtValue::Int(1)])
            .expect_err("missing id")
            .kind,
        RtErrorKind::UnsupportedBuiltin
    );
}

#[test]
fn function_registry_preserves_argument_type_checks_inside_function() {
    let mut registry = RtFunctionRegistry::new();
    let f = registry.register(sum_two);
    let mut host = RecordingHostBuilder::seeded().build();
    assert_eq!(
        registry
            .call(&mut host, f, &[RtValue::Int(1), RtValue::Bool(true)])
            .expect_err("type mismatch")
            .kind,
        RtErrorKind::TypeMismatch
    );
}

#[test]
fn function_registry_assigns_stable_incrementing_ids() {
    let mut registry = RtFunctionRegistry::new();
    let first = registry.register(add_one);
    let second = registry.register(sum_two);
    assert_eq!(first, RtFunctionRef(0));
    assert_eq!(second, RtFunctionRef(1));
}

#[test]
fn function_registry_can_store_and_call_many_functions() {
    let mut registry = RtFunctionRegistry::new();
    let mut ids = Vec::new();
    for _ in 0..16 {
        ids.push(registry.register(add_one));
    }

    let mut host = RecordingHostBuilder::seeded().build();
    assert_eq!(
        registry
            .call(&mut host, ids[15], &[RtValue::Int(41)])
            .expect("high id call"),
        RtValue::Int(42)
    );
}
