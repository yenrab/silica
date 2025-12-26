use std::fmt;

/// Core result type used throughout the compiler
pub type Result<T> = std::result::Result<T, CompilerError>;

/// Source location information for error reporting
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl SourceLocation {
    pub fn new(file: String, line: usize, column: usize, offset: usize) -> Self {
        SourceLocation {
            file,
            line,
            column,
            offset,
        }
    }

    pub fn unknown() -> Self {
        SourceLocation {
            file: "<unknown>".to_string(),
            line: 0,
            column: 0,
            offset: 0,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// Main compiler error type
#[derive(Debug)]
pub enum CompilerError {
    LexerError {
        location: SourceLocation,
        message: String,
    },

    ParseError {
        location: SourceLocation,
        message: String,
    },

    TypeError {
        location: SourceLocation,
        message: String,
    },

    EffectError {
        location: SourceLocation,
        message: String,
    },

    CodegenError { message: String },

    IoError(std::io::Error),

    Utf8Error(std::string::FromUtf8Error),

    NotImplemented(String),

    InternalError(String),
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompilerError::LexerError { location, message } => {
                write!(f, "Lexer error at {}: {}", location, message)
            }
            CompilerError::ParseError { location, message } => {
                write!(f, "Parse error at {}: {}", location, message)
            }
            CompilerError::TypeError { location, message } => {
                write!(f, "Type error at {}: {}", location, message)
            }
            CompilerError::EffectError { location, message } => {
                write!(f, "Effect error at {}: {}", location, message)
            }
            CompilerError::CodegenError { message } => {
                write!(f, "Code generation error: {}", message)
            }
            CompilerError::IoError(err) => {
                write!(f, "IO error: {}", err)
            }
            CompilerError::Utf8Error(err) => {
                write!(f, "UTF-8 error: {}", err)
            }
            CompilerError::NotImplemented(msg) => {
                write!(f, "Not implemented: {}", msg)
            }
            CompilerError::InternalError(msg) => {
                write!(f, "Internal compiler error: {}", msg)
            }
        }
    }
}

impl std::error::Error for CompilerError {}

impl From<std::io::Error> for CompilerError {
    fn from(err: std::io::Error) -> Self {
        CompilerError::IoError(err)
    }
}

impl From<std::string::FromUtf8Error> for CompilerError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        CompilerError::Utf8Error(err)
    }
}

impl CompilerError {
    pub fn lexer_error(location: SourceLocation, message: String) -> Self {
        CompilerError::LexerError { location, message }
    }

    pub fn parse_error(location: SourceLocation, message: String) -> Self {
        CompilerError::ParseError { location, message }
    }

    pub fn type_error(location: SourceLocation, message: String) -> Self {
        CompilerError::TypeError { location, message }
    }

    pub fn effect_error(location: SourceLocation, message: String) -> Self {
        CompilerError::EffectError { location, message }
    }

    pub fn codegen_error(message: String) -> Self {
        CompilerError::CodegenError { message }
    }

    pub fn internal_error(message: String) -> Self {
        CompilerError::InternalError(message)
    }
}

/// Convenience functions for creating errors
pub fn lexer_error<T>(location: SourceLocation, message: String) -> Result<T> {
    Err(CompilerError::lexer_error(location, message))
}

pub fn parse_error<T>(location: SourceLocation, message: String) -> Result<T> {
    Err(CompilerError::parse_error(location, message))
}

pub fn type_error<T>(location: SourceLocation, message: String) -> Result<T> {
    Err(CompilerError::type_error(location, message))
}

pub fn effect_error<T>(location: SourceLocation, message: String) -> Result<T> {
    Err(CompilerError::effect_error(location, message))
}

pub fn codegen_error<T>(message: String) -> Result<T> {
    Err(CompilerError::codegen_error(message))
}

pub fn internal_error<T>(message: String) -> Result<T> {
    Err(CompilerError::internal_error(message))
}
