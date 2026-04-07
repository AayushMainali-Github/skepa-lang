use crate::ast::Expr;
use crate::builtins::find_builtin_spec;
use crate::ir::{BlockId, ConstValue, Instr, IrType, Operand};

use super::context::{ExternFunctionSig, FunctionLowering, IrLowerer};

impl IrLowerer {
    fn direct_callee_target(&self, callee: &Expr) -> Option<String> {
        match callee {
            Expr::Ident(name) => Some(
                self.direct_import_calls
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| self.qualify_name(name)),
            ),
            Expr::Path(parts) => Some({
                let name = parts.join(".");
                self.namespace_call_targets
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| self.qualify_name(&name))
            }),
            _ => None,
        }
    }

    pub(super) fn compile_call(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        callee: &Expr,
        args: &[Expr],
    ) -> Option<Operand> {
        if let Expr::Ident(name) = callee {
            match (name.as_str(), args) {
                ("Some" | "some", [value_expr]) => {
                    let value = self.compile_expr(func, lowering, value_expr)?;
                    let value_ty = self.infer_operand_type(func, &value);
                    let ret_ty = IrType::Option {
                        value: Box::new(value_ty),
                    };
                    let dst = Some(self.builder.push_temp(func, ret_ty.clone()));
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::CallBuiltin {
                            dst,
                            ret_ty: ret_ty.clone(),
                            builtin: crate::ir::BuiltinCall {
                                package: "option".to_string(),
                                name: "some".to_string(),
                            },
                            args: vec![value],
                        },
                    );
                    return OkOperand::from_call_result(dst);
                }
                ("None" | "none", []) => {
                    let ret_ty = IrType::Option {
                        value: Box::new(IrType::Unknown),
                    };
                    let dst = Some(self.builder.push_temp(func, ret_ty.clone()));
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::CallBuiltin {
                            dst,
                            ret_ty: ret_ty.clone(),
                            builtin: crate::ir::BuiltinCall {
                                package: "option".to_string(),
                                name: "none".to_string(),
                            },
                            args: Vec::new(),
                        },
                    );
                    return OkOperand::from_call_result(dst);
                }
                ("Ok" | "ok", [value_expr]) => {
                    let value = self.compile_expr(func, lowering, value_expr)?;
                    let value_ty = self.infer_operand_type(func, &value);
                    let ret_ty = IrType::Result {
                        ok: Box::new(value_ty),
                        err: Box::new(IrType::Unknown),
                    };
                    let dst = Some(self.builder.push_temp(func, ret_ty.clone()));
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::CallBuiltin {
                            dst,
                            ret_ty: ret_ty.clone(),
                            builtin: crate::ir::BuiltinCall {
                                package: "result".to_string(),
                                name: "ok".to_string(),
                            },
                            args: vec![value],
                        },
                    );
                    return OkOperand::from_call_result(dst);
                }
                ("Err" | "err", [value_expr]) => {
                    let value = self.compile_expr(func, lowering, value_expr)?;
                    let value_ty = self.infer_operand_type(func, &value);
                    let ret_ty = IrType::Result {
                        ok: Box::new(IrType::Unknown),
                        err: Box::new(value_ty),
                    };
                    let dst = Some(self.builder.push_temp(func, ret_ty.clone()));
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::CallBuiltin {
                            dst,
                            ret_ty: ret_ty.clone(),
                            builtin: crate::ir::BuiltinCall {
                                package: "result".to_string(),
                                name: "err".to_string(),
                            },
                            args: vec![value],
                        },
                    );
                    return OkOperand::from_call_result(dst);
                }
                _ => {}
            }
        }

        let mut lowered_args = Vec::with_capacity(args.len());
        for arg in args {
            lowered_args.push(self.compile_expr(func, lowering, arg)?);
        }

        if let Some(target_name) = self.direct_callee_target(callee) {
            if let Some(extern_sig) = self.extern_functions.get(&target_name).cloned() {
                return self.compile_extern_call(
                    func,
                    lowering.current_block,
                    &extern_sig,
                    lowered_args,
                );
            }
            if let Some(sig) = self.functions.get(&target_name).cloned() {
                let dst = if sig.ret.is_void() {
                    None
                } else {
                    Some(self.builder.push_temp(func, sig.ret.clone()))
                };
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::CallDirect {
                        dst,
                        ret_ty: sig.ret.clone(),
                        function: sig.id,
                        args: lowered_args,
                    },
                );
                return OkOperand::from_call_result(dst);
            }
        }

        match callee {
            Expr::Ident(_) | Expr::Path(_) => {
                let callee = self.compile_expr(func, lowering, callee)?;
                let ret_ty = self.indirect_call_return_type(func, &callee);
                let dst = if ret_ty.is_void() {
                    None
                } else {
                    Some(self.builder.push_temp(func, ret_ty.clone()))
                };
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::CallIndirect {
                        dst,
                        ret_ty: ret_ty.clone(),
                        callee,
                        args: lowered_args,
                    },
                );
                OkOperand::from_call_result(dst)
            }
            Expr::Field { base, field } => {
                if let Expr::Ident(package) = base.as_ref() {
                    let is_value_receiver = lowering.locals.contains_key(package)
                        || self.globals.contains_key(package)
                        || self.globals.contains_key(&self.qualify_name(package))
                        || self.functions.contains_key(package);
                    if package == "vec" {
                        return self.compile_vec_call(
                            func,
                            lowering.current_block,
                            field,
                            lowered_args,
                        );
                    }
                    if !is_value_receiver
                        && let Some(target_name) = self
                            .namespace_call_targets
                            .get(&format!("{package}.{field}"))
                        && let Some(sig) = self.functions.get(target_name).cloned()
                    {
                        let dst = if sig.ret.is_void() {
                            None
                        } else {
                            Some(self.builder.push_temp(func, sig.ret.clone()))
                        };
                        self.builder.push_instr(
                            func,
                            lowering.current_block,
                            Instr::CallDirect {
                                dst,
                                ret_ty: sig.ret.clone(),
                                function: sig.id,
                                args: lowered_args,
                            },
                        );
                        return OkOperand::from_call_result(dst);
                    }
                    if !is_value_receiver {
                        let ret_ty = self
                            .builtin_return_type(func, package, field, &lowered_args)
                            .unwrap_or(IrType::Unknown);
                        let dst = if ret_ty.is_void() {
                            None
                        } else {
                            Some(self.builder.push_temp(func, ret_ty.clone()))
                        };
                        self.builder.push_instr(
                            func,
                            lowering.current_block,
                            Instr::CallBuiltin {
                                dst,
                                ret_ty: ret_ty.clone(),
                                builtin: crate::ir::BuiltinCall {
                                    package: package.clone(),
                                    name: field.clone(),
                                },
                                args: lowered_args,
                            },
                        );
                        return OkOperand::from_call_result(dst);
                    }
                }
                self.compile_method_call(func, lowering, base, field, lowered_args)
            }
            _ => {
                let callee = self.compile_expr(func, lowering, callee)?;
                let ret_ty = self.indirect_call_return_type(func, &callee);
                let dst = if ret_ty.is_void() {
                    None
                } else {
                    Some(self.builder.push_temp(func, ret_ty.clone()))
                };
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::CallIndirect {
                        dst,
                        ret_ty: ret_ty.clone(),
                        callee,
                        args: lowered_args,
                    },
                );
                OkOperand::from_call_result(dst)
            }
        }
    }

    fn compile_extern_call(
        &mut self,
        func: &mut crate::ir::IrFunction,
        block: BlockId,
        sig: &ExternFunctionSig,
        args: Vec<Operand>,
    ) -> Option<Operand> {
        let Some(library) = sig.library.as_ref() else {
            self.unsupported(format!(
                "extern function `{}` requires a linked library path in IR lowering",
                sig.symbol
            ));
            return None;
        };

        let lib_result_ty = IrType::Result {
            ok: Box::new(IrType::Opaque("ffi.Library".to_string())),
            err: Box::new(IrType::String),
        };
        let lib_result_dst = self.builder.push_temp(func, lib_result_ty.clone());
        self.builder.push_instr(
            func,
            block,
            Instr::CallBuiltin {
                dst: Some(lib_result_dst),
                ret_ty: lib_result_ty,
                builtin: crate::ir::BuiltinCall {
                    package: "ffi".to_string(),
                    name: "open".to_string(),
                },
                args: vec![Operand::Const(ConstValue::String(library.clone()))],
            },
        );

        let lib_ty = IrType::Opaque("ffi.Library".to_string());
        let lib_dst = self.builder.push_temp(func, lib_ty.clone());
        self.builder.push_instr(
            func,
            block,
            Instr::CallBuiltin {
                dst: Some(lib_dst),
                ret_ty: lib_ty.clone(),
                builtin: crate::ir::BuiltinCall {
                    package: "result".to_string(),
                    name: "unwrapOk".to_string(),
                },
                args: vec![Operand::Temp(lib_result_dst)],
            },
        );

        let sym_result_ty = IrType::Result {
            ok: Box::new(IrType::Opaque("ffi.Symbol".to_string())),
            err: Box::new(IrType::String),
        };
        let sym_result_dst = self.builder.push_temp(func, sym_result_ty.clone());
        self.builder.push_instr(
            func,
            block,
            Instr::CallBuiltin {
                dst: Some(sym_result_dst),
                ret_ty: sym_result_ty,
                builtin: crate::ir::BuiltinCall {
                    package: "ffi".to_string(),
                    name: "bind".to_string(),
                },
                args: vec![
                    Operand::Temp(lib_dst),
                    Operand::Const(ConstValue::String(sig.symbol.clone())),
                ],
            },
        );

        let sym_ty = IrType::Opaque("ffi.Symbol".to_string());
        let sym_dst = self.builder.push_temp(func, sym_ty.clone());
        self.builder.push_instr(
            func,
            block,
            Instr::CallBuiltin {
                dst: Some(sym_dst),
                ret_ty: sym_ty.clone(),
                builtin: crate::ir::BuiltinCall {
                    package: "result".to_string(),
                    name: "unwrapOk".to_string(),
                },
                args: vec![Operand::Temp(sym_result_dst)],
            },
        );

        let Some(signature_text) = self.extern_abi_signature(sig) else {
            self.unsupported(format!(
                "extern function `{}` uses unsupported lowered ABI in IR",
                sig.symbol
            ));
            return None;
        };
        let mut call_args = Vec::with_capacity(args.len() + 2);
        call_args.push(Operand::Temp(sym_dst));
        call_args.push(Operand::Const(ConstValue::String(signature_text)));
        call_args.extend(args);
        let call_dst = if sig.ret.is_void() {
            None
        } else {
            Some(self.builder.push_temp(func, sig.ret.clone()))
        };
        self.builder.push_instr(
            func,
            block,
            Instr::CallBuiltin {
                dst: call_dst,
                ret_ty: sig.ret.clone(),
                builtin: crate::ir::BuiltinCall {
                    package: "ffi".to_string(),
                    name: "call".to_string(),
                },
                args: call_args,
            },
        );

        self.builder.push_instr(
            func,
            block,
            Instr::CallBuiltin {
                dst: None,
                ret_ty: IrType::Void,
                builtin: crate::ir::BuiltinCall {
                    package: "ffi".to_string(),
                    name: "closeSymbol".to_string(),
                },
                args: vec![Operand::Temp(sym_dst)],
            },
        );
        self.builder.push_instr(
            func,
            block,
            Instr::CallBuiltin {
                dst: None,
                ret_ty: IrType::Void,
                builtin: crate::ir::BuiltinCall {
                    package: "ffi".to_string(),
                    name: "closeLibrary".to_string(),
                },
                args: vec![Operand::Temp(lib_dst)],
            },
        );

        OkOperand::from_call_result(call_dst)
    }

    fn compile_method_call(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        base: &Expr,
        field: &str,
        mut args: Vec<Operand>,
    ) -> Option<Operand> {
        let receiver = self.compile_expr(func, lowering, base)?;
        let IrType::Named(struct_name) = self.infer_operand_type(func, &receiver) else {
            self.unsupported(
                "method call on non-struct receiver is not in the initial IR lowering subset",
            );
            return None;
        };
        let method_name = Self::mangle_method_name(&struct_name, field);
        let Some(sig) = self.functions.get(&method_name).cloned() else {
            self.unsupported(format!(
                "unknown method `{field}` for struct `{struct_name}` in IR lowering"
            ));
            return None;
        };
        let mut call_args = Vec::with_capacity(args.len() + 1);
        call_args.push(receiver);
        call_args.append(&mut args);
        let dst = if sig.ret.is_void() {
            None
        } else {
            Some(self.builder.push_temp(func, sig.ret.clone()))
        };
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::CallDirect {
                dst,
                ret_ty: sig.ret.clone(),
                function: sig.id,
                args: call_args,
            },
        );
        OkOperand::from_call_result(dst)
    }

    pub(super) fn try_compile_vec_new_let(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        name: &str,
        ty: &Option<crate::ast::TypeName>,
        value: &Expr,
    ) -> Option<bool> {
        let Some(crate::ast::TypeName::Vec { elem }) = ty else {
            return None;
        };
        let Expr::Call { callee, args } = value else {
            return None;
        };
        if !args.is_empty()
            || !matches!(&**callee, Expr::Field { base, field } if field == "new" && matches!(&**base, Expr::Ident(pkg) if pkg == "vec"))
        {
            return None;
        }

        let elem_ty = IrType::from(&crate::types::TypeInfo::from_ast(elem));
        let local_ty = IrType::Vec {
            elem: Box::new(elem_ty.clone()),
        };
        let local = self
            .builder
            .push_local(func, name.to_string(), local_ty.clone());
        lowering.locals.insert(name.to_string(), local);
        let dst = self.builder.push_temp(func, local_ty.clone());
        self.builder
            .push_instr(func, lowering.current_block, Instr::VecNew { dst, elem_ty });
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::StoreLocal {
                local,
                ty: local_ty,
                value: Operand::Temp(dst),
            },
        );
        Some(true)
    }

    pub(super) fn try_compile_map_new_let(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        name: &str,
        ty: &Option<crate::ast::TypeName>,
        value: &Expr,
    ) -> Option<bool> {
        let Some(crate::ast::TypeName::Map { value: map_value }) = ty else {
            return None;
        };
        let Expr::Call { callee, args } = value else {
            return None;
        };
        if !args.is_empty()
            || !matches!(&**callee, Expr::Field { base, field } if field == "new" && matches!(&**base, Expr::Ident(pkg) if pkg == "map"))
        {
            return None;
        }

        let value_ty = IrType::from(&crate::types::TypeInfo::from_ast(map_value));
        let local_ty = IrType::Map {
            value: Box::new(value_ty),
        };
        let local = self
            .builder
            .push_local(func, name.to_string(), local_ty.clone());
        lowering.locals.insert(name.to_string(), local);
        let dst = self.builder.push_temp(func, local_ty.clone());
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::CallBuiltin {
                dst: Some(dst),
                ret_ty: local_ty.clone(),
                builtin: crate::ir::BuiltinCall {
                    package: "map".to_string(),
                    name: "new".to_string(),
                },
                args: Vec::new(),
            },
        );
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::StoreLocal {
                local,
                ty: local_ty,
                value: Operand::Temp(dst),
            },
        );
        Some(true)
    }

    pub(super) fn try_compile_task_channel_let(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        name: &str,
        ty: &Option<crate::ast::TypeName>,
        value: &Expr,
    ) -> Option<bool> {
        let Some(crate::ast::TypeName::Named(channel_name)) = ty else {
            return None;
        };
        let _ = crate::types::task_channel_value_type(channel_name)?;
        let Expr::Call { callee, args } = value else {
            return None;
        };
        if !args.is_empty()
            || !matches!(&**callee, Expr::Field { base, field } if field == "channel" && matches!(&**base, Expr::Ident(pkg) if pkg == "task"))
        {
            return None;
        }

        let local_ty = IrType::Opaque(channel_name.clone());
        let local = self
            .builder
            .push_local(func, name.to_string(), local_ty.clone());
        lowering.locals.insert(name.to_string(), local);
        let dst = self.builder.push_temp(func, local_ty.clone());
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::CallBuiltin {
                dst: Some(dst),
                ret_ty: local_ty.clone(),
                builtin: crate::ir::BuiltinCall {
                    package: "task".to_string(),
                    name: "channel".to_string(),
                },
                args: Vec::new(),
            },
        );
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::StoreLocal {
                local,
                ty: local_ty,
                value: Operand::Temp(dst),
            },
        );
        Some(true)
    }

    fn compile_vec_call(
        &mut self,
        func: &mut crate::ir::IrFunction,
        block: BlockId,
        field: &str,
        args: Vec<Operand>,
    ) -> Option<Operand> {
        match (field, args.as_slice()) {
            ("new", []) => {
                let dst = self.builder.push_temp(
                    func,
                    IrType::Vec {
                        elem: Box::new(IrType::Unknown),
                    },
                );
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecNew {
                        dst,
                        elem_ty: IrType::Unknown,
                    },
                );
                Some(Operand::Temp(dst))
            }
            ("len", [vec]) => {
                let dst = self.builder.push_temp(func, IrType::Int);
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecLen {
                        dst,
                        vec: vec.clone(),
                    },
                );
                Some(Operand::Temp(dst))
            }
            ("push", [vec, value]) => {
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecPush {
                        vec: vec.clone(),
                        value: value.clone(),
                    },
                );
                Some(Operand::Const(ConstValue::Unit))
            }
            ("get", [vec, index]) => {
                let elem_ty = self.array_element_type(func, vec);
                let dst = self.builder.push_temp(func, elem_ty.clone());
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecGet {
                        dst,
                        elem_ty,
                        vec: vec.clone(),
                        index: index.clone(),
                    },
                );
                Some(Operand::Temp(dst))
            }
            ("set", [vec, index, value]) => {
                let elem_ty = self.array_element_type(func, vec);
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecSet {
                        elem_ty,
                        vec: vec.clone(),
                        index: index.clone(),
                        value: value.clone(),
                    },
                );
                Some(Operand::Const(ConstValue::Unit))
            }
            ("delete", [vec, index]) => {
                let elem_ty = self.array_element_type(func, vec);
                let dst = self.builder.push_temp(func, elem_ty.clone());
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecDelete {
                        dst,
                        elem_ty,
                        vec: vec.clone(),
                        index: index.clone(),
                    },
                );
                Some(Operand::Temp(dst))
            }
            _ => {
                self.unsupported(format!("vec.{field} is not supported in IR lowering"));
                None
            }
        }
    }

    fn builtin_return_type(
        &self,
        func: &crate::ir::IrFunction,
        package: &str,
        name: &str,
        args: &[Operand],
    ) -> Option<IrType> {
        match (package, name) {
            ("option", "some") => {
                let value = args.first()?;
                return Some(IrType::Option {
                    value: Box::new(self.infer_operand_type(func, value)),
                });
            }
            ("option", "none") => {
                return Some(IrType::Option {
                    value: Box::new(IrType::Unknown),
                });
            }
            ("result", "ok") => {
                let value = args.first()?;
                return Some(IrType::Result {
                    ok: Box::new(self.infer_operand_type(func, value)),
                    err: Box::new(IrType::Unknown),
                });
            }
            ("result", "err") => {
                let value = args.first()?;
                return Some(IrType::Result {
                    ok: Box::new(IrType::Unknown),
                    err: Box::new(self.infer_operand_type(func, value)),
                });
            }
            ("net", "__testSocket") => {
                return Some(IrType::Opaque("net.Socket".to_string()));
            }
            ("net", "connect") | ("net", "accept") | ("net", "tlsConnect") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Opaque("net.Socket".to_string())),
                    err: Box::new(IrType::String),
                });
            }
            ("net", "listen") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Opaque("net.Listener".to_string())),
                    err: Box::new(IrType::String),
                });
            }
            ("net", "read") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::String),
                    err: Box::new(IrType::String),
                });
            }
            ("net", "readBytes") | ("net", "readN") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Bytes),
                    err: Box::new(IrType::String),
                });
            }
            ("net", "write") | ("net", "writeBytes") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Void),
                    err: Box::new(IrType::String),
                });
            }
            ("net", "localAddr") | ("net", "peerAddr") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::String),
                    err: Box::new(IrType::String),
                });
            }
            ("net", "flush") | ("net", "setReadTimeout") | ("net", "setWriteTimeout") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Void),
                    err: Box::new(IrType::String),
                });
            }
            ("net", "parseUrl") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Map {
                        value: Box::new(IrType::String),
                    }),
                    err: Box::new(IrType::String),
                });
            }
            ("ffi", "open") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Opaque("ffi.Library".to_string())),
                    err: Box::new(IrType::String),
                });
            }
            ("ffi", "bind") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Opaque("ffi.Symbol".to_string())),
                    err: Box::new(IrType::String),
                });
            }
            ("ffi", "call0Int")
            | ("ffi", "call1Int")
            | ("ffi", "call1StringInt")
            | ("ffi", "call2IntInt")
            | ("ffi", "call2BytesIntInt")
            | ("ffi", "call2StringInt")
            | ("ffi", "call2StringIntInt")
            | ("ffi", "call1BytesInt") => {
                return Some(IrType::Int);
            }
            ("ffi", "call0Bool") | ("ffi", "call1IntBool") => {
                return Some(IrType::Bool);
            }
            ("ffi", "call0Void") | ("ffi", "call1IntVoid") | ("ffi", "call1StringVoid") => {
                return Some(IrType::Void);
            }
            ("net", "fetch") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Map {
                        value: Box::new(IrType::String),
                    }),
                    err: Box::new(IrType::String),
                });
            }
            ("net", "resolve") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::String),
                    err: Box::new(IrType::String),
                });
            }
            ("datetime", "parseUnix") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Int),
                    err: Box::new(IrType::String),
                });
            }
            ("fs", "readText") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::String),
                    err: Box::new(IrType::String),
                });
            }
            ("fs", "exists") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Bool),
                    err: Box::new(IrType::String),
                });
            }
            ("fs", "writeText")
            | ("fs", "appendText")
            | ("fs", "mkdirAll")
            | ("fs", "removeFile")
            | ("fs", "removeDirAll") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Void),
                    err: Box::new(IrType::String),
                });
            }
            ("os", "envGet") => {
                return Some(IrType::Option {
                    value: Box::new(IrType::String),
                });
            }
            ("os", "arg") => {
                return Some(IrType::Option {
                    value: Box::new(IrType::String),
                });
            }
            ("os", "exec") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::Int),
                    err: Box::new(IrType::String),
                });
            }
            ("os", "execOut") => {
                return Some(IrType::Result {
                    ok: Box::new(IrType::String),
                    err: Box::new(IrType::String),
                });
            }
            ("task", "__testTask") => {
                let value = args.first()?;
                return Some(IrType::Opaque(format!(
                    "task.Task[{}]",
                    self.display_ir_type(&self.infer_operand_type(func, value))
                )));
            }
            ("task", "__testChannel") => {
                return Some(IrType::Opaque("task.Channel".to_string()));
            }
            ("task", "spawn") => {
                let function = args.first()?;
                if let IrType::Fn { params, ret } = self.infer_operand_type(func, function)
                    && params.is_empty()
                {
                    return Some(IrType::Opaque(format!(
                        "task.Task[{}]",
                        self.display_ir_type(&ret)
                    )));
                }
            }
            ("task", "join") => {
                let task = args.first()?;
                if let IrType::Opaque(name) = self.infer_operand_type(func, task) {
                    return crate::types::task_task_value_type(&name)
                        .map(|value| IrType::from(&value));
                }
            }
            ("task", "recv") => {
                let channel = args.first()?;
                if let IrType::Opaque(name) = self.infer_operand_type(func, channel) {
                    return crate::types::task_channel_value_type(&name)
                        .map(|value| IrType::from(&value));
                }
            }
            ("option", "unwrapSome") => {
                let value = args.first()?;
                if let IrType::Option { value } = self.infer_operand_type(func, value) {
                    return Some(*value);
                }
            }
            ("result", "unwrapOk") => {
                let value = args.first()?;
                if let IrType::Result { ok, .. } = self.infer_operand_type(func, value) {
                    return Some(*ok);
                }
            }
            ("result", "unwrapErr") => {
                let value = args.first()?;
                if let IrType::Result { err, .. } = self.infer_operand_type(func, value) {
                    return Some(*err);
                }
            }
            ("map", "get") | ("map", "remove") => {
                let map = args.first()?;
                if let IrType::Map { value } = self.infer_operand_type(func, map) {
                    return Some(IrType::Option { value });
                }
            }
            ("bytes", "tryGet") => {
                return Some(IrType::Option {
                    value: Box::new(IrType::Int),
                });
            }
            ("vec", "tryGet") => {
                let vec = args.first()?;
                if let IrType::Vec { elem } = self.infer_operand_type(func, vec) {
                    return Some(IrType::Option { value: elem });
                }
            }
            ("arr", "tryFirst") | ("arr", "tryLast") => {
                let array = args.first()?;
                if let IrType::Array { elem, .. } = self.infer_operand_type(func, array) {
                    return Some(IrType::Option { value: elem });
                }
            }
            _ => {}
        }
        let spec = find_builtin_spec(package, name)?;
        Some(IrType::from(&spec.sig.ret))
    }

    fn indirect_call_return_type(&self, func: &crate::ir::IrFunction, callee: &Operand) -> IrType {
        match self.infer_operand_type(func, callee) {
            IrType::Fn { ret, .. } => (*ret).clone(),
            _ => IrType::Unknown,
        }
    }

    fn extern_abi_signature(&self, sig: &ExternFunctionSig) -> Option<String> {
        let params = sig
            .params
            .iter()
            .map(Self::extern_abi_type_code)
            .collect::<Option<Vec<_>>>()?
            .join("");
        let ret = Self::extern_abi_type_code(&sig.ret)?;
        Some(format!("{params}->{ret}"))
    }

    fn extern_abi_type_code(ty: &IrType) -> Option<&'static str> {
        match ty {
            IrType::Int => Some("I"),
            IrType::Bool => Some("B"),
            IrType::Void => Some("V"),
            IrType::String => Some("S"),
            IrType::Bytes => Some("Y"),
            _ => None,
        }
    }
}

