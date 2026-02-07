// Parser error types with helpful error messages
// Provides context, line/column info, and suggestions for common mistakes

use std::fmt;

/// Detailed parse error with location and context
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Error message
    pub message: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Surrounding context (a few lines around the error)
    pub context: String,
    /// Optional suggestion for fixing the error
    pub suggestion: Option<String>,
    /// The kind of error
    pub kind: ParseErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// YAML syntax error
    YamlSyntax,
    /// Invalid schema (wrong types, missing fields)
    InvalidSchema,
    /// Unknown field
    UnknownField,
    /// Invalid value
    InvalidValue,
    /// Template resolution error
    TemplateError,
    /// Expression syntax error
    ExpressionError,
    /// IO error (file not found, etc.)
    IoError,
    /// Validation error (semantic)
    ValidationError,
}

impl ParseError {
    pub fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
            context: String::new(),
            suggestion: None,
            kind: ParseErrorKind::InvalidSchema,
        }
    }

    pub fn yaml_error(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
            context: String::new(),
            suggestion: None,
            kind: ParseErrorKind::YamlSyntax,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = context.into();
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_kind(mut self, kind: ParseErrorKind) -> Self {
        self.kind = kind;
        self
    }

    /// Create context from source content
    pub fn with_source_context(mut self, source: &str, context_lines: usize) -> Self {
        let lines: Vec<&str> = source.lines().collect();
        let start = self.line.saturating_sub(context_lines + 1);
        let end = (self.line + context_lines).min(lines.len());

        let mut context = String::new();
        for (i, line) in lines.iter().enumerate().take(end).skip(start) {
            let line_num = i + 1;
            let prefix = if line_num == self.line { ">" } else { " " };
            context.push_str(&format!("{} {:4} | {}\n", prefix, line_num, line));

            // Add column indicator for error line
            if line_num == self.line && self.column > 0 {
                let indicator = " ".repeat(self.column + 7) + "^";
                context.push_str(&format!("       | {}\n", indicator));
            }
        }

        self.context = context;
        self
    }

    /// Create from serde_yaml error
    pub fn from_yaml_error(err: &serde_yaml::Error, source: &str) -> Self {
        let location = err.location();
        let (line, column) = location
            .map(|loc| (loc.line(), loc.column()))
            .unwrap_or((1, 1));

        let message = format_yaml_error_message(err);
        let suggestion = suggest_yaml_fix(err, source, line);

        ParseError::yaml_error(message, line, column)
            .with_source_context(source, 2)
            .with_suggestion_opt(suggestion)
    }

    fn with_suggestion_opt(mut self, suggestion: Option<String>) -> Self {
        self.suggestion = suggestion;
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "error: {}", self.message)?;
        writeln!(f, "  --> line {}:{}", self.line, self.column)?;

        if !self.context.is_empty() {
            writeln!(f)?;
            write!(f, "{}", self.context)?;
        }

        if let Some(suggestion) = &self.suggestion {
            writeln!(f)?;
            writeln!(f, "help: {}", suggestion)?;
        }

        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// Format serde_yaml error message into something more readable
fn format_yaml_error_message(err: &serde_yaml::Error) -> String {
    let msg = err.to_string();

    // Clean up common serde_yaml error patterns
    if msg.contains("missing field") {
        if let Some(field) = extract_field_name(&msg, "missing field `", "`") {
            return format!("missing required field '{}'", field);
        }
    }

    if msg.contains("unknown field") {
        if let Some(field) = extract_field_name(&msg, "unknown field `", "`") {
            if let Some(expected) = extract_expected_fields(&msg) {
                return format!(
                    "unknown field '{}', expected one of: {}",
                    field,
                    expected.join(", ")
                );
            }
            return format!("unknown field '{}'", field);
        }
    }

    if msg.contains("invalid type") {
        return format_invalid_type_error(&msg);
    }

    // Return original if no pattern matched
    msg
}

fn extract_field_name(msg: &str, prefix: &str, suffix: &str) -> Option<String> {
    let start = msg.find(prefix)? + prefix.len();
    let end = msg[start..].find(suffix)? + start;
    Some(msg[start..end].to_string())
}

fn extract_expected_fields(msg: &str) -> Option<Vec<String>> {
    let start = msg.find("expected one of ")? + "expected one of ".len();
    let fields_str = &msg[start..];
    let end = fields_str.find(" at").unwrap_or(fields_str.len());
    let fields: Vec<String> = fields_str[..end]
        .split(", ")
        .map(|s| s.trim_matches('`').to_string())
        .collect();
    Some(fields)
}

fn format_invalid_type_error(msg: &str) -> String {
    // Extract what was expected and what was found
    if let (Some(expected), Some(found)) = (
        extract_field_name(msg, "expected ", ","),
        extract_field_name(msg, "found ", " at"),
    ) {
        return format!("expected {}, but found {}", expected, found);
    }
    msg.to_string()
}

/// Suggest fixes for common YAML errors
fn suggest_yaml_fix(err: &serde_yaml::Error, source: &str, line: usize) -> Option<String> {
    let msg = err.to_string();
    let lines: Vec<&str> = source.lines().collect();
    let error_line = lines.get(line.saturating_sub(1)).unwrap_or(&"");

    // Suggest fixes for common mistakes
    if msg.contains("missing field `steps`") {
        return Some(
            "jobs must have a 'steps' field. Add steps to define what the job should do."
                .to_string(),
        );
    }

    if msg.contains("missing field `job`") && msg.contains("missing field `deployment`") {
        return Some(
            "each job needs either 'job:' or 'deployment:' to define its identifier".to_string(),
        );
    }

    if msg.contains("unknown field `script`") && error_line.contains("script:") {
        return Some("'script:' should be at the step level, not nested inside another key. Check your indentation.".to_string());
    }

    // Indentation errors
    if msg.contains("expected") && msg.contains("found") && error_line.starts_with('\t') {
        return Some(
            "YAML prefers spaces over tabs for indentation. Replace tabs with spaces.".to_string(),
        );
    }

    // Common typos
    let typo_suggestions = [
        ("dependson", "dependsOn"),
        ("displayname", "displayName"),
        ("vmimage", "vmImage"),
        ("workingdirectory", "workingDirectory"),
        (
            "continueOnError",
            "continueOnError (note: lowercase 'n' in 'on')",
        ),
        ("timeout", "timeoutInMinutes"),
    ];

    let lower_line = error_line.to_lowercase();
    for (typo, correct) in typo_suggestions {
        if lower_line.contains(typo) {
            return Some(format!("did you mean '{}'?", correct));
        }
    }

    None
}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Validation error for semantic checks
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    pub path: String,
    pub suggestion: Option<String>,
}

