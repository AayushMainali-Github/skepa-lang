use crate::{RtHost, RtResult, RtValue};

pub fn print(host: &mut dyn RtHost, value: &RtValue) -> RtResult<()> {
    host.io_print(&display_value(value))
}

pub fn println(host: &mut dyn RtHost, value: &RtValue) -> RtResult<()> {
    host.io_println(&display_value(value))
}

pub fn read_line(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::String(host.io_read_line()?))
}

fn display_value(value: &RtValue) -> String {
    match value {
        RtValue::Int(value) => value.to_string(),
        RtValue::Float(value) => value.to_string(),
        RtValue::Bool(value) => value.to_string(),
        RtValue::String(value) => value.as_str().to_owned(),
        RtValue::Array(_) => "[array]".to_owned(),
        RtValue::Vec(_) => "[vec]".to_owned(),
        RtValue::Function(_) => "[function]".to_owned(),
        RtValue::Struct(value) => format!("[struct {}]", value.layout.name),
        RtValue::Unit => String::new(),
    }
}
