use super::VmError;

pub trait BuiltinHost {
    fn write(&mut self, s: &str, newline: bool) -> Result<(), VmError>;
    fn read_line(&mut self) -> Result<String, VmError>;
    fn set_random_seed(&mut self, seed: u64) -> Result<(), VmError> {
        let _ = seed;
        Ok(())
    }
    fn next_random_u64(&mut self) -> Result<u64, VmError>;
}
