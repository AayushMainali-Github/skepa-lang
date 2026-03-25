use skepart::{RtHost, RtResult, RtString};
use skeplib::ir::{
    self, BasicBlock, BlockId, FunctionId, Instr, IrFunction, IrInterpError, IrInterpreter,
    IrProgram, IrType, IrValue, Terminator,
};

#[path = "../../common.rs"]
mod common;

#[derive(Default)]
struct TestHost {
    out: String,
}

impl RtHost for TestHost {
    fn io_print(&mut self, text: &str) -> RtResult<()> {
        self.out.push_str(text);
        Ok(())
    }

    fn datetime_now_unix(&mut self) -> RtResult<i64> {
        Ok(123)
    }

    fn datetime_now_millis(&mut self) -> RtResult<i64> {
        Ok(456_789)
    }

    fn random_seed(&mut self, _seed: i64) -> RtResult<()> {
        Ok(())
    }

    fn random_int(&mut self, min: i64, max: i64) -> RtResult<i64> {
        Ok(min + max)
    }

    fn random_float(&mut self) -> RtResult<f64> {
        Ok(0.25)
    }

    fn fs_exists(&mut self, path: &str) -> RtResult<bool> {
        Ok(path == "exists.txt")
    }

    fn fs_read_text(&mut self, path: &str) -> RtResult<RtString> {
        Ok(RtString::from(format!("read:{path}")))
    }

    fn fs_write_text(&mut self, _path: &str, _text: &str) -> RtResult<()> {
        Ok(())
    }

    fn fs_append_text(&mut self, _path: &str, _text: &str) -> RtResult<()> {
        Ok(())
    }

    fn fs_mkdir_all(&mut self, _path: &str) -> RtResult<()> {
        Ok(())
    }

    fn fs_remove_file(&mut self, _path: &str) -> RtResult<()> {
        Ok(())
    }

    fn fs_remove_dir_all(&mut self, _path: &str) -> RtResult<()> {
        Ok(())
    }

    fn fs_join(&mut self, left: &str, right: &str) -> RtResult<RtString> {
        Ok(RtString::from(format!("{left}/{right}")))
    }

    fn os_platform(&mut self) -> RtResult<RtString> {
        Ok(RtString::from("test-os"))
    }

    fn os_arch(&mut self) -> RtResult<RtString> {
        Ok(RtString::from("test-arch"))
    }

    fn os_arg(&mut self, index: i64) -> RtResult<RtString> {
        match index {
            0 => Ok(RtString::from("prog")),
            1 => Ok(RtString::from("--flag")),
            _ => Err(skepart::RtError::index_out_of_bounds(index as usize, 2)),
        }
    }

    fn os_env_has(&mut self, name: &str) -> RtResult<bool> {
        Ok(name == "HOME")
    }

    fn os_env_get(&mut self, name: &str) -> RtResult<RtString> {
        match name {
            "HOME" => Ok(RtString::from("/tmp/home")),
            _ => Err(skepart::RtError::new(
                skepart::RtErrorKind::InvalidArgument,
                format!("environment variable `{name}` is not set or not valid UTF-8"),
            )),
        }
    }

    fn os_env_set(&mut self, _name: &str, _value: &str) -> RtResult<()> {
        Ok(())
    }

    fn os_env_remove(&mut self, _name: &str) -> RtResult<()> {
        Ok(())
    }

    fn os_sleep(&mut self, _millis: i64) -> RtResult<()> {
        Ok(())
    }

    fn os_exit(&mut self, _code: i64) -> RtResult<()> {
        Ok(())
    }

    fn os_exec(&mut self, program: &str) -> RtResult<i64> {
        Ok(program.len() as i64)
    }

