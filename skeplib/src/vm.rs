use crate::bytecode::{FunctionChunk, Instr, Value};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vm;

impl Vm {
    pub fn run_main(chunk: &FunctionChunk) -> Result<Value, String> {
        let mut stack: Vec<Value> = Vec::new();
        let mut locals: Vec<Value> = vec![Value::Unit; chunk.locals_count.max(1)];

        let mut ip = 0usize;
        while ip < chunk.code.len() {
            match &chunk.code[ip] {
                Instr::LoadConst(v) => stack.push(v.clone()),
                Instr::LoadLocal(slot) => {
                    let Some(v) = locals.get(*slot).cloned() else {
                        return Err(format!("Invalid local slot {slot}"));
                    };
                    stack.push(v);
                }
                Instr::StoreLocal(slot) => {
                    let Some(v) = stack.pop() else {
                        return Err("Stack underflow on StoreLocal".to_string());
                    };
                    if *slot >= locals.len() {
                        locals.resize(*slot + 1, Value::Unit);
                    }
                    locals[*slot] = v;
                }
                Instr::Pop => {
                    if stack.pop().is_none() {
                        return Err("Stack underflow on Pop".to_string());
                    }
                }
                Instr::NegInt => {
                    let Some(Value::Int(v)) = stack.pop() else {
                        return Err("NegInt expects Int".to_string());
                    };
                    stack.push(Value::Int(-v));
                }
                Instr::NotBool => {
                    let Some(Value::Bool(v)) = stack.pop() else {
                        return Err("NotBool expects Bool".to_string());
                    };
                    stack.push(Value::Bool(!v));
                }
                Instr::AddInt
                | Instr::SubInt
                | Instr::MulInt
                | Instr::DivInt
                | Instr::EqInt
                | Instr::NeqInt
                | Instr::LtInt
                | Instr::LteInt
                | Instr::GtInt
                | Instr::GteInt => {
                    let Some(Value::Int(r)) = stack.pop() else {
                        return Err("int binary op expects rhs Int".to_string());
                    };
                    let Some(Value::Int(l)) = stack.pop() else {
                        return Err("int binary op expects lhs Int".to_string());
                    };
                    match chunk.code[ip] {
                        Instr::AddInt => stack.push(Value::Int(l + r)),
                        Instr::SubInt => stack.push(Value::Int(l - r)),
                        Instr::MulInt => stack.push(Value::Int(l * r)),
                        Instr::DivInt => {
                            if r == 0 {
                                return Err("division by zero".to_string());
                            }
                            stack.push(Value::Int(l / r));
                        }
                        Instr::EqInt => stack.push(Value::Bool(l == r)),
                        Instr::NeqInt => stack.push(Value::Bool(l != r)),
                        Instr::LtInt => stack.push(Value::Bool(l < r)),
                        Instr::LteInt => stack.push(Value::Bool(l <= r)),
                        Instr::GtInt => stack.push(Value::Bool(l > r)),
                        Instr::GteInt => stack.push(Value::Bool(l >= r)),
                        _ => unreachable!(),
                    }
                }
                Instr::AndBool | Instr::OrBool => {
                    let Some(Value::Bool(r)) = stack.pop() else {
                        return Err("logical op expects rhs Bool".to_string());
                    };
                    let Some(Value::Bool(l)) = stack.pop() else {
                        return Err("logical op expects lhs Bool".to_string());
                    };
                    match chunk.code[ip] {
                        Instr::AndBool => stack.push(Value::Bool(l && r)),
                        Instr::OrBool => stack.push(Value::Bool(l || r)),
                        _ => unreachable!(),
                    }
                }
                Instr::Jump(target) => {
                    ip = *target;
                    continue;
                }
                Instr::JumpIfFalse(target) => {
                    let Some(Value::Bool(v)) = stack.pop() else {
                        return Err("JumpIfFalse expects Bool".to_string());
                    };
                    if !v {
                        ip = *target;
                        continue;
                    }
                }
                Instr::Return => {
                    return Ok(stack.pop().unwrap_or(Value::Unit));
                }
            }
            ip += 1;
        }

        Ok(Value::Unit)
    }
}
