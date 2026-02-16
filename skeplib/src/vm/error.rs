use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmErrorKind {
    UnknownFunction,
    ArityMismatch,
    StackUnderflow,
    TypeMismatch,
    InvalidLocal,
    DivisionByZero,
    UnknownBuiltin,
    HostError,
    StackOverflow,
    IndexOutOfBounds,
}

impl VmErrorKind {
    pub fn code(self) -> &'static str {
        match self {
            VmErrorKind::UnknownFunction => "E-VM-UNKNOWN-FUNCTION",
            VmErrorKind::ArityMismatch => "E-VM-ARITY",
            VmErrorKind::StackUnderflow => "E-VM-STACK-UNDERFLOW",
            VmErrorKind::TypeMismatch => "E-VM-TYPE",
            VmErrorKind::InvalidLocal => "E-VM-INVALID-LOCAL",
            VmErrorKind::DivisionByZero => "E-VM-DIV-ZERO",
            VmErrorKind::UnknownBuiltin => "E-VM-UNKNOWN-BUILTIN",
            VmErrorKind::HostError => "E-VM-HOST",
            VmErrorKind::StackOverflow => "E-VM-STACK-OVERFLOW",
            VmErrorKind::IndexOutOfBounds => "E-VM-INDEX-OOB",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VmError {
    pub kind: VmErrorKind,
    pub message: String,
}

impl VmError {
    pub(crate) fn new(kind: VmErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