impl ValidationError {
    pub fn new(message: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: path.into(),
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "validation error at '{}': {}", self.path, self.message)?;
        if let Some(suggestion) = &self.suggestion {
            write!(f, " ({})", suggestion)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_display() {
        let err = ParseError::new("missing required field 'steps'", 10, 5)
            .with_context("   9 | jobs:\n> 10 |   - job: Build\n  11 |     pool: ubuntu-latest")
            .with_suggestion("add 'steps:' to define what the job should do");

        let output = format!("{}", err);
        assert!(output.contains("missing required field"));
        assert!(output.contains("line 10:5"));
        assert!(output.contains("help:"));
    }

    #[test]
    fn test_parse_error_with_source_context() {
        let source = r#"trigger:
  - main

pool:
  vmImage: ubuntu-latest

jobs:
  - job: Build
    displayName: Build Job"#;

        let err =
            ParseError::new("missing required field 'steps'", 8, 5).with_source_context(source, 2);

        assert!(err.context.contains("> "));
        assert!(err.context.contains("job: Build"));
    }

    #[test]
    fn test_extract_field_name() {
        let msg = "missing field `steps` at line 10";
        assert_eq!(
            extract_field_name(msg, "missing field `", "`"),
            Some("steps".to_string())
        );
    }
}
