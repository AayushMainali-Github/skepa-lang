use std::sync::{Arc, Mutex, MutexGuard};

use crate::{RtError, RtResult, RtString, RtValue};

#[derive(Debug, Clone, PartialEq)]
enum RtVecRepr {
    Values(Vec<RtValue>),
    Ints(Vec<i64>),
    Floats(Vec<f64>),
    Bools(Vec<bool>),
    Strings(Vec<RtString>),
}

#[derive(Debug, Clone)]
pub struct RtVec(Arc<Mutex<RtVecRepr>>);

impl RtVec {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(RtVecRepr::Values(Vec::new()))))
    }

    pub fn len(&self) -> usize {
        match &*self.guard() {
            RtVecRepr::Values(items) => items.len(),
            RtVecRepr::Ints(items) => items.len(),
            RtVecRepr::Floats(items) => items.len(),
            RtVecRepr::Bools(items) => items.len(),
            RtVecRepr::Strings(items) => items.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&self, value: RtValue) {
        let mut repr = self.guard();
        match (&mut *repr, value) {
            (RtVecRepr::Values(items), RtValue::Int(value)) if items.is_empty() => {
                *repr = RtVecRepr::Ints(vec![value]);
            }
            (RtVecRepr::Values(items), RtValue::Float(value)) if items.is_empty() => {
                *repr = RtVecRepr::Floats(vec![value]);
            }
            (RtVecRepr::Values(items), RtValue::Bool(value)) if items.is_empty() => {
                *repr = RtVecRepr::Bools(vec![value]);
            }
            (RtVecRepr::Values(items), RtValue::String(value)) if items.is_empty() => {
                *repr = RtVecRepr::Strings(vec![value]);
            }
            (RtVecRepr::Values(items), value) => items.push(value),
            (RtVecRepr::Ints(items), RtValue::Int(value)) => items.push(value),
            (RtVecRepr::Floats(items), RtValue::Float(value)) => items.push(value),
            (RtVecRepr::Bools(items), RtValue::Bool(value)) => items.push(value),
            (RtVecRepr::Strings(items), RtValue::String(value)) => items.push(value),
            (repr, value) => {
                let mut values = Self::repr_to_values(repr);
                values.push(value);
                *repr = RtVecRepr::Values(values);
            }
        }
    }

    pub fn get(&self, index: usize) -> RtResult<RtValue> {
        match &*self.guard() {
            RtVecRepr::Values(items) => items
                .get(index)
                .cloned()
                .ok_or_else(|| RtError::index_out_of_bounds(index, items.len())),
            RtVecRepr::Ints(items) => items
                .get(index)
                .copied()
                .map(RtValue::Int)
                .ok_or_else(|| RtError::index_out_of_bounds(index, items.len())),
            RtVecRepr::Floats(items) => items
                .get(index)
                .copied()
                .map(RtValue::Float)
                .ok_or_else(|| RtError::index_out_of_bounds(index, items.len())),
            RtVecRepr::Bools(items) => items
                .get(index)
                .copied()
                .map(RtValue::Bool)
                .ok_or_else(|| RtError::index_out_of_bounds(index, items.len())),
            RtVecRepr::Strings(items) => items
                .get(index)
                .cloned()
                .map(RtValue::String)
                .ok_or_else(|| RtError::index_out_of_bounds(index, items.len())),
        }
    }

    pub fn set(&self, index: usize, value: RtValue) -> RtResult<()> {
        let mut repr = self.guard();
        match (&mut *repr, value) {
            (RtVecRepr::Values(items), value) => {
                let len = items.len();
                let slot = items
                    .get_mut(index)
                    .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
                *slot = value;
                Ok(())
            }
            (RtVecRepr::Ints(items), RtValue::Int(value)) => {
                let len = items.len();
                let slot = items
                    .get_mut(index)
                    .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
                *slot = value;
                Ok(())
            }
            (RtVecRepr::Floats(items), RtValue::Float(value)) => {
                let len = items.len();
                let slot = items
                    .get_mut(index)
                    .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
                *slot = value;
                Ok(())
            }
            (RtVecRepr::Bools(items), RtValue::Bool(value)) => {
                let len = items.len();
                let slot = items
                    .get_mut(index)
                    .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
                *slot = value;
                Ok(())
            }
            (RtVecRepr::Strings(items), RtValue::String(value)) => {
                let len = items.len();
                let slot = items
                    .get_mut(index)
                    .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
                *slot = value;
                Ok(())
            }
            (repr, value) => {
                let mut values = Self::repr_to_values(repr);
                let len = values.len();
                let slot = values
                    .get_mut(index)
                    .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
                *slot = value;
                *repr = RtVecRepr::Values(values);
                Ok(())
            }
        }
    }

    pub fn delete(&self, index: usize) -> RtResult<RtValue> {
        let mut repr = self.guard();
        match &mut *repr {
            RtVecRepr::Values(items) => {
                let len = items.len();
                if index >= len {
                    return Err(RtError::index_out_of_bounds(index, len));
                }
                Ok(items.remove(index))
            }
            RtVecRepr::Ints(items) => {
                let len = items.len();
                if index >= len {
                    return Err(RtError::index_out_of_bounds(index, len));
                }
                Ok(RtValue::Int(items.remove(index)))
            }
            RtVecRepr::Floats(items) => {
                let len = items.len();
                if index >= len {
                    return Err(RtError::index_out_of_bounds(index, len));
                }
                Ok(RtValue::Float(items.remove(index)))
            }
            RtVecRepr::Bools(items) => {
                let len = items.len();
                if index >= len {
                    return Err(RtError::index_out_of_bounds(index, len));
                }
                Ok(RtValue::Bool(items.remove(index)))
            }
            RtVecRepr::Strings(items) => {
                let len = items.len();
                if index >= len {
                    return Err(RtError::index_out_of_bounds(index, len));
                }
                Ok(RtValue::String(items.remove(index)))
            }
        }
    }

    fn repr_to_values(repr: &RtVecRepr) -> Vec<RtValue> {
        match repr {
            RtVecRepr::Values(items) => items.clone(),
            RtVecRepr::Ints(items) => items.iter().copied().map(RtValue::Int).collect(),
            RtVecRepr::Floats(items) => items.iter().copied().map(RtValue::Float).collect(),
            RtVecRepr::Bools(items) => items.iter().copied().map(RtValue::Bool).collect(),
            RtVecRepr::Strings(items) => items.iter().cloned().map(RtValue::String).collect(),
        }
    }

    fn guard(&self) -> MutexGuard<'_, RtVecRepr> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl PartialEq for RtVec {
    fn eq(&self, other: &Self) -> bool {
        *self.guard() == *other.guard()
    }
}

impl Default for RtVec {
    fn default() -> Self {
        Self::new()
    }
}
