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
                Instr::AddInt | Instr::SubInt | Instr::MulInt | Instr::DivInt => {
                    let Some(Value::Int(r)) = stack.pop() else {
                        return Err("binary op expects rhs Int".to_string());
                    };
                    let Some(Value::Int(l)) = stack.pop() else {
                        return Err("binary op expects lhs Int".to_string());
                    };
                    let out = match chunk.code[ip] {
                        Instr::AddInt => l + r,
                        Instr::SubInt => l - r,
                        Instr::MulInt => l * r,
                        Instr::DivInt => {
                            if r == 0 {
                                return Err("division by zero".to_string());
                            }
                            l / r
                        }
                        _ => unreachable!(),
                    };
                    stack.push(Value::Int(out));
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
