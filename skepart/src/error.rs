use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RtErrorKind {
    DivisionByZero,
    IndexOutOfBounds,
    TypeMismatch,
    MissingField,
    InvalidArgument,
    Io,
    Process,
    UnsupportedBuiltin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtError {
    pub kind: RtErrorKind,
    pub message: String,
}

pub type RtResult<T> = Result<T, RtError>;

impl RtError {
    pub fn new(kind: RtErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn type_mismatch(message: impl Into<String>) -> Self {
        Self::new(RtErrorKind::TypeMismatch, message)
    }

    pub fn index_out_of_bounds(index: usize, len: usize) -> Self {
        Self::new(
            RtErrorKind::IndexOutOfBounds,
            format!("index {index} out of bounds for length {len}"),
        )
    }

    pub fn unsupported_builtin(name: impl Into<String>) -> Self {
        Self::new(
            RtErrorKind::UnsupportedBuiltin,
            format!("unsupported builtin `{}`", name.into()),
        )
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(RtErrorKind::Io, message)
    }

    pub fn process(message: impl Into<String>) -> Self {
        Self::new(RtErrorKind::Process, message)
    }
}

impl fmt::Display for RtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for RtError {}
