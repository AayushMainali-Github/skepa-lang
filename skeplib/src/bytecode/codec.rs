use std::collections::HashMap;

use super::{BytecodeModule, FunctionChunk, Instr, Value};

impl BytecodeModule {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"SKBC");
        write_u32(&mut out, 1);
        write_u32(&mut out, self.functions.len() as u32);
        let mut funcs: Vec<_> = self.functions.values().collect();
        funcs.sort_by(|a, b| a.name.cmp(&b.name));
        for f in funcs {
            write_str(&mut out, &f.name);
            write_u32(&mut out, f.locals_count as u32);
            write_u32(&mut out, f.param_count as u32);
            write_u32(&mut out, f.code.len() as u32);
            for instr in &f.code {
                encode_instr(instr, &mut out);
            }
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut rd = Reader { bytes, idx: 0 };
        let magic = rd.read_exact(4)?;
        if magic != b"SKBC" {
            return Err("Invalid bytecode magic header".to_string());
        }
        let version = rd.read_u32()?;
        if version != 1 {
            return Err(format!("Unsupported bytecode version {version}"));
        }
        let funcs_len = rd.read_u32()? as usize;
        let mut functions = HashMap::new();
        for _ in 0..funcs_len {
            let name = rd.read_str()?;
            let locals_count = rd.read_u32()? as usize;
            let param_count = rd.read_u32()? as usize;
            let code_len = rd.read_u32()? as usize;
            let mut code = Vec::with_capacity(code_len);
            for _ in 0..code_len {
                code.push(decode_instr(&mut rd)?);
            }
            functions.insert(
                name.clone(),
                FunctionChunk {
                    name,
                    code,
                    locals_count,
                    param_count,
                },
            );
        }
        Ok(Self { functions })
    }
}

fn write_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}
fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_i64(out: &mut Vec<u8>, v: i64) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_f64(out: &mut Vec<u8>, v: f64) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_bool(out: &mut Vec<u8>, v: bool) {
    write_u8(out, if v { 1 } else { 0 });
}
fn write_str(out: &mut Vec<u8>, s: &str) {
    write_u32(out, s.len() as u32);
    out.extend_from_slice(s.as_bytes());
}

fn encode_value(v: &Value, out: &mut Vec<u8>) {
    match v {
        Value::Int(n) => {
            write_u8(out, 0);
            write_i64(out, *n);
        }
        Value::Float(n) => {
            write_u8(out, 1);
            write_f64(out, *n);
        }
        Value::Bool(b) => {
            write_u8(out, 2);
            write_bool(out, *b);
        }
        Value::String(s) => {
            write_u8(out, 3);
            write_str(out, s);
        }
        Value::Array(items) => {
            write_u8(out, 4);
            write_u32(out, items.len() as u32);
            for item in items {
                encode_value(item, out);
            }
        }
        Value::Unit => write_u8(out, 5),
    }
}