    fn os_exec_out(&mut self, program: &str) -> RtResult<RtString> {
        Ok(RtString::from(format!("exec:{program}")))
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedErrorKind {
    DivisionByZero,
    IndexOutOfBounds,
    TypeMismatch,
}

fn assert_ir_rejects_source(source: &str, expected: ExpectedErrorKind) {
    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let ir_err = IrInterpreter::new(&program)
        .run_main()
        .expect_err("IR interpreter should fail");
    let ir_kind = match ir_err {
        IrInterpError::DivisionByZero => ExpectedErrorKind::DivisionByZero,
        IrInterpError::IndexOutOfBounds => ExpectedErrorKind::IndexOutOfBounds,
        IrInterpError::TypeMismatch(_) => ExpectedErrorKind::TypeMismatch,
        other => panic!("unexpected IR error kind in comparison test: {other:?}"),
    };
    assert_eq!(ir_kind, expected);
}

#[test]
fn interpreter_rejects_non_bool_branch_condition() {
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: Vec::new(),
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                name: "entry".into(),
                instrs: Vec::new(),
                terminator: ir::Terminator::Branch(ir::BranchTerminator {
                    cond: ir::Operand::Const(ir::ConstValue::Int(1)),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                }),
            },
            BasicBlock {
                id: BlockId(1),
                name: "then".into(),
                instrs: Vec::new(),
                terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(1)))),
            },
            BasicBlock {
                id: BlockId(2),
                name: "else".into(),
                instrs: Vec::new(),
                terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
            },
        ],
    };
    let program = IrProgram {
        functions: vec![func],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: None,
    };
    let err = IrInterpreter::new(&program)
        .run_main()
        .expect_err("interpreter should reject non-bool branch conditions");
    assert!(matches!(
        err,
        IrInterpError::TypeMismatch("branch condition must be bool")
    ));
}

#[test]
fn interpreter_rejects_indirect_call_on_non_closure() {
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: Vec::new(),
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![Instr::CallIndirect {
                dst: None,
                ret_ty: IrType::Int,
                callee: ir::Operand::Const(ir::ConstValue::Int(7)),
                args: Vec::new(),
            }],
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
        }],
    };
    let program = IrProgram {
        functions: vec![func],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: None,
    };
    let err = IrInterpreter::new(&program)
        .run_main()
        .expect_err("interpreter should reject non-closure indirect calls");
    assert!(matches!(
        err,
        IrInterpError::TypeMismatch("indirect call on non-closure")
    ));
}

#[test]
fn interpreter_rejects_wrong_arity_direct_and_indirect_calls() {
    let callee = IrFunction {
        id: FunctionId(1),
        name: "step".into(),
        params: vec![skeplib::ir::IrParam {
            id: skeplib::ir::ParamId(0),
            name: "x".into(),
            ty: IrType::Int,
        }],
        locals: vec![skeplib::ir::IrLocal {
            id: skeplib::ir::LocalId(0),
            name: "x".into(),
            ty: IrType::Int,
        }],
        temps: Vec::new(),
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: Vec::new(),
            terminator: Terminator::Return(Some(ir::Operand::Local(ir::LocalId(0)))),
        }],
    };
    let direct = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: vec![skeplib::ir::IrTemp {
            id: ir::TempId(0),
            ty: IrType::Fn {
                params: vec![IrType::Int],
                ret: Box::new(IrType::Int),
            },
        }],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![Instr::CallDirect {
                dst: None,
                ret_ty: IrType::Int,
                function: FunctionId(1),
                args: Vec::new(),
            }],
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
        }],
    };
    let direct_program = IrProgram {
        functions: vec![direct, callee.clone()],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: None,
    };
    let err = IrInterpreter::new(&direct_program)
        .run_main()
        .expect_err("interpreter should reject wrong-arity direct call");
    assert!(matches!(
        err,
        IrInterpError::InvalidOperand("call arity mismatch")
    ));

    let indirect = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: vec![skeplib::ir::IrTemp {
            id: ir::TempId(0),
            ty: IrType::Fn {
                params: vec![IrType::Int],
                ret: Box::new(IrType::Int),
            },
        }],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![
                Instr::MakeClosure {
                    dst: ir::TempId(0),
                    function: FunctionId(1),
                },
                Instr::CallIndirect {
                    dst: None,
                    ret_ty: IrType::Int,
                    callee: ir::Operand::Temp(ir::TempId(0)),
                    args: Vec::new(),
                },
            ],
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
        }],
    };
    let indirect_program = IrProgram {
        functions: vec![indirect, callee],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: None,
    };
    let err = IrInterpreter::new(&indirect_program)
        .run_main()
        .expect_err("interpreter should reject wrong-arity indirect call");
    assert!(matches!(
        err,
        IrInterpError::InvalidOperand("call arity mismatch")
    ));
}

