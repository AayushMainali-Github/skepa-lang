use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, col: usize) -> Self {
        Self {
            start,
            end,
            line,
            col,
        }
    }

    pub fn merge(self, other: Span) -> Span {
        let (line, col) = if self.start <= other.start {
            (self.line, self.col)
        } else {
            (other.line, other.col)
        };

        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line,
            col,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            span,
        }
    }

    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.level {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
        };
        write!(
            f,
            "{level} at {}:{} ({}..{}): {}",
            self.span.line, self.span.col, self.span.start, self.span.end, self.message
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct DiagnosticBag {
    items: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn error(&mut self, message: impl Into<String>, span: Span) {
        self.push(Diagnostic::error(message, span));
    }

    pub fn error_expected_found(&mut self, expected_message: &str, found_label: &str, span: Span) {
        self.error(format_expected_found(expected_message, found_label), span);
    }

    pub fn warning(&mut self, message: impl Into<String>, span: Span) {
        self.push(Diagnostic::warning(message, span));
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.items
    }

    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.items
    }
}

pub fn format_expected_found(expected_message: &str, found_label: &str) -> String {
    format!("{expected_message}; found {found_label}")
}
