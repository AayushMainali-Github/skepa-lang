pub const DEFAULT_MAX_CALL_DEPTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmConfig {
    pub max_call_depth: usize,
    pub trace: bool,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
            trace: false,
        }
    }
}
