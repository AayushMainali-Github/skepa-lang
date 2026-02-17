use std::collections::VecDeque;
use std::io::{self, Write};

use super::{BuiltinHost, VmError, VmErrorKind};

#[derive(Default)]
pub struct StdIoHost {
    rng_state: u64,
}

impl BuiltinHost for StdIoHost {
    fn write(&mut self, s: &str, newline: bool) -> Result<(), VmError> {
        if newline {
            println!("{s}");
        } else {
            print!("{s}");
            io::stdout()
                .flush()
                .map_err(|e| VmError::new(VmErrorKind::HostError, e.to_string()))?;
        }
        Ok(())
    }

    fn read_line(&mut self) -> Result<String, VmError> {
        let mut buf = String::new();
        io::stdin()
            .read_line(&mut buf)
            .map_err(|e| VmError::new(VmErrorKind::HostError, e.to_string()))?;
        while buf.ends_with('\n') || buf.ends_with('\r') {
            buf.pop();
        }
        Ok(buf)
    }

    fn set_random_seed(&mut self, seed: u64) -> Result<(), VmError> {
        self.rng_state = seed;
        Ok(())
    }
}

#[derive(Default)]
pub struct TestHost {
    pub output: String,
    pub input: VecDeque<String>,
    pub rng_state: u64,
}

impl BuiltinHost for TestHost {
    fn write(&mut self, s: &str, newline: bool) -> Result<(), VmError> {
        self.output.push_str(s);
        if newline {
            self.output.push('\n');
        }
        Ok(())
    }

    fn read_line(&mut self) -> Result<String, VmError> {
        Ok(self.input.pop_front().unwrap_or_default())
    }

    fn set_random_seed(&mut self, seed: u64) -> Result<(), VmError> {
        self.rng_state = seed;
        Ok(())
    }
}
