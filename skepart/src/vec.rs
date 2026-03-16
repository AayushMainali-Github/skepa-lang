use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use crate::{RtError, RtResult, RtValue};

#[derive(Debug, Clone, PartialEq)]
pub struct RtVec(Rc<RefCell<Vec<RtValue>>>);

impl RtVec {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(Vec::new())))
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    pub fn borrow(&self) -> Ref<'_, Vec<RtValue>> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, Vec<RtValue>> {
        self.0.borrow_mut()
    }

    pub fn push(&self, value: RtValue) {
        self.0.borrow_mut().push(value);
    }

    pub fn get(&self, index: usize) -> RtResult<RtValue> {
        self.0
            .borrow()
            .get(index)
            .cloned()
            .ok_or_else(|| RtError::index_out_of_bounds(index, self.len()))
    }

    pub fn set(&self, index: usize, value: RtValue) -> RtResult<()> {
        let len = self.len();
        let mut items = self.0.borrow_mut();
        let slot = items
            .get_mut(index)
            .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
        *slot = value;
        Ok(())
    }

    pub fn delete(&self, index: usize) -> RtResult<RtValue> {
        let len = self.len();
        let mut items = self.0.borrow_mut();
        if index >= len {
            return Err(RtError::index_out_of_bounds(index, len));
        }
        Ok(items.remove(index))
    }
}

impl Default for RtVec {
    fn default() -> Self {
        Self::new()
    }
}
