use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::{RtError, RtResult, RtValue};

#[derive(Debug, Clone, Default)]
pub struct RtMap(Arc<Mutex<HashMap<String, RtValue>>>);

impl RtMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.guard().len()
    }

    pub fn is_empty(&self) -> bool {
        self.guard().is_empty()
    }

    pub fn has(&self, key: &str) -> bool {
        self.guard().contains_key(key)
    }

    pub fn get(&self, key: &str) -> RtResult<RtValue> {
        self.guard().get(key).cloned().ok_or_else(|| {
            RtError::new(
                crate::RtErrorKind::MissingField,
                format!("missing map key `{key}`"),
            )
        })
    }

    pub fn insert(&self, key: impl Into<String>, value: RtValue) {
        self.guard().insert(key.into(), value);
    }

    pub fn remove(&self, key: &str) -> RtResult<RtValue> {
        self.guard().remove(key).ok_or_else(|| {
            RtError::new(
                crate::RtErrorKind::MissingField,
                format!("missing map key `{key}`"),
            )
        })
    }

    fn guard(&self) -> MutexGuard<'_, HashMap<String, RtValue>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl PartialEq for RtMap {
    fn eq(&self, other: &Self) -> bool {
        if Arc::ptr_eq(&self.0, &other.0) {
            return true;
        }
        *self.guard() == *other.guard()
    }
}