#[test]
fn interpreter_rejects_store_value_type_mismatch() {
    let func = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: vec![skeplib::ir::IrLocal {
            id: skeplib::ir::LocalId(0),
            name: "x".into(),
            ty: IrType::Int,
        }],
        temps: Vec::new(),
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![Instr::StoreLocal {
                local: skeplib::ir::LocalId(0),
                ty: IrType::Int,
                value: ir::Operand::Const(ir::ConstValue::Bool(true)),
            }],
            terminator: Terminator::Return(Some(ir::Operand::Const(ir::ConstValue::Int(0)))),
        }],
    };
    let program = IrProgram {
        functions: vec![func],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: None,
    };
    let err = IrInterpreter::new(&program)
        .run_main()
        .expect_err("interpreter should reject typed store mismatch");
    assert!(matches!(
        err,
        IrInterpError::TypeMismatch("stored value does not match declared type")
    ));
}

#[test]
fn interpreter_initializes_parameter_backed_locals_without_name_matching() {
    let callee = IrFunction {
        id: FunctionId(1),
        name: "id".into(),
        params: vec![skeplib::ir::IrParam {
            id: skeplib::ir::ParamId(0),
            name: "x".into(),
            ty: IrType::Int,
        }],
        locals: vec![skeplib::ir::IrLocal {
            id: skeplib::ir::LocalId(0),
            name: "__arg0".into(),
            ty: IrType::Int,
        }],
        temps: Vec::new(),
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: Vec::new(),
            terminator: Terminator::Return(Some(ir::Operand::Local(ir::LocalId(0)))),
        }],
    };
    let main = IrFunction {
        id: FunctionId(0),
        name: "main".into(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: vec![skeplib::ir::IrTemp {
            id: ir::TempId(0),
            ty: IrType::Int,
        }],
        ret_ty: IrType::Int,
        entry: BlockId(0),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            name: "entry".into(),
            instrs: vec![Instr::CallDirect {
                dst: Some(ir::TempId(0)),
                ret_ty: IrType::Int,
                function: FunctionId(1),
                args: vec![ir::Operand::Const(ir::ConstValue::Int(9))],
            }],
            terminator: Terminator::Return(Some(ir::Operand::Temp(ir::TempId(0)))),
        }],
    };
    let program = IrProgram {
        functions: vec![main, callee],
        globals: Vec::new(),
        structs: Vec::new(),
        module_init: None,
    };
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("interpreter should seed parameter locals by position");
    assert_eq!(value, IrValue::Int(9));
}

#[test]
fn interpreter_handles_runtime_managed_values_and_function_values() {
    let source = r#"
struct Pair {
  a: Int,
  b: Int
}

impl Pair {
  fn mix(self, x: Int) -> Int {
    return self.a + self.b + x;
  }
}

fn add1(x: Int) -> Int {
  return x + 1;
}

fn main() -> Int {
  let arr: [Int; 2] = [1; 2];
  let xs: Vec[Int] = vec.new();
  let p = Pair { a: arr[0], b: 3 };
  let f: Fn(Int) -> Int = add1;
  vec.push(xs, p.mix(4));
  return f(vec.get(xs, 0));
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(9));
}

