use std::rc::Rc;

use crate::{RtError, RtResult, RtString, RtValue};

#[derive(Debug, Clone, PartialEq)]
enum RtArrayRepr {
    Values(Rc<Vec<RtValue>>),
    Ints(Rc<Vec<i64>>),
    Floats(Rc<Vec<f64>>),
    Bools(Rc<Vec<bool>>),
    Strings(Rc<Vec<RtString>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RtArray(RtArrayRepr);

impl RtArray {
    pub fn new(items: Vec<RtValue>) -> Self {
        Self(Self::infer_repr(items))
    }

    pub fn repeat(value: RtValue, size: usize) -> Self {
        match value {
            RtValue::Int(v) => Self(RtArrayRepr::Ints(Rc::new(vec![v; size]))),
            RtValue::Float(v) => Self(RtArrayRepr::Floats(Rc::new(vec![v; size]))),
            RtValue::Bool(v) => Self(RtArrayRepr::Bools(Rc::new(vec![v; size]))),
            RtValue::String(v) => Self(RtArrayRepr::Strings(Rc::new(vec![v; size]))),
            other => Self::new(vec![other; size]),
        }
    }

    pub fn len(&self) -> usize {
        match &self.0 {
            RtArrayRepr::Values(items) => items.len(),
            RtArrayRepr::Ints(items) => items.len(),
            RtArrayRepr::Floats(items) => items.len(),
            RtArrayRepr::Bools(items) => items.len(),
            RtArrayRepr::Strings(items) => items.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> RtResult<RtValue> {
        match &self.0 {
            RtArrayRepr::Values(items) => items
                .get(index)
                .cloned()
                .ok_or_else(|| RtError::index_out_of_bounds(index, items.len())),
            RtArrayRepr::Ints(items) => items
                .get(index)
                .copied()
                .map(RtValue::Int)
                .ok_or_else(|| RtError::index_out_of_bounds(index, items.len())),
            RtArrayRepr::Floats(items) => items
                .get(index)
                .copied()
                .map(RtValue::Float)
                .ok_or_else(|| RtError::index_out_of_bounds(index, items.len())),
            RtArrayRepr::Bools(items) => items
                .get(index)
                .copied()
                .map(RtValue::Bool)
                .ok_or_else(|| RtError::index_out_of_bounds(index, items.len())),
            RtArrayRepr::Strings(items) => items
                .get(index)
                .cloned()
                .map(RtValue::String)
                .ok_or_else(|| RtError::index_out_of_bounds(index, items.len())),
        }
    }

    pub fn set(&mut self, index: usize, value: RtValue) -> RtResult<()> {
        match (&mut self.0, value) {
            (RtArrayRepr::Values(items), value) => {
                let items = Rc::make_mut(items);
                let len = items.len();
                let slot = items
                    .get_mut(index)
                    .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
                *slot = value;
                Ok(())
            }
            (RtArrayRepr::Ints(items), RtValue::Int(value)) => {
                let items = Rc::make_mut(items);
                let len = items.len();
                let slot = items
                    .get_mut(index)
                    .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
                *slot = value;
                Ok(())
            }
            (RtArrayRepr::Floats(items), RtValue::Float(value)) => {
                let items = Rc::make_mut(items);
                let len = items.len();
                let slot = items
                    .get_mut(index)
                    .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
                *slot = value;
                Ok(())
            }
            (RtArrayRepr::Bools(items), RtValue::Bool(value)) => {
                let items = Rc::make_mut(items);
                let len = items.len();
                let slot = items
                    .get_mut(index)
                    .ok_or_else(|| RtError::index_out_of_bounds(index, len))?;
                *slot = value;
                Ok(())
            }
            (RtArrayRepr::Strings(items), RtValue::String(value)) => {
                let items = Rc::make_mut(items);
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
                *repr = RtArrayRepr::Values(Rc::new(values));
                Ok(())
            }
        }
    }

    pub fn items(&self) -> Vec<RtValue> {
        Self::repr_to_values(&self.0)
    }

    pub fn iter(&self) -> impl Iterator<Item = RtValue> {
        self.items().into_iter()
    }

    fn infer_repr(items: Vec<RtValue>) -> RtArrayRepr {
        if items.iter().all(|item| matches!(item, RtValue::Int(_))) {
            return RtArrayRepr::Ints(Rc::new(
                items
                    .into_iter()
                    .map(|item| match item {
                        RtValue::Int(value) => value,
                        _ => unreachable!(),
                    })
                    .collect(),
            ));
        }
        if items.iter().all(|item| matches!(item, RtValue::Float(_))) {
            return RtArrayRepr::Floats(Rc::new(
                items
                    .into_iter()
                    .map(|item| match item {
                        RtValue::Float(value) => value,
                        _ => unreachable!(),
                    })
                    .collect(),
            ));
        }
        if items.iter().all(|item| matches!(item, RtValue::Bool(_))) {
            return RtArrayRepr::Bools(Rc::new(
                items
                    .into_iter()
                    .map(|item| match item {
                        RtValue::Bool(value) => value,
                        _ => unreachable!(),
                    })
                    .collect(),
            ));
        }
        if items.iter().all(|item| matches!(item, RtValue::String(_))) {
            return RtArrayRepr::Strings(Rc::new(
                items
                    .into_iter()
                    .map(|item| match item {
                        RtValue::String(value) => value,
                        _ => unreachable!(),
                    })
                    .collect(),
            ));
        }
        RtArrayRepr::Values(Rc::new(items))
    }

    fn repr_to_values(repr: &RtArrayRepr) -> Vec<RtValue> {
        match repr {
            RtArrayRepr::Values(items) => items.as_ref().clone(),
            RtArrayRepr::Ints(items) => items.iter().copied().map(RtValue::Int).collect(),
            RtArrayRepr::Floats(items) => items.iter().copied().map(RtValue::Float).collect(),
            RtArrayRepr::Bools(items) => items.iter().copied().map(RtValue::Bool).collect(),
            RtArrayRepr::Strings(items) => items.iter().cloned().map(RtValue::String).collect(),
        }
    }
}
