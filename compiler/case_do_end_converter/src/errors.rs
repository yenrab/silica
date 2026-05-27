/*
   Copyright 2026 Lee Scott Barney

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

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

/// Error severity level
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorSeverity::Error => write!(f, "error"),
            ErrorSeverity::Warning => write!(f, "warning"),
            ErrorSeverity::Info => write!(f, "info"),
        }
    }
}

/// Specification reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecificationReference {
    pub section: String,  // e.g., "§6.2.1"
    pub title: Option<String>,
}

/// Surrounding code context
#[derive(Debug, Clone)]
pub struct SurroundingCode {
    pub before: Vec<String>,  // Lines before error
    pub error_line: String,   // Line containing error
    pub after: Vec<String>,   // Lines after error
}

/// Type information for error context
#[derive(Debug, Clone)]
pub enum TypeInfo {
    Simple { name: String },
    Tuple { elements: Vec<TypeInfo> },
    Record { fields: Vec<(String, TypeInfo)> },
    Function { parameters: Vec<TypeInfo>, return_type: Box<TypeInfo> },
    Named { name: String },
    Other { description: String },
}

/// Expected vs actual values
#[derive(Debug, Clone)]
pub struct ExpectedActual {
    pub expected: String,
    pub actual: String,
    pub expected_type: Option<TypeInfo>,
    pub actual_type: Option<TypeInfo>,
}

/// Error suggestion
#[derive(Debug, Clone)]
pub struct ErrorSuggestion {
    pub description: String,
    pub code_example: Option<String>,
    pub fix: Option<FixDescription>,
}

/// Fix description for automated tools
#[derive(Debug, Clone)]
pub struct FixDescription {
    pub action: String,  // "insert", "replace", "delete"
    pub position: SourceLocation,
    pub old_text: Option<String>,
    pub new_text: Option<String>,
}

/// Error metadata for structured error information
#[derive(Debug, Clone)]
pub struct ErrorMetadata {
    pub error_code: String,  // e.g., "E2001"
    pub severity: ErrorSeverity,
    pub specification: Option<SpecificationReference>,
    pub surrounding_code: Option<SurroundingCode>,
    pub expected_actual: Option<ExpectedActual>,
    pub ast_node: Option<String>,
    pub suggestions: Vec<ErrorSuggestion>,
    pub related_errors: Vec<String>,  // Related error codes
}

/// Main compiler error type
#[derive(Debug)]
pub enum CompilerError {
    LexerError {
        location: SourceLocation,
        message: String,
        metadata: ErrorMetadata,
    },

    ParseError {
        location: SourceLocation,
        message: String,
        metadata: ErrorMetadata,
    },

    TypeError {
        location: SourceLocation,
        message: String,
        metadata: ErrorMetadata,
    },

    EffectError {
        location: SourceLocation,
        message: String,
        metadata: ErrorMetadata,
    },

    CodegenError {
        message: String,
        location: Option<SourceLocation>,
        metadata: ErrorMetadata,
    },

    IoError(std::io::Error),

    Utf8Error(std::string::FromUtf8Error),

    NotImplemented {
        message: String,
        metadata: ErrorMetadata,
    },

    InternalError {
        message: String,
        metadata: ErrorMetadata,
    },
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (error_type, location, message, metadata) = match self {
            CompilerError::LexerError { location, message, metadata } => {
                ("LexerError", Some(location), message, Some(metadata))
            }
            CompilerError::ParseError { location, message, metadata } => {
                ("ParseError", Some(location), message, Some(metadata))
            }
            CompilerError::TypeError { location, message, metadata } => {
                ("TypeError", Some(location), message, Some(metadata))
            }
            CompilerError::EffectError { location, message, metadata } => {
                ("EffectError", Some(location), message, Some(metadata))
            }
            CompilerError::CodegenError { message, location, metadata } => {
                ("CodegenError", location.as_ref(), message, Some(metadata))
            }
            CompilerError::IoError(err) => {
                // Convert IO error to InternalError with metadata
                let metadata = ErrorMetadataBuilder::new("E9003".to_string())
                    .severity(ErrorSeverity::Error)
                    .build();
                return write!(f, "IO error: {}\n\n<!-- SILICA-ERROR-METADATA\n{}\n-->", err, format_metadata_json(&metadata, None));
            }
            CompilerError::Utf8Error(err) => {
                // Convert UTF-8 error to InternalError with metadata
                let metadata = ErrorMetadataBuilder::new("E9004".to_string())
                    .severity(ErrorSeverity::Error)
                    .build();
                return write!(f, "UTF-8 error: {}\n\n<!-- SILICA-ERROR-METADATA\n{}\n-->", err, format_metadata_json(&metadata, None));
            }
            CompilerError::NotImplemented { message, metadata } => {
                ("NotImplemented", None, message, Some(metadata))
            }
            CompilerError::InternalError { message, metadata } => {
                ("InternalError", None, message, Some(metadata))
            }
        };

        // Write human-readable error message
        let metadata = metadata.expect("Metadata must always be present");
        if let Some(loc) = location {
            if !metadata.error_code.is_empty() {
                write!(f, "{} error at {} [{}]\n\n", error_type, loc, metadata.error_code)?;
            } else {
                write!(f, "{} error at {}: ", error_type, loc)?;
            }
        } else {
            if !metadata.error_code.is_empty() {
                write!(f, "{} [{}]: ", error_type, metadata.error_code)?;
            } else {
                write!(f, "{}: ", error_type)?;
            }
        }
        write!(f, "{}", message)?;

        // Add context if available
        {
            if let Some(ref code) = metadata.surrounding_code {
                write!(f, "\n\nContext:\n")?;
                for (i, line) in code.before.iter().enumerate() {
                    let line_num = if let Some(loc) = location {
                        loc.line.saturating_sub(code.before.len() - i)
                    } else {
                        0
                    };
                    write!(f, "{:3}| {}\n", line_num, line)?;
                }
                if let Some(loc) = location {
                    write!(f, "{:3}| {}", loc.line, code.error_line)?;
                    // Add caret indicator
                    let caret_pos = if loc.column > 0 && loc.column <= code.error_line.len() {
                        loc.column - 1
                    } else {
                        code.error_line.len()
                    };
                    write!(f, "\n    {:width$}^\n", "", width = caret_pos)?;
                } else {
                    write!(f, "   >| {}\n", code.error_line)?;
                }
                for (i, line) in code.after.iter().enumerate() {
                    let line_num = if let Some(loc) = location {
                        loc.line + i + 1
                    } else {
                        0
                    };
                    write!(f, "{:3}| {}\n", line_num, line)?;
                }
            }

            // Add expected/actual if available
            if let Some(ref ea) = metadata.expected_actual {
                write!(f, "\nExpected: {}\n", ea.expected)?;
                write!(f, "Actual: {}\n", ea.actual)?;
            }

            // Add suggestions
            if !metadata.suggestions.is_empty() {
                write!(f, "\nSuggestions:\n")?;
                for (i, suggestion) in metadata.suggestions.iter().enumerate() {
                    write!(f, "  {}. {}\n", i + 1, suggestion.description)?;
                    if let Some(ref example) = suggestion.code_example {
                        write!(f, "     Example: {}\n", example)?;
                    }
                }
            }

            // Add specification reference
            if let Some(ref spec) = metadata.specification {
                // Strip any existing "spec:" prefix, then add it back to ensure consistent format
                let clean_section = if spec.section.starts_with("spec:") {
                    spec.section.strip_prefix("spec:").unwrap_or(&spec.section)
                } else {
                    &spec.section
                };
                write!(f, "\nSee specification: spec:{}\n", clean_section)?;
            }

            // Write JSON-LD metadata in HTML comment
            write!(f, "\n<!-- SILICA-ERROR-METADATA\n{}\n-->", format_metadata_json(&metadata, location))?;
        }

        Ok(())
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
    /// Create a default metadata for errors when full context isn't available
    fn default_metadata(error_code: &str, spec_section: Option<&str>) -> ErrorMetadata {
        let mut builder = ErrorMetadataBuilder::new(error_code.to_string())
            .severity(ErrorSeverity::Error);
        if let Some(section) = spec_section {
            // Remove "spec:" prefix if present, store just "§X.Y"
            let clean_section = if section.starts_with("spec:") {
                section.strip_prefix("spec:").unwrap_or(section)
            } else {
                section
            };
            builder = builder.specification(clean_section.to_string(), None);
        }
        builder.build()
    }

    pub fn lexer_error(location: SourceLocation, message: String) -> Self {
        CompilerError::LexerError {
            location: location.clone(),
            message,
            metadata: Self::default_metadata("E0000", Some("§2")),  // Don't include "spec:" prefix here
        }
    }

    pub fn lexer_error_with_metadata(location: SourceLocation, message: String, metadata: ErrorMetadata) -> Self {
        CompilerError::LexerError {
            location,
            message,
            metadata,
        }
    }

    pub fn parse_error(location: SourceLocation, message: String) -> Self {
        CompilerError::ParseError {
            location: location.clone(),
            message,
            metadata: Self::default_metadata("E1000", Some("§3")),  // Don't include "spec:" prefix here
        }
    }

    pub fn parse_error_with_metadata(location: SourceLocation, message: String, metadata: ErrorMetadata) -> Self {
        CompilerError::ParseError {
            location,
            message,
            metadata,
        }
    }

    pub fn type_error(location: SourceLocation, message: String) -> Self {
        CompilerError::TypeError {
            location: location.clone(),
            message,
            metadata: Self::default_metadata("E2000", Some("§6")),  // Don't include "spec:" prefix here
        }
    }

    pub fn type_error_with_metadata(location: SourceLocation, message: String, metadata: ErrorMetadata) -> Self {
        CompilerError::TypeError {
            location,
            message,
            metadata,
        }
    }

    pub fn effect_error(location: SourceLocation, message: String) -> Self {
        CompilerError::EffectError {
            location: location.clone(),
            message,
            metadata: Self::default_metadata("E3000", Some("§8")),  // Don't include "spec:" prefix here
        }
    }

    pub fn effect_error_with_metadata(location: SourceLocation, message: String, metadata: ErrorMetadata) -> Self {
        CompilerError::EffectError {
            location,
            message,
            metadata,
        }
    }

    pub fn codegen_error(message: String) -> Self {
        CompilerError::CodegenError {
            message,
            location: None,
            metadata: Self::default_metadata("E4000", None),
        }
    }

    pub fn codegen_error_with_location(message: String, location: SourceLocation) -> Self {
        CompilerError::CodegenError {
            message,
            location: Some(location.clone()),
            metadata: Self::default_metadata("E4000", None),
        }
    }

    pub fn codegen_error_with_metadata(message: String, location: Option<SourceLocation>, metadata: ErrorMetadata) -> Self {
        CompilerError::CodegenError {
            message,
            location,
            metadata,
        }
    }

    pub fn internal_error(message: String) -> Self {
        CompilerError::InternalError {
            message,
            metadata: Self::default_metadata("E9000", None),
        }
    }

    pub fn internal_error_with_metadata(message: String, metadata: ErrorMetadata) -> Self {
        CompilerError::InternalError {
            message,
            metadata,
        }
    }
}

/// Extract surrounding code context from source
pub fn extract_surrounding_code(source: &str, location: &SourceLocation, context_lines: usize) -> Option<SurroundingCode> {
    if location.line == 0 || location.line > 10000 {
        return None;
    }

    let lines: Vec<&str> = source.lines().collect();
    if location.line > lines.len() {
        return None;
    }

    let line_index = location.line - 1;  // Convert to 0-based index
    let start_line = line_index.saturating_sub(context_lines);
    let end_line = std::cmp::min(line_index + context_lines + 1, lines.len());

    let before: Vec<String> = lines[start_line..line_index]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let error_line = lines[line_index].to_string();
    let after: Vec<String> = lines[line_index + 1..end_line]
        .iter()
        .map(|s| s.to_string())
        .collect();

    Some(SurroundingCode {
        before,
        error_line,
        after,
    })
}

/// Format type information as JSON
fn format_type_info_json(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Simple { name } => format!(r#"{{"type": "{}"}}"#, name),
        TypeInfo::Tuple { elements } => {
            let elements_str: Vec<String> = elements.iter().map(format_type_info_json).collect();
            format!(r#"{{"type": "tuple", "elements": [{}]}}"#, elements_str.join(", "))
        }
        TypeInfo::Record { fields } => {
            let fields_str: Vec<String> = fields.iter().map(|(name, ty)| {
                format!(r#""{}": {}"#, name, format_type_info_json(ty))
            }).collect();
            format!(r#"{{"type": "record", "fields": {{{}}}"}}"#, fields_str.join(", "))
        }
        TypeInfo::Function { parameters, return_type } => {
            let params_str: Vec<String> = parameters.iter().map(format_type_info_json).collect();
            format!(r#"{{"type": "function", "parameters": [{}], "returnType": {}}}"#,
                params_str.join(", "), format_type_info_json(return_type))
        }
        TypeInfo::Named { name } => format!(r#"{{"type": "named", "name": "{}"}}"#, name),
        TypeInfo::Other { description } => format!(r#"{{"type": "other", "description": "{}"}}"#, description),
    }
}

/// Format error metadata as JSON-LD
fn format_metadata_json(metadata: &ErrorMetadata, location: Option<&SourceLocation>) -> String {
    let mut json = String::from("{\n");
    json.push_str(r#"  "@context": "https://aalang.dev/silica-dev/error/","#);
    json.push_str(&format!("\n  \"errorCode\": \"{}\",", metadata.error_code));
    json.push_str(&format!("\n  \"errorType\": \"{}\",", metadata.severity));
    json.push_str(&format!("\n  \"severity\": \"{}\",", metadata.severity));

    if let Some(loc) = location {
        json.push_str(&format!(
            r#"
  "location": {{
    "file": "{}",
    "line": {},
    "column": {},
    "offset": {}
  }},"#,
            loc.file, loc.line, loc.column, loc.offset
        ));
    }

    if let Some(ref spec) = metadata.specification {
        json.push_str(&format!(
            r#"
  "specification": {{
    "section": "{}""#,
            spec.section
        ));
        if let Some(ref title) = spec.title {
            json.push_str(&format!(r#",
    "title": "{}""#, title));
        }
        json.push_str("\n  },");
    }

    if let Some(ref code) = metadata.surrounding_code {
        json.push_str(r#"
  "context": {
    "surroundingCode": {"#);
        json.push_str(&format!(
            r#"
      "before": [{}],
      "errorLine": "{}",
      "after": [{}]"#,
            code.before.iter().map(|l| format!("\"{}\"", l.replace("\"", "\\\""))).collect::<Vec<_>>().join(", "),
            code.error_line.replace("\"", "\\\""),
            code.after.iter().map(|l| format!("\"{}\"", l.replace("\"", "\\\""))).collect::<Vec<_>>().join(", ")
        ));
        json.push_str("\n    }");

        if let Some(ref ea) = metadata.expected_actual {
            json.push_str(&format!(
                r#",
    "expected": "{}",
    "actual": "{}""#,
                ea.expected.replace("\"", "\\\""),
                ea.actual.replace("\"", "\\\"")
            ));

            if let Some(ref exp_ty) = ea.expected_type {
                json.push_str(&format!(
                    r#",
    "types": {{
      "expected": {},
      "actual": {}}}"#,
                    format_type_info_json(exp_ty),
                    if let Some(ref act_ty) = ea.actual_type {
                        format_type_info_json(act_ty)
                    } else {
                        "null".to_string()
                    }
                ));
            }
        }

        if let Some(ref ast) = metadata.ast_node {
            json.push_str(&format!(r#",
    "astNode": "{}""#, ast));
        }

        json.push_str("\n  },");
    }

    if !metadata.suggestions.is_empty() {
        json.push_str(r#"
  "suggestions": ["#);
        let suggestions_str: Vec<String> = metadata.suggestions.iter().map(|s| {
            let mut sug = format!(r#"{{"description": "{}""#, s.description.replace("\"", "\\\""));
            if let Some(ref ex) = s.code_example {
                sug.push_str(&format!(r#", "codeExample": "{}""#, ex.replace("\"", "\\\"")));
            }
            if let Some(ref fix) = s.fix {
                sug.push_str(&format!(
                    r#", "fix": {{"action": "{}", "position": {{"line": {}, "column": {}}}"#,
                    fix.action, fix.position.line, fix.position.column
                ));
                if let Some(ref old) = fix.old_text {
                    sug.push_str(&format!(r#", "oldText": "{}""#, old.replace("\"", "\\\"")));
                }
                if let Some(ref new) = fix.new_text {
                    sug.push_str(&format!(r#", "newText": "{}""#, new.replace("\"", "\\\"")));
                }
                sug.push_str("}");
            }
            sug.push_str("}");
            sug
        }).collect();
        json.push_str(&suggestions_str.join(", "));
        json.push_str("\n  ],");
    }

    if !metadata.related_errors.is_empty() {
        json.push_str(&format!(
            r#"
  "relatedErrors": [{}]"#,
            metadata.related_errors.iter().map(|e| format!("\"{}\"", e)).collect::<Vec<_>>().join(", ")
        ));
    }

    json.push_str("\n}");
    json
}

/// Convenience functions for creating errors
/// Note: These functions create errors with default metadata. Use the _with_metadata variants for full control.
pub fn lexer_error<T>(location: SourceLocation, message: String) -> Result<T> {
    Err(CompilerError::lexer_error(location, message))
}

pub fn lexer_error_with_metadata<T>(location: SourceLocation, message: String, metadata: ErrorMetadata) -> Result<T> {
    Err(CompilerError::lexer_error_with_metadata(location, message, metadata))
}

pub fn parse_error<T>(location: SourceLocation, message: String) -> Result<T> {
    Err(CompilerError::parse_error(location, message))
}

pub fn parse_error_with_metadata<T>(location: SourceLocation, message: String, metadata: ErrorMetadata) -> Result<T> {
    Err(CompilerError::parse_error_with_metadata(location, message, metadata))
}

pub fn type_error<T>(location: SourceLocation, message: String) -> Result<T> {
    Err(CompilerError::type_error(location, message))
}

pub fn type_error_with_metadata<T>(location: SourceLocation, message: String, metadata: ErrorMetadata) -> Result<T> {
    Err(CompilerError::type_error_with_metadata(location, message, metadata))
}

pub fn effect_error<T>(location: SourceLocation, message: String) -> Result<T> {
    Err(CompilerError::effect_error(location, message))
}

pub fn effect_error_with_metadata<T>(location: SourceLocation, message: String, metadata: ErrorMetadata) -> Result<T> {
    Err(CompilerError::effect_error_with_metadata(location, message, metadata))
}

pub fn codegen_error<T>(message: String) -> Result<T> {
    Err(CompilerError::codegen_error(message))
}

pub fn codegen_error_with_location<T>(message: String, location: SourceLocation) -> Result<T> {
    Err(CompilerError::codegen_error_with_location(message, location))
}

pub fn codegen_error_with_metadata<T>(message: String, location: Option<SourceLocation>, metadata: ErrorMetadata) -> Result<T> {
    Err(CompilerError::codegen_error_with_metadata(message, location, metadata))
}

pub fn internal_error<T>(message: String) -> Result<T> {
    Err(CompilerError::internal_error(message))
}

pub fn internal_error_with_metadata<T>(message: String, metadata: ErrorMetadata) -> Result<T> {
    Err(CompilerError::internal_error_with_metadata(message, metadata))
}

/// Builder for error metadata
pub struct ErrorMetadataBuilder {
    error_code: String,
    severity: ErrorSeverity,
    specification: Option<SpecificationReference>,
    surrounding_code: Option<SurroundingCode>,
    expected_actual: Option<ExpectedActual>,
    ast_node: Option<String>,
    suggestions: Vec<ErrorSuggestion>,
    related_errors: Vec<String>,
}

impl ErrorMetadataBuilder {
    pub fn new(error_code: String) -> Self {
        ErrorMetadataBuilder {
            error_code,
            severity: ErrorSeverity::Error,
            specification: None,
            surrounding_code: None,
            expected_actual: None,
            ast_node: None,
            suggestions: Vec::new(),
            related_errors: Vec::new(),
        }
    }

    pub fn severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn specification(mut self, section: String, title: Option<String>) -> Self {
        self.specification = Some(SpecificationReference { section, title });
        self
    }

    pub fn surrounding_code(mut self, code: SurroundingCode) -> Self {
        self.surrounding_code = Some(code);
        self
    }

    pub fn expected_actual(mut self, expected: String, actual: String) -> Self {
        self.expected_actual = Some(ExpectedActual {
            expected,
            actual,
            expected_type: None,
            actual_type: None,
        });
        self
    }

    pub fn expected_actual_with_types(mut self, expected: String, actual: String, expected_type: TypeInfo, actual_type: TypeInfo) -> Self {
        self.expected_actual = Some(ExpectedActual {
            expected,
            actual,
            expected_type: Some(expected_type),
            actual_type: Some(actual_type),
        });
        self
    }

    pub fn ast_node(mut self, node: String) -> Self {
        self.ast_node = Some(node);
        self
    }

    pub fn suggestion(mut self, description: String) -> Self {
        self.suggestions.push(ErrorSuggestion {
            description,
            code_example: None,
            fix: None,
        });
        self
    }

    pub fn suggestion_with_example(mut self, description: String, code_example: String) -> Self {
        self.suggestions.push(ErrorSuggestion {
            description,
            code_example: Some(code_example),
            fix: None,
        });
        self
    }

    pub fn related_error(mut self, error_code: String) -> Self {
        self.related_errors.push(error_code);
        self
    }

    pub fn build(self) -> ErrorMetadata {
        ErrorMetadata {
            error_code: self.error_code,
            severity: self.severity,
            specification: self.specification,
            surrounding_code: self.surrounding_code,
            expected_actual: self.expected_actual,
            ast_node: self.ast_node,
            suggestions: self.suggestions,
            related_errors: self.related_errors,
        }
    }
}