#[test]
fn interpreter_handles_nested_runtime_managed_values() {
    let source = r#"
struct Boxed {
  items: Vec[Int]
}

fn main() -> Int {
  let outer: [Int; 2] = [4; 2];
  let xs: Vec[Int] = vec.new();
  vec.push(xs, outer[0]);
  vec.push(xs, outer[1] + 3);
  let boxed = Boxed { items: xs };
  return vec.get(boxed.items, 0) + vec.get(boxed.items, 1);
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(11));
}

#[test]
fn interpreter_preserves_shared_aliasing_for_vecs_and_struct_handles() {
    let source = r#"
fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  let ys = xs;
  vec.push(xs, 3);
  vec.set(ys, 0, 9);
  return vec.get(xs, 0);
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(9));
}

#[test]
fn interpreter_supports_closure_calls_across_locals() {
    let source = r#"
fn add2(x: Int) -> Int {
  return x + 2;
}

fn main() -> Int {
  let f: Fn(Int) -> Int = add2;
  let g: Fn(Int) -> Int = f;
  return g(5);
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(7));
}

#[test]
fn interpreter_runs_globals_module_init_and_core_builtins() {
    let source = r#"
import datetime;
import str;

let base: String = "skepa-language-runtime";

fn main() -> Int {
  let total = str.len(base) + str.indexOf(base, "time");
  let cut = str.slice(base, 6, 14);
  if (str.contains(cut, "language")) {
    return total + 1;
  }
  return datetime.nowMillis();
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(40));
}

#[test]
fn interpreter_respects_project_module_init_ordering() {
    let source = r#"
let seed: Int = 4;
let answer: Int = seed + 3;

fn main() -> Int {
  return answer;
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(7));
}

#[test]
fn interpreter_supports_io_and_datetime_builtins_through_runtime() {
    let source = r#"
import datetime;
import io;

fn main() -> Int {
  io.printInt(7);
  let now = datetime.nowUnix();
  if (now >= 0) {
    return 1;
  }
  return 0;
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(1));
}