struct OkOperand;

impl OkOperand {
    fn from_call_result(dst: Option<crate::ir::TempId>) -> Option<Operand> {
        Some(match dst {
            Some(dst) => Operand::Temp(dst),
            None => Operand::Const(ConstValue::Unit),
        })
    }
}

impl IrLowerer {
    fn display_ir_type(&self, value: &IrType) -> String {
        let _ = self;
        match value {
            IrType::Int => "Int".to_string(),
            IrType::Float => "Float".to_string(),
            IrType::Bool => "Bool".to_string(),
            IrType::String => "String".to_string(),
            IrType::Bytes => "Bytes".to_string(),
            IrType::Void => "Void".to_string(),
            IrType::Option { value } => format!("Option[{}]", self.display_ir_type(value)),
            IrType::Result { ok, err } => format!(
                "Result[{}, {}]",
                self.display_ir_type(ok),
                self.display_ir_type(err)
            ),
            IrType::Named(name) | IrType::Opaque(name) => name.clone(),
            IrType::Array { elem, size } => format!("[{}; {}]", self.display_ir_type(elem), size),
            IrType::Vec { elem } => format!("Vec[{}]", self.display_ir_type(elem)),
            IrType::Map { value } => format!("Map[String, {}]", self.display_ir_type(value)),
            IrType::Fn { params, ret } => format!(
                "Fn({}) -> {}",
                params
                    .iter()
                    .map(|param| self.display_ir_type(param))
                    .collect::<Vec<_>>()
                    .join(", "),
                self.display_ir_type(ret)
            ),
            IrType::Unknown => "Unknown".to_string(),
        }
    }
}