fn encode_instr(i: &Instr, out: &mut Vec<u8>) {
    match i {
        Instr::LoadConst(v) => {
            write_u8(out, 0);
            encode_value(v, out);
        }
        Instr::LoadLocal(s) => {
            write_u8(out, 1);
            write_u32(out, *s as u32);
        }
        Instr::StoreLocal(s) => {
            write_u8(out, 2);
            write_u32(out, *s as u32);
        }
        Instr::Pop => write_u8(out, 3),
        Instr::NegInt => write_u8(out, 4),
        Instr::NotBool => write_u8(out, 5),
        Instr::Add => write_u8(out, 6),
        Instr::SubInt => write_u8(out, 7),
        Instr::MulInt => write_u8(out, 8),
        Instr::DivInt => write_u8(out, 9),
        Instr::ModInt => write_u8(out, 10),
        Instr::Eq => write_u8(out, 11),
        Instr::Neq => write_u8(out, 12),
        Instr::LtInt => write_u8(out, 13),
        Instr::LteInt => write_u8(out, 14),
        Instr::GtInt => write_u8(out, 15),
        Instr::GteInt => write_u8(out, 16),
        Instr::AndBool => write_u8(out, 17),
        Instr::OrBool => write_u8(out, 18),
        Instr::Jump(t) => {
            write_u8(out, 19);
            write_u32(out, *t as u32);
        }
        Instr::JumpIfFalse(t) => {
            write_u8(out, 20);
            write_u32(out, *t as u32);
        }
        Instr::JumpIfTrue(t) => {
            write_u8(out, 21);
            write_u32(out, *t as u32);
        }
        Instr::Call { name, argc } => {
            write_u8(out, 22);
            write_str(out, name);
            write_u32(out, *argc as u32);
        }
        Instr::CallBuiltin {
            package,
            name,
            argc,
        } => {
            write_u8(out, 23);
            write_str(out, package);
            write_str(out, name);
            write_u32(out, *argc as u32);
        }
        Instr::MakeArray(n) => {
            write_u8(out, 24);
            write_u32(out, *n as u32);
        }
        Instr::MakeArrayRepeat(n) => {
            write_u8(out, 25);
            write_u32(out, *n as u32);
        }
        Instr::ArrayGet => write_u8(out, 26),
        Instr::ArraySet => write_u8(out, 27),
        Instr::ArraySetChain(n) => {
            write_u8(out, 28);
            write_u32(out, *n as u32);
        }
        Instr::ArrayLen => write_u8(out, 29),
        Instr::Return => write_u8(out, 30),
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    idx: usize,
}
impl<'a> Reader<'a> {
    fn read_exact(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.idx + n > self.bytes.len() {
            return Err("Unexpected EOF while decoding bytecode".to_string());
        }
        let s = &self.bytes[self.idx..self.idx + n];
        self.idx += n;
        Ok(s)
    }
    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_exact(1)?[0])
    }
    fn read_u32(&mut self) -> Result<u32, String> {
        let b = self.read_exact(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn read_i64(&mut self) -> Result<i64, String> {
        let b = self.read_exact(8)?;
        Ok(i64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
    fn read_f64(&mut self) -> Result<f64, String> {
        let b = self.read_exact(8)?;
        Ok(f64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
    fn read_bool(&mut self) -> Result<bool, String> {
        Ok(self.read_u8()? != 0)
    }
    fn read_str(&mut self) -> Result<String, String> {
        let n = self.read_u32()? as usize;
        let b = self.read_exact(n)?;
        String::from_utf8(b.to_vec()).map_err(|e| e.to_string())
    }
}

fn decode_value(rd: &mut Reader<'_>) -> Result<Value, String> {
    match rd.read_u8()? {
        0 => Ok(Value::Int(rd.read_i64()?)),
        1 => Ok(Value::Float(rd.read_f64()?)),
        2 => Ok(Value::Bool(rd.read_bool()?)),
        3 => Ok(Value::String(rd.read_str()?)),
        4 => {
            let n = rd.read_u32()? as usize;
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(decode_value(rd)?);
            }
            Ok(Value::Array(items))
        }
        5 => Ok(Value::Unit),
        t => Err(format!("Unknown value tag {t}")),
    }
}

fn decode_instr(rd: &mut Reader<'_>) -> Result<Instr, String> {
    Ok(match rd.read_u8()? {
        0 => Instr::LoadConst(decode_value(rd)?),
        1 => Instr::LoadLocal(rd.read_u32()? as usize),
        2 => Instr::StoreLocal(rd.read_u32()? as usize),
        3 => Instr::Pop,
        4 => Instr::NegInt,
        5 => Instr::NotBool,
        6 => Instr::Add,
        7 => Instr::SubInt,
        8 => Instr::MulInt,
        9 => Instr::DivInt,
        10 => Instr::ModInt,
        11 => Instr::Eq,
        12 => Instr::Neq,
        13 => Instr::LtInt,
        14 => Instr::LteInt,
        15 => Instr::GtInt,
        16 => Instr::GteInt,
        17 => Instr::AndBool,
        18 => Instr::OrBool,
        19 => Instr::Jump(rd.read_u32()? as usize),
        20 => Instr::JumpIfFalse(rd.read_u32()? as usize),
        21 => Instr::JumpIfTrue(rd.read_u32()? as usize),
        22 => Instr::Call {
            name: rd.read_str()?,
            argc: rd.read_u32()? as usize,
        },
        23 => Instr::CallBuiltin {
            package: rd.read_str()?,
            name: rd.read_str()?,
            argc: rd.read_u32()? as usize,
        },
        24 => Instr::MakeArray(rd.read_u32()? as usize),
        25 => Instr::MakeArrayRepeat(rd.read_u32()? as usize),
        26 => Instr::ArrayGet,
        27 => Instr::ArraySet,
        28 => Instr::ArraySetChain(rd.read_u32()? as usize),
        29 => Instr::ArrayLen,
        30 => Instr::Return,
        t => return Err(format!("Unknown instruction tag {t}")),
    })
}
