use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::{RtError, RtResult, RtValue};

#[derive(Debug, Clone, Default)]
pub struct RtMap(Rc<RefCell<HashMap<String, RtValue>>>);

impl RtMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    pub fn has(&self, key: &str) -> bool {
        self.0.borrow().contains_key(key)
    }

    pub fn get(&self, key: &str) -> RtResult<RtValue> {
        self.0.borrow().get(key).cloned().ok_or_else(|| {
            RtError::new(
                crate::RtErrorKind::MissingField,
                format!("missing map key `{key}`"),
            )
        })
    }

    pub fn insert(&self, key: impl Into<String>, value: RtValue) {
        self.0.borrow_mut().insert(key.into(), value);
    }

    pub fn remove(&self, key: &str) -> RtResult<RtValue> {
        self.0.borrow_mut().remove(key).ok_or_else(|| {
            RtError::new(
                crate::RtErrorKind::MissingField,
                format!("missing map key `{key}`"),
            )
        })
    }
}

impl PartialEq for RtMap {
    fn eq(&self, other: &Self) -> bool {
        *self.0.borrow() == *other.0.borrow()
    }
}