#[test]
fn interpreter_builtin_matrix_covers_arr_vec_io_datetime() {
    let source = r#"
import arr;
import datetime;
import io;

fn main() -> Int {
  let xs: [Int; 3] = [5; 3];
  let total = arr.len(xs);
  let empty = arr.isEmpty(xs);
  io.println("ok");
  if (empty) {
    return 0;
  }
  return total + datetime.nowUnix();
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::with_host(&program, Box::new(TestHost::default()))
        .run_main()
        .expect("IR interpreter should run source");
    assert_eq!(value, IrValue::Int(126));
}

#[test]
fn interpreter_builtin_matrix_covers_random_fs_and_os_with_deterministic_host() {
    let source = r#"
import fs;
import os;
import random;
import str;

fn main() -> Int {
  random.seed(9);
  let total = random.int(2, 5);
  let plat = os.platform();
  let out = os.execOut("git");
  if (fs.exists("exists.txt")) {
    return total + str.len(plat) + str.len(out);
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::with_host(&program, Box::new(TestHost::default()))
        .run_main()
        .expect("IR interpreter should run source");
    assert_eq!(value, IrValue::Int(22));
}

#[test]
fn interpreter_builtin_matrix_covers_new_os_host_helpers() {
    let source = r#"
import os;
import str;

fn main() -> Int {
  let arch = os.arch();
  let arg0 = os.arg(0);
  let has = os.envHas("HOME");
  let home = os.envGet("HOME");
  os.envSet("MODE", "debug");
  os.envRemove("MODE");
  os.sleep(1);
  os.exit(0);
  let code = os.exec("git");
  let out = os.execOut("git");
  if (has && str.len(arch) > 0 && str.len(arg0) > 0 && str.len(home) > 0 && code > 0 && str.len(out) > 0) {
    return 1;
  }
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::with_host(&program, Box::new(TestHost::default()))
        .run_main()
        .expect("IR interpreter should run source");
    assert_eq!(value, IrValue::Int(1));
}

#[test]
fn interpreter_builtin_matrix_covers_more_edge_results() {
    let source = r#"
import arr;
import fs;
import io;
import os;
import random;
import str;

fn main() -> Int {
  let parts: [String; 2] = ["ab"; 2];
  let joined = arr.join(parts, "-");
  let text = fs.readText("alpha.txt");
  let path = fs.join("root", "leaf");
  let out = io.format("v=%d %b", 12, true);
  let code = os.exec("git");
  let bonus = random.int(1, 2);
  return str.len(joined) + str.len(text) + str.len(path) + str.len(out) + code + bonus;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::with_host(&program, Box::new(TestHost::default()))
        .run_main()
        .expect("IR interpreter should run source");
    assert_eq!(value, IrValue::Int(47));
}

#[test]
fn interpreter_supports_float_and_string_compare_shapes() {
    let float_src = r#"
fn main() -> Int {
  let x = 1.5;
  let y = 2.0;
  if ((x + y) >= 3.5) {
    return 1;
  }
  return 0;
}
"#;
    let string_src = r#"
fn main() -> Int {
  let a = "alpha";
  let b = "alpha";
  if (a == b) {
    return 1;
  }
  return 0;
}
"#;

    assert_eq!(common::ir_run_ok(float_src), IrValue::Int(1));
    assert_eq!(common::ir_run_ok(string_src), IrValue::Int(1));
}

#[test]
fn interpreter_supports_global_float_and_string_compare_shapes() {
    let float_src = r#"
let threshold: Float = 3.5;

fn main() -> Int {
  let value = 1.5 + 2.0;
  if (value >= threshold) {
    return 1;
  }
  return 0;
}
"#;
    let string_src = r#"
let expected: String = "alpha";

fn main() -> Int {
  let actual = "alpha";
  let other = "beta";
  if (actual == expected && actual != other) {
    return 1;
  }
  return 0;
}
"#;

    assert_eq!(common::ir_run_ok(float_src), IrValue::Int(1));
    assert_eq!(common::ir_run_ok(string_src), IrValue::Int(1));
}

#[test]
fn interpreter_supports_bitwise_integer_operators() {
    let source = r#"
fn main() -> Int {
  let a = 12;
  let b = 10;
  let c = ~a;
  let d = a & b;
  let e = a | b;
  let f = a ^ b;
  let g = a << 2;
  let h = a >> 1;
  if (c == -13 && d == 8 && e == 14 && f == 6 && g == 48 && h == 6) {
    return 1;
  }
  return 0;
}
"#;

    assert_eq!(common::ir_run_ok(source), IrValue::Int(1));
}

#[test]
fn interpreter_reports_runtime_error_cases() {
    assert_ir_rejects_source(
        r#"
fn main() -> Int {
  return 8 / 0;
}
"#,
        ExpectedErrorKind::DivisionByZero,
    );
    assert_ir_rejects_source(
        r#"
fn main() -> Int {
  let arr: [Int; 2] = [1; 2];
  return arr[3];
}
"#,
        ExpectedErrorKind::IndexOutOfBounds,
    );
    assert_ir_rejects_source(
        r#"
import str;

fn main() -> String {
  return str.slice("abc", 0, 99);
}
"#,
        ExpectedErrorKind::IndexOutOfBounds,
    );
    assert_ir_rejects_source(
        r#"
import arr;

fn main() -> Int {
  let xs: [Int; 0] = [];
  return arr.first(xs);
}
"#,
        ExpectedErrorKind::IndexOutOfBounds,
    );
    assert_ir_rejects_source(
        r#"
import vec;

fn main() -> Int {
  let xs: Vec[Int] = vec.new();
  return vec.get(xs, 0);
}
"#,
        ExpectedErrorKind::IndexOutOfBounds,
    );
}
