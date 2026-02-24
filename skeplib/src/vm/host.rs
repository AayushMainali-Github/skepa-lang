use std::collections::{HashMap, VecDeque};
use std::io::{self, Write};

use crate::bytecode::Value;

use super::{BuiltinHost, VmError, VmErrorKind};

#[derive(Default)]
pub struct StdIoHost {
    rng_state: u64,
    next_vec_id: u64,
    vec_store: HashMap<u64, Vec<Value>>,
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

    fn vec_new(&mut self) -> Result<u64, VmError> {
        let id = self.next_vec_id;
        self.next_vec_id = self.next_vec_id.wrapping_add(1);
        self.vec_store.insert(id, Vec::new());
        Ok(id)
    }

    fn vec_len(&mut self, id: u64) -> Result<usize, VmError> {
        self.vec_store
            .get(&id)
            .map(|v| v.len())
            .ok_or_else(|| VmError::new(VmErrorKind::TypeMismatch, "invalid vec handle"))
    }

    fn vec_push(&mut self, id: u64, v: Value) -> Result<(), VmError> {
        let vec = self
            .vec_store
            .get_mut(&id)
            .ok_or_else(|| VmError::new(VmErrorKind::TypeMismatch, "invalid vec handle"))?;
        vec.push(v);
        Ok(())
    }

    fn vec_get(&mut self, id: u64, idx: i64) -> Result<Value, VmError> {
        let idx = usize::try_from(idx)
            .map_err(|_| VmError::new(VmErrorKind::IndexOutOfBounds, "vec index out of bounds"))?;
        let vec = self
            .vec_store
            .get(&id)
            .ok_or_else(|| VmError::new(VmErrorKind::TypeMismatch, "invalid vec handle"))?;
        vec.get(idx)
            .cloned()
            .ok_or_else(|| VmError::new(VmErrorKind::IndexOutOfBounds, "vec index out of bounds"))
    }

    fn vec_set(&mut self, id: u64, idx: i64, v: Value) -> Result<(), VmError> {
        let idx = usize::try_from(idx)
            .map_err(|_| VmError::new(VmErrorKind::IndexOutOfBounds, "vec index out of bounds"))?;
        let vec = self
            .vec_store
            .get_mut(&id)
            .ok_or_else(|| VmError::new(VmErrorKind::TypeMismatch, "invalid vec handle"))?;
        let slot = vec
            .get_mut(idx)
            .ok_or_else(|| VmError::new(VmErrorKind::IndexOutOfBounds, "vec index out of bounds"))?;
        *slot = v;
        Ok(())
    }

    fn vec_delete(&mut self, id: u64, idx: i64) -> Result<Value, VmError> {
        let idx = usize::try_from(idx)
            .map_err(|_| VmError::new(VmErrorKind::IndexOutOfBounds, "vec index out of bounds"))?;
        let vec = self
            .vec_store
            .get_mut(&id)
            .ok_or_else(|| VmError::new(VmErrorKind::TypeMismatch, "invalid vec handle"))?;
        if idx >= vec.len() {
            return Err(VmError::new(
                VmErrorKind::IndexOutOfBounds,
                "vec index out of bounds",
            ));
        }
        Ok(vec.remove(idx))
    }

    fn set_random_seed(&mut self, seed: u64) -> Result<(), VmError> {
        self.rng_state = seed;
        Ok(())
    }

    fn next_random_u64(&mut self) -> Result<u64, VmError> {
        // LCG step for deterministic pseudo-random sequence.
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        Ok(self.rng_state)
    }
}

#[derive(Default)]
pub struct TestHost {
    pub output: String,
    pub input: VecDeque<String>,
    pub rng_state: u64,
    pub next_vec_id: u64,
    pub vec_store: HashMap<u64, Vec<Value>>,
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

    fn vec_new(&mut self) -> Result<u64, VmError> {
        let id = self.next_vec_id;
        self.next_vec_id = self.next_vec_id.wrapping_add(1);
        self.vec_store.insert(id, Vec::new());
        Ok(id)
    }

    fn vec_len(&mut self, id: u64) -> Result<usize, VmError> {
        self.vec_store
            .get(&id)
            .map(|v| v.len())
            .ok_or_else(|| VmError::new(VmErrorKind::TypeMismatch, "invalid vec handle"))
    }

    fn vec_push(&mut self, id: u64, v: Value) -> Result<(), VmError> {
        let vec = self
            .vec_store
            .get_mut(&id)
            .ok_or_else(|| VmError::new(VmErrorKind::TypeMismatch, "invalid vec handle"))?;
        vec.push(v);
        Ok(())
    }

    fn vec_get(&mut self, id: u64, idx: i64) -> Result<Value, VmError> {
        let idx = usize::try_from(idx)
            .map_err(|_| VmError::new(VmErrorKind::IndexOutOfBounds, "vec index out of bounds"))?;
        let vec = self
            .vec_store
            .get(&id)
            .ok_or_else(|| VmError::new(VmErrorKind::TypeMismatch, "invalid vec handle"))?;
        vec.get(idx)
            .cloned()
            .ok_or_else(|| VmError::new(VmErrorKind::IndexOutOfBounds, "vec index out of bounds"))
    }

    fn vec_set(&mut self, id: u64, idx: i64, v: Value) -> Result<(), VmError> {
        let idx = usize::try_from(idx)
            .map_err(|_| VmError::new(VmErrorKind::IndexOutOfBounds, "vec index out of bounds"))?;
        let vec = self
            .vec_store
            .get_mut(&id)
            .ok_or_else(|| VmError::new(VmErrorKind::TypeMismatch, "invalid vec handle"))?;
        let slot = vec
            .get_mut(idx)
            .ok_or_else(|| VmError::new(VmErrorKind::IndexOutOfBounds, "vec index out of bounds"))?;
        *slot = v;
        Ok(())
    }

    fn vec_delete(&mut self, id: u64, idx: i64) -> Result<Value, VmError> {
        let idx = usize::try_from(idx)
            .map_err(|_| VmError::new(VmErrorKind::IndexOutOfBounds, "vec index out of bounds"))?;
        let vec = self
            .vec_store
            .get_mut(&id)
            .ok_or_else(|| VmError::new(VmErrorKind::TypeMismatch, "invalid vec handle"))?;
        if idx >= vec.len() {
            return Err(VmError::new(
                VmErrorKind::IndexOutOfBounds,
                "vec index out of bounds",
            ));
        }
        Ok(vec.remove(idx))
    }

    fn set_random_seed(&mut self, seed: u64) -> Result<(), VmError> {
        self.rng_state = seed;
        Ok(())
    }

    fn next_random_u64(&mut self) -> Result<u64, VmError> {
        // Match StdIoHost sequence exactly for reproducible tests.
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        Ok(self.rng_state)
    }
}
