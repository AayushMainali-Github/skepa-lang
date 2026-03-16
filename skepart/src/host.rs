use crate::{RtResult, RtString};

pub trait RtHost {
    fn io_print(&mut self, text: &str) -> RtResult<()>;

    fn io_println(&mut self, text: &str) -> RtResult<()> {
        self.io_print(text)?;
        self.io_print("\n")
    }

    fn io_read_line(&mut self) -> RtResult<RtString> {
        Ok(RtString::from(""))
    }
}

#[derive(Default)]
pub struct NoopHost;

impl RtHost for NoopHost {
    fn io_print(&mut self, _text: &str) -> RtResult<()> {
        Ok(())
    }
}
