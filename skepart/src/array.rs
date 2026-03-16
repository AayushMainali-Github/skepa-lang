use std::rc::Rc;

use crate::{RtError, RtResult, RtValue};

#[derive(Debug, Clone, PartialEq)]
pub struct RtArray(Rc<Vec<RtValue>>);

impl RtArray {
    pub fn new(items: Vec<RtValue>) -> Self {
        Self(Rc::new(items))
    }

    pub fn repeat(value: RtValue, size: usize) -> Self {
        Self::new(vec![value; size])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, index: usize) -> RtResult<RtValue> {
        self.0
            .get(index)
            .cloned()
            .ok_or_else(|| RtError::index_out_of_bounds(index, self.len()))
    }

    pub fn set(&mut self, index: usize, value: RtValue) -> RtResult<()> {
        let items = Rc::make_mut(&mut self.0);
        let len = items.len();
        let slot = items
            .get_mut(index)
            .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
        *slot = value;
        Ok(())
    }

    pub fn items(&self) -> &[RtValue] {
        &self.0
    }

    pub fn iter(&self) -> impl Iterator<Item = &RtValue> {
        self.0.iter()
    }
}
