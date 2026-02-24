use crate::bytecode::Value;

use super::VmError;

pub trait BuiltinHost {
    fn write(&mut self, s: &str, newline: bool) -> Result<(), VmError>;
    fn read_line(&mut self) -> Result<String, VmError>;
    fn vec_new(&mut self) -> Result<u64, VmError>;
    fn vec_len(&mut self, id: u64) -> Result<usize, VmError>;
    fn vec_push(&mut self, id: u64, v: Value) -> Result<(), VmError>;
    fn vec_get(&mut self, id: u64, idx: i64) -> Result<Value, VmError>;
    fn vec_set(&mut self, id: u64, idx: i64, v: Value) -> Result<(), VmError>;
    fn vec_delete(&mut self, id: u64, idx: i64) -> Result<Value, VmError>;
    fn set_random_seed(&mut self, seed: u64) -> Result<(), VmError> {
        let _ = seed;
        Ok(())
    }
    fn next_random_u64(&mut self) -> Result<u64, VmError>;
}
