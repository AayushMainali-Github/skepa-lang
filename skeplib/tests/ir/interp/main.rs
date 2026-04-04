use std::sync::{Arc, Mutex};

use skepart::{RtBytes, RtHandleKind, RtHost, RtResult, RtString};
use skeplib::ir::{
    self, BasicBlock, BlockId, FunctionId, Instr, IrFunction, IrInterpError, IrInterpreter,
    IrProgram, IrType, IrValue, Terminator,
};

#[path = "../../common.rs"]
mod common;

#[derive(Default)]
struct TestHost {
    out: Arc<Mutex<String>>,
    next_handle_id: usize,
}

impl RtHost for TestHost {
    fn io_print(&mut self, text: &str) -> RtResult<()> {
        self.out.lock().expect("lock trace").push_str(text);
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

    fn ffi_open_library(&mut self, path: &str) -> RtResult<skepart::RtHandle> {
        let handle = self.net_alloc_handle(RtHandleKind::Library)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[ffiopen {path}={}]",
                handle.id
            ));
        Ok(handle)
    }

    fn ffi_bind_symbol(
        &mut self,
        library: skepart::RtHandle,
        symbol: &str,
    ) -> RtResult<skepart::RtHandle> {
        self.net_lookup_handle_kind(library)?;
        let handle = self.net_alloc_handle(RtHandleKind::Symbol)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[ffibind {}:{symbol}={}]", library.id, handle.id));
        Ok(handle)
    }

    fn ffi_call_0_int(&mut self, symbol: skepart::RtHandle) -> RtResult<i64> {
        self.net_lookup_handle_kind(symbol)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[fficall0int {}]", symbol.id));
        Ok(11)
    }

    fn ffi_call_1_int(&mut self, symbol: skepart::RtHandle, value: i64) -> RtResult<i64> {
        self.net_lookup_handle_kind(symbol)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[fficall1int {}={}]", symbol.id, value));
        Ok(value + 5)
    }

    fn ffi_call_1_int_void(&mut self, symbol: skepart::RtHandle, value: i64) -> RtResult<()> {
        self.net_lookup_handle_kind(symbol)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[fficall1intvoid {}={}]", symbol.id, value));
        Ok(())
    }

    fn ffi_call_1_string_int(&mut self, symbol: skepart::RtHandle, value: &str) -> RtResult<i64> {
        self.net_lookup_handle_kind(symbol)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[fficall1stringint {}={value}]", symbol.id));
        Ok(value.len() as i64)
    }

    fn ffi_call_1_string_void(
        &mut self,
        symbol: skepart::RtHandle,
        value: &str,
    ) -> RtResult<()> {
        self.net_lookup_handle_kind(symbol)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[fficall1stringvoid {}={value}]", symbol.id));
        Ok(())
    }

    fn ffi_call_1_bytes_int(
        &mut self,
        symbol: skepart::RtHandle,
        value: &RtBytes,
    ) -> RtResult<i64> {
        self.net_lookup_handle_kind(symbol)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[fficall1bytesint {} len={}]", symbol.id, value.len()));
        Ok(value.len() as i64)
    }

    fn ffi_call_2_string_int_int(
        &mut self,
        symbol: skepart::RtHandle,
        left: &str,
        right: i64,
    ) -> RtResult<i64> {
        self.net_lookup_handle_kind(symbol)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!(
                "[fficall2stringintint {}={left}|{right}]",
                symbol.id
            ));
        Ok(left.len() as i64 + right)
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

    fn os_exec(&mut self, program: &str, args: &[String]) -> RtResult<i64> {
        Ok((program.len() + args.iter().map(|arg| arg.len()).sum::<usize>()) as i64)
    }

    fn os_exec_out(&mut self, program: &str, args: &[String]) -> RtResult<RtString> {
        let suffix = if args.is_empty() {
            String::new()
        } else {
            format!(":{}", args.join(","))
        };
        Ok(RtString::from(format!("exec:{program}{suffix}")))
    }

    fn net_listen(&mut self, _address: &str) -> RtResult<skepart::RtHandle> {
        let handle = self.net_alloc_handle(RtHandleKind::Listener)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[listen {}]", handle.id));
        Ok(handle)
    }

    fn net_connect(&mut self, _address: &str) -> RtResult<skepart::RtHandle> {
        let handle = self.net_alloc_handle(RtHandleKind::Socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[connect {}]", handle.id));
        Ok(handle)
    }

    fn net_tls_connect(&mut self, host: &str, port: i64) -> RtResult<skepart::RtHandle> {
        let handle = self.net_alloc_handle(RtHandleKind::Socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[tlsconnect {}={host}:{port}]", handle.id));
        Ok(handle)
    }

    fn net_resolve(&mut self, host: &str) -> RtResult<RtString> {
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[resolve {host}]"));
        Ok(RtString::from("127.0.0.1"))
    }

    fn net_parse_url(&mut self, url: &str) -> RtResult<skepart::RtMap> {
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[parseurl {url}]"));
        let map = skepart::RtMap::new();
        map.insert("scheme", skepart::RtValue::String(RtString::from("https")));
        map.insert("host", skepart::RtValue::String(RtString::from("example.com")));
        map.insert("port", skepart::RtValue::String(RtString::from("443")));
        map.insert("path", skepart::RtValue::String(RtString::from("/a")));
        map.insert("query", skepart::RtValue::String(RtString::from("x=1")));
        map.insert("fragment", skepart::RtValue::String(RtString::from("frag")));
        Ok(map)
    }

    fn net_fetch(&mut self, url: &str, options: &skepart::RtMap) -> RtResult<skepart::RtMap> {
        let method = options
            .get("method")
            .ok()
            .and_then(|value| value.expect_string().ok())
            .map(|value| value.as_str().to_owned())
            .unwrap_or_else(|| "GET".to_string());
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[fetch {url} method={method}]"));
        let map = skepart::RtMap::new();
        map.insert("status", skepart::RtValue::String(RtString::from("201")));
        map.insert("body", skepart::RtValue::String(RtString::from("fetch-body")));
        map.insert(
            "contentType",
            skepart::RtValue::String(RtString::from("application/json")),
        );
        Ok(map)
    }

    fn net_accept(&mut self, listener: skepart::RtHandle) -> RtResult<skepart::RtHandle> {
        self.net_lookup_handle_kind(listener)?;
        let handle = self.net_alloc_handle(RtHandleKind::Socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[accept {}->{}]", listener.id, handle.id));
        Ok(handle)
    }

    fn net_read(&mut self, socket: skepart::RtHandle) -> RtResult<RtString> {
        self.net_lookup_handle_kind(socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[read {}]", socket.id));
        Ok(RtString::from("net-read"))
    }

    fn net_write(&mut self, socket: skepart::RtHandle, data: &str) -> RtResult<()> {
        self.net_lookup_handle_kind(socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[write {}={data}]", socket.id));
        Ok(())
    }

    fn net_read_bytes(&mut self, socket: skepart::RtHandle) -> RtResult<RtBytes> {
        self.net_lookup_handle_kind(socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[readbytes {}]", socket.id));
        Ok(RtBytes::from(vec![1_u8, 2, 3]))
    }

    fn net_write_bytes(&mut self, socket: skepart::RtHandle, data: &RtBytes) -> RtResult<()> {
        self.net_lookup_handle_kind(socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[writebytes {} len={}]", socket.id, data.len()));
        Ok(())
    }

    fn net_local_addr(&mut self, socket: skepart::RtHandle) -> RtResult<RtString> {
        self.net_lookup_handle_kind(socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[localaddr {}]", socket.id));
        Ok(RtString::from("127.0.0.1:3000"))
    }

    fn net_peer_addr(&mut self, socket: skepart::RtHandle) -> RtResult<RtString> {
        self.net_lookup_handle_kind(socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[peeraddr {}]", socket.id));
        Ok(RtString::from("127.0.0.1:4000"))
    }

    fn net_read_n(&mut self, socket: skepart::RtHandle, count: i64) -> RtResult<RtBytes> {
        self.net_lookup_handle_kind(socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[readn {} count={}]", socket.id, count));
        Ok(RtBytes::from(vec![9_u8, 8, 7]))
    }

    fn net_flush(&mut self, socket: skepart::RtHandle) -> RtResult<()> {
        self.net_lookup_handle_kind(socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[flush {}]", socket.id));
        Ok(())
    }

    fn net_set_read_timeout(&mut self, socket: skepart::RtHandle, millis: i64) -> RtResult<()> {
        self.net_lookup_handle_kind(socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[setreadtimeout {}={}]", socket.id, millis));
        Ok(())
    }

    fn net_set_write_timeout(&mut self, socket: skepart::RtHandle, millis: i64) -> RtResult<()> {
        self.net_lookup_handle_kind(socket)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[setwritetimeout {}={}]", socket.id, millis));
        Ok(())
    }

    fn net_close_handle(&mut self, handle: skepart::RtHandle) -> RtResult<()> {
        self.net_lookup_handle_kind(handle)?;
        self.out
            .lock()
            .expect("lock trace")
            .push_str(&format!("[close {}]", handle.id));
        Ok(())
    }

    fn net_alloc_handle(&mut self, kind: RtHandleKind) -> RtResult<skepart::RtHandle> {
        let handle = skepart::RtHandle {
            id: self.next_handle_id,
            kind,
        };
        self.next_handle_id += 1;
        Ok(handle)
    }

    fn net_lookup_handle_kind(
        &mut self,
        handle: skepart::RtHandle,
    ) -> RtResult<skepart::RtHandleKind> {
        Ok(handle.kind)
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
fn interpreter_supports_bytes_builtins_through_runtime() {
    let source = r#"
import bytes;

fn main() -> Int {
  let raw: Bytes = bytes.fromString("net");
  let text: String = bytes.toString(raw);
  let mid: Bytes = bytes.slice(raw, 1, 3);
  let joined: Bytes = bytes.concat(mid, bytes.fromString("t"));
  let grown: Bytes = bytes.push(joined, 33);
  let same: Bool = bytes.append(mid, bytes.fromString("t")) == joined;
  if (text == "net" && bytes.get(raw, 0) == 110 && bytes.toString(grown) == "ett!" && same) {
    return bytes.len(raw);
  }
  return 0;
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(3));
}

#[test]
fn interpreter_supports_option_values_and_equality() {
    let source = r#"
fn wrap(x: Int) -> Option[Int] {
  return Some(x);
}

#[test]
fn interpreter_supports_result_values_and_equality() {
    let source = r#"
fn wrap(x: Int) -> Result[Int, String] {
  return Ok(x);
}

fn fail() -> Result[Int, String] {
  return Err("bad");
}

fn main() -> Int {
  let a: Result[Int, String] = wrap(7);
  let b: Result[Int, String] = Ok(7);
  let c: Result[Int, String] = fail();
  let d: Result[Int, String] = Err("bad");
  if (a == b && c == d && a != c) {
    return 0;
  }
  return 1;
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(0));
}

fn missing() -> Option[Int] {
  return None();
}

fn main() -> Int {
  let a: Option[Int] = wrap(7);
  let b: Option[Int] = Some(7);
  let c: Option[Int] = missing();
  if (a == b && a != c && c == None()) {
    return 0;
  }
  return 1;
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(0));
}

#[test]
fn interpreter_supports_map_builtins_through_runtime() {
    let source = r#"
import map;

fn main() -> Int {
  let headers: Map[String, Int] = map.new();
  map.insert(headers, "content-length", 12);
  let same = headers;
  let value = map.get(same, "content-length");
  let removed = map.remove(headers, "content-length");
  if (map.has(same, "content-length") || map.len(headers) != 0) {
    return 0;
  }
  return value + removed;
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(24));
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
import vec;

fn main() -> Int {
  random.seed(9);
  let total = random.int(2, 5);
  let plat = os.platform();
  let args: Vec[String] = vec.new();
  vec.push(args, "status");
  let out = os.execOut("git", args);
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
import vec;

fn main() -> Int {
  let arch = os.arch();
  let arg0 = os.arg(0);
  let has = os.envHas("HOME");
  let home = os.envGet("HOME");
  os.envSet("MODE", "debug");
  os.envRemove("MODE");
  os.sleep(1);
  os.exit(0);
  let args: Vec[String] = vec.new();
  vec.push(args, "status");
  let code = os.exec("git", args);
  let out = os.execOut("git", args);
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
import vec;

fn main() -> Int {
  let parts: [String; 2] = ["ab"; 2];
  let joined = arr.join(parts, "-");
  let text = fs.readText("alpha.txt");
  let path = fs.join("root", "leaf");
  let out = io.format("v=%d %b", 12, true);
  let args: Vec[String] = vec.new();
  vec.push(args, "status");
  let code = os.exec("git", args);
  let bonus = random.int(1, 2);
  return str.len(joined) + str.len(text) + str.len(path) + str.len(out) + code + bonus;
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let value = IrInterpreter::with_host(&program, Box::new(TestHost::default()))
        .run_main()
        .expect("IR interpreter should run source");
    assert_eq!(value, IrValue::Int(47));
}

#[test]
fn interpreter_carries_dummy_net_handle_values_through_calls() {
    let source = r#"
import net;

fn make() -> net.Socket {
  return net.__testSocket();
}

fn main() -> Int {
  let s: net.Socket = make();
  let t: net.Socket = s;
  return 0;
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(0));
}

#[test]
fn interpreter_carries_real_net_builtin_handles_through_calls() {
    let source = r#"
import net;
import bytes;
import map;

fn main() -> Int {
  let parts: Map[String, String] = net.parseUrl("https://example.com:443/a?x=1#frag");
  let fetchOptions: Map[String, String] = map.new();
  map.insert(fetchOptions, "method", "POST");
  map.insert(fetchOptions, "body", "{}");
  map.insert(fetchOptions, "contentType", "application/json");
  let response: Map[String, String] = net.fetch("https://example.com/api", fetchOptions);
  let listener: net.Listener = net.listen("127.0.0.1:0");
  let server: net.Socket = net.accept(listener);
  let client: net.Socket = net.connect("127.0.0.1:8080");
  let secure: net.Socket = net.tlsConnect("example.com", 443);
  let resolved: String = net.resolve("localhost");
  let msg = net.read(server);
  let host = map.get(parts, "host");
  let status = map.get(response, "status");
  let raw: Bytes = net.readBytes(server);
  let exact: Bytes = net.readN(server, 3);
  let local = net.localAddr(client);
  let peer = net.peerAddr(secure);
  net.write(client, msg);
  net.writeBytes(client, raw);
  net.writeBytes(client, exact);
  net.flush(client);
  net.setReadTimeout(client, 25);
  net.setWriteTimeout(client, 50);
  net.close(server);
  net.close(client);
  net.closeListener(listener);
  if ((local != peer) && (resolved != "") && (host == "example.com") && (status == "201")) {
    return 0;
  }
  return 1;
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let trace = Arc::new(Mutex::new(String::new()));
    let host = TestHost {
        out: Arc::clone(&trace),
        next_handle_id: 0,
    };
    let value = IrInterpreter::with_host(&program, Box::new(host))
        .run_main()
        .expect("IR interpreter should run source with net handles");
    assert_eq!(value, IrValue::Int(0));
    assert_eq!(
        trace.lock().expect("lock trace").as_str(),
        "[listen 0][accept 0->1][connect 2][tlsconnect 3=example.com:443][read 1][readbytes 1][readn 1 count=3][localaddr 2][peeraddr 3][write 2=net-read][writebytes 2 len=3][writebytes 2 len=3][flush 2][setreadtimeout 2=25][setwritetimeout 2=50][close 1][close 2][close 0]"
    );
}

#[test]
fn interpreter_carries_ffi_library_and_symbol_handles() {
    let source = r#"
import ffi;

fn main() -> Int {
  let lib: ffi.Library = ffi.open("test-lib");
  let sym: ffi.Symbol = ffi.bind(lib, "puts");
  ffi.closeSymbol(sym);
  ffi.closeLibrary(lib);
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let trace = Arc::new(Mutex::new(String::new()));
    let host = TestHost {
        out: Arc::clone(&trace),
        next_handle_id: 0,
    };
    let interp = IrInterpreter::new(program, host);
    let result = interp.run_main();
    assert_eq!(result.expect("program should run"), IrValue::Int(0));
    let trace = trace.lock().expect("lock trace").clone();
    assert!(trace.contains("[ffiopen test-lib=0]"), "trace was: {trace}");
    assert!(trace.contains("[ffibind 0:puts=1]"), "trace was: {trace}");
}

#[test]
fn interpreter_lowers_linked_extern_calls_through_ffi_runtime() {
    let source = r#"
extern("test-lib") fn strlen(s: String) -> Int;
extern("test-lib") fn plus(seed: Int) -> Int;

fn main() -> Int {
  return strlen("hello") + plus(7);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let trace = Arc::new(Mutex::new(String::new()));
    let host = TestHost {
        out: Arc::clone(&trace),
        next_handle_id: 0,
    };
    let interp = IrInterpreter::new(program, host);
    let result = interp.run_main();
    assert_eq!(result.expect("program should run"), IrValue::Int(17));
    let trace = trace.lock().expect("lock trace").clone();
    assert!(trace.contains("[ffiopen test-lib=0]"), "trace was: {trace}");
    assert!(trace.contains("[ffibind 0:strlen=1]"), "trace was: {trace}");
    assert!(trace.contains("[fficall1stringint 1=hello]"), "trace was: {trace}");
    assert!(trace.contains("[ffibind 0:plus=2]"), "trace was: {trace}");
    assert!(trace.contains("[fficall1int 2=7]"), "trace was: {trace}");
}

#[test]
fn interpreter_lowers_linked_extern_calls_through_ffi_builtins() {
    let source = r#"
extern("test-lib") fn strlen(s: String) -> Int;

fn main() -> Int {
  return strlen("hello");
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let trace = Arc::new(Mutex::new(String::new()));
    let host = TestHost {
        out: Arc::clone(&trace),
        next_handle_id: 0,
    };
    let interp = IrInterpreter::new(program, host);
    let result = interp.run_main();
    assert_eq!(result.expect("program should run"), IrValue::Int(5));
    let trace = trace.lock().expect("lock trace").clone();
    assert!(
        trace.contains("[ffiopen test-lib=0]"),
        "trace was: {trace}"
    );
    assert!(trace.contains("[ffibind 0:strlen=1]"), "trace was: {trace}");
    assert!(
        trace.contains("[fficall1stringint 1=hello]"),
        "trace was: {trace}"
    );
}

#[test]
fn interpreter_lowers_linked_extern_void_calls_through_ffi_builtins() {
    let source = r#"
extern("test-lib") fn trace(s: String) -> Void;

fn main() -> Int {
  trace("hello");
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let trace = Arc::new(Mutex::new(String::new()));
    let host = TestHost {
        out: Arc::clone(&trace),
        next_handle_id: 0,
    };
    let interp = IrInterpreter::new(program, host);
    let result = interp.run_main();
    assert_eq!(result.expect("program should run"), IrValue::Int(0));
    let trace = trace.lock().expect("lock trace").clone();
    assert!(trace.contains("[ffiopen test-lib=0]"), "trace was: {trace}");
    assert!(trace.contains("[ffibind 0:trace=1]"), "trace was: {trace}");
    assert!(
        trace.contains("[fficall1stringvoid 1=hello]"),
        "trace was: {trace}"
    );
}

#[test]
fn interpreter_lowers_linked_extern_int_void_calls_through_ffi_builtins() {
    let source = r#"
extern("test-lib") fn seed(x: Int) -> Void;

fn main() -> Int {
  seed(7);
  return 0;
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let trace = Arc::new(Mutex::new(String::new()));
    let host = TestHost {
        out: Arc::clone(&trace),
        next_handle_id: 0,
    };
    let interp = IrInterpreter::new(program, host);
    let result = interp.run_main();
    assert_eq!(result.expect("program should run"), IrValue::Int(0));
    let trace = trace.lock().expect("lock trace").clone();
    assert!(trace.contains("[ffiopen test-lib=0]"), "trace was: {trace}");
    assert!(trace.contains("[ffibind 0:seed=1]"), "trace was: {trace}");
    assert!(trace.contains("[fficall1intvoid 1=7]"), "trace was: {trace}");
}

#[test]
fn interpreter_lowers_linked_extern_two_string_calls_through_ffi_builtins() {
    let source = r#"
extern("test-lib") fn compare(a: String, b: String) -> Int;

fn main() -> Int {
  return compare("alpha", "beta");
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let trace = Arc::new(Mutex::new(String::new()));
    let host = TestHost {
        out: Arc::clone(&trace),
        next_handle_id: 0,
    };
    let interp = IrInterpreter::new(program, host);
    let result = interp.run_main();
    assert_eq!(result.expect("program should run"), IrValue::Int(0));
    let trace = trace.lock().expect("lock trace").clone();
    assert!(trace.contains("[ffiopen test-lib=0]"), "trace was: {trace}");
    assert!(trace.contains("[ffibind 0:compare=1]"), "trace was: {trace}");
    assert!(
        trace.contains("[fficall2stringint 1=alpha|beta]"),
        "trace was: {trace}"
    );
}

#[test]
fn interpreter_lowers_linked_extern_string_int_calls_through_ffi_builtins() {
    let source = r#"
extern("test-lib") fn count(s: String, n: Int) -> Int;

fn main() -> Int {
  return count("abc", 2);
}
"#;

    let program = ir::lowering::compile_source(source).expect("IR lowering should succeed");
    let trace = Arc::new(Mutex::new(String::new()));
    let host = TestHost {
        out: Arc::clone(&trace),
        next_handle_id: 0,
    };
    let interp = IrInterpreter::new(program, host);
    let result = interp.run_main();
    assert_eq!(result.expect("program should run"), IrValue::Int(5));
    let trace = trace.lock().expect("lock trace").clone();
    assert!(trace.contains("[ffiopen test-lib=0]"), "trace was: {trace}");
    assert!(trace.contains("[ffibind 0:count=1]"), "trace was: {trace}");
    assert!(
        trace.contains("[fficall2stringintint 1=abc|2]"),
        "trace was: {trace}"
    );
}

#[test]
fn interpreter_carries_dummy_task_handles_through_calls() {
    let source = r#"
import task;

fn make_task() -> task.Task[Int] {
  return task.__testTask(7);
}

fn make_channel() -> task.Channel {
  return task.__testChannel();
}

fn main() -> Int {
  let t: task.Task[Int] = make_task();
  let c: task.Channel = make_channel();
  let t2: task.Task[Int] = t;
  let c2: task.Channel = c;
  let v: Int = task.join(t2);
  let _ = c2;
  return v;
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(7));
}

#[test]
fn interpreter_supports_typed_task_channel_roundtrip() {
    let source = r#"
import task;

fn main() -> Int {
  let jobs: task.Channel[Int] = task.channel();
  task.send(jobs, 12);
  return task.recv(jobs);
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(12));
}

#[test]
fn interpreter_supports_task_spawn_and_join_roundtrip() {
    let source = r#"
import task;

fn job() -> Int {
  return 33;
}

fn main() -> Int {
  let t: task.Task[Int] = task.spawn(job);
  return task.join(t);
}
"#;

    let value = common::ir_run_ok(source);
    assert_eq!(value, IrValue::Int(33));
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
