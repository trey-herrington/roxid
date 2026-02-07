// Expression Engine Lexer
// Tokenizes Azure DevOps expressions: ${{ }}, $[ ], and $(var)

use std::fmt;

/// Token types for Azure DevOps expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Null,
    True,
    False,
    Number(f64),
    String(String),

    // Identifiers and references
    Identifier(String),

    // Operators
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    Percent,  // %
    Eq,       // ==
    Ne,       // !=
    Lt,       // <
    Le,       // <=
    Gt,       // >
    Ge,       // >=
    And,      // &&
    Or,       // ||
    Not,      // !
    Dot,      // .
    Comma,    // ,
    Colon,    // :
    Question, // ?

    // Delimiters
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LBrace,   // {
    RBrace,   // }

    // End of input
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Null => write!(f, "null"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Number(n) => write!(f, "{}", n),
            Token::String(s) => write!(f, "'{}'", s),
            Token::Identifier(s) => write!(f, "{}", s),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Eq => write!(f, "=="),
            Token::Ne => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Le => write!(f, "<="),
            Token::Gt => write!(f, ">"),
            Token::Ge => write!(f, ">="),
            Token::And => write!(f, "&&"),
            Token::Or => write!(f, "||"),
            Token::Not => write!(f, "!"),
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Question => write!(f, "?"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

/// Lexer error
#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub position: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "lex error at position {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for LexError {}

/// Lexer for Azure DevOps expressions
pub struct Lexer<'a> {
    #[allow(dead_code)]
    input: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
            position: 0,
        }
    }

    /// Tokenize the entire input
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            if token == Token::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }

        Ok(tokens)
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace();

        let Some(&(pos, ch)) = self.chars.peek() else {
            return Ok(Token::Eof);
        };

        self.position = pos;

        match ch {
            // Single-character tokens
            '+' => {
                self.advance();
                Ok(Token::Plus)
            }
            '-' => {
                self.advance();
                Ok(Token::Minus)
            }
            '*' => {
                self.advance();
                Ok(Token::Star)
            }
            '/' => {
                self.advance();
                Ok(Token::Slash)
            }
            '%' => {
                self.advance();
                Ok(Token::Percent)
            }
            '.' => {
                self.advance();
                Ok(Token::Dot)
            }
            ',' => {
                self.advance();
                Ok(Token::Comma)
            }
            ':' => {
                self.advance();
                Ok(Token::Colon)
            }
            '?' => {
                self.advance();
                Ok(Token::Question)
            }
            '(' => {
                self.advance();
                Ok(Token::LParen)
            }
            ')' => {
                self.advance();
                Ok(Token::RParen)
            }
            '[' => {
                self.advance();
                Ok(Token::LBracket)
            }
            ']' => {
                self.advance();
                Ok(Token::RBracket)
            }
            '{' => {
                self.advance();
                Ok(Token::LBrace)
            }
            '}' => {
                self.advance();
                Ok(Token::RBrace)
            }

            // Two-character operators
            '=' => {
                self.advance();
                if self.peek_char() == Some('=') {
                    self.advance();
                    Ok(Token::Eq)
                } else {
                    Err(LexError {
                        message: "expected '==' operator".to_string(),
                        position: pos,
                    })
                }
            }
            '!' => {
                self.advance();
                if self.peek_char() == Some('=') {
                    self.advance();
                    Ok(Token::Ne)
                } else {
                    Ok(Token::Not)
                }
            }
            '<' => {
                self.advance();
                if self.peek_char() == Some('=') {
                    self.advance();
                    Ok(Token::Le)
                } else {
                    Ok(Token::Lt)
                }
            }
            '>' => {
                self.advance();
                if self.peek_char() == Some('=') {
                    self.advance();
                    Ok(Token::Ge)
                } else {
                    Ok(Token::Gt)
                }
            }
            '&' => {
                self.advance();
                if self.peek_char() == Some('&') {
                    self.advance();
                    Ok(Token::And)
                } else {
                    Err(LexError {
                        message: "expected '&&' operator".to_string(),
                        position: pos,
                    })
                }
            }
            '|' => {
                self.advance();
                if self.peek_char() == Some('|') {
                    self.advance();
                    Ok(Token::Or)
                } else {
                    Err(LexError {
                        message: "expected '||' operator".to_string(),
                        position: pos,
                    })
                }
            }

            // String literals
            '\'' => self.read_string(),

            // Numbers
            '0'..='9' => self.read_number(),

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => self.read_identifier(),

            _ => Err(LexError {
                message: format!("unexpected character: '{}'", ch),
                position: pos,
            }),
        }
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        self.chars.next()
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_, c)| c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(&(_, ch)) = self.chars.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self) -> Result<Token, LexError> {
        let start = self.position;
        self.advance(); // consume opening quote

        let mut value = String::new();

        loop {
            match self.chars.peek() {
                Some(&(_, '\'')) => {
                    self.advance();
                    // Check for escaped quote ('')
                    if self.peek_char() == Some('\'') {
                        value.push('\'');
                        self.advance();
                    } else {
                        break;
                    }
                }
                Some(&(_, ch)) => {
                    value.push(ch);
                    self.advance();
                }
                None => {
                    return Err(LexError {
                        message: "unterminated string".to_string(),
                        position: start,
                    });
                }
            }
        }

        Ok(Token::String(value))
    }

    fn read_number(&mut self) -> Result<Token, LexError> {
        let start = self.position;
        let mut num_str = String::new();

        // Integer part
        while let Some(&(_, ch)) = self.chars.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Decimal part - check if next char is '.' followed by a digit
        if self.peek_char() == Some('.') {
            // We need to look two characters ahead: . and then a digit
            // Clone the iterator to peek further
            let mut peek_iter = self.chars.clone();
            peek_iter.next(); // skip the '.'
            if let Some(&(_, next_ch)) = peek_iter.peek() {
                if next_ch.is_ascii_digit() {
                    // It's a decimal number
                    num_str.push('.');
                    self.advance(); // consume the '.'

                    while let Some(&(_, ch)) = self.chars.peek() {
                        if ch.is_ascii_digit() {
                            num_str.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        num_str
            .parse::<f64>()
            .map(Token::Number)
            .map_err(|_| LexError {
                message: format!("invalid number: {}", num_str),
                position: start,
            })
    }

    fn read_identifier(&mut self) -> Result<Token, LexError> {
        let mut ident = String::new();

        while let Some(&(_, ch)) = self.chars.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check for keywords
        let token = match ident.to_lowercase().as_str() {
            "null" => Token::Null,
            "true" => Token::True,
            "false" => Token::False,
            _ => Token::Identifier(ident),
        };

        Ok(token)
    }
}

/// Expression type extracted from text
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionType {
    /// Compile-time expression: ${{ expression }}
    CompileTime(String),
    /// Runtime expression: $[ expression ]
    Runtime(String),
    /// Macro/variable reference: $(variableName)
    Macro(String),
    /// Plain text (not an expression)
    Text(String),
}

/// Extract all expressions from a string
pub fn extract_expressions(input: &str) -> Vec<ExpressionType> {
    let mut results = Vec::new();
    let mut current_pos = 0;
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();

    while current_pos < len {
        // Check for compile-time expression ${{ }}
        if current_pos + 3 < len
            && chars[current_pos] == '$'
            && chars[current_pos + 1] == '{'
            && chars[current_pos + 2] == '{'
        {
            // Find closing }}
            if let Some(end) = find_closing(&chars, current_pos + 3, '}', '}') {
                let expr = chars[current_pos + 3..end]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_string();
                results.push(ExpressionType::CompileTime(expr));
                current_pos = end + 2;
                continue;
            }
        }

        // Check for runtime expression $[ ]
        if current_pos + 2 < len && chars[current_pos] == '$' && chars[current_pos + 1] == '[' {
            // Find closing ]
            if let Some(end) = find_closing_single(&chars, current_pos + 2, ']') {
                let expr = chars[current_pos + 2..end]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_string();
                results.push(ExpressionType::Runtime(expr));
                current_pos = end + 1;
                continue;
            }
        }

        // Check for macro $(var)
        if current_pos + 2 < len && chars[current_pos] == '$' && chars[current_pos + 1] == '(' {
            // Find closing )
            if let Some(end) = find_closing_single(&chars, current_pos + 2, ')') {
                let var = chars[current_pos + 2..end]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_string();
                results.push(ExpressionType::Macro(var));
                current_pos = end + 1;
                continue;
            }
        }

        // Regular text - accumulate until we hit an expression
        let text_start = current_pos;
        while current_pos < len {
            if current_pos + 1 < len && chars[current_pos] == '$' {
                let next = chars[current_pos + 1];
                if next == '{' || next == '[' || next == '(' {
                    break;
                }
            }
            current_pos += 1;
        }

        if current_pos > text_start {
            let text: String = chars[text_start..current_pos].iter().collect();
            results.push(ExpressionType::Text(text));
        }
    }

    results
}

fn find_closing(chars: &[char], start: usize, c1: char, c2: char) -> Option<usize> {
    let mut depth = 1;
    let mut i = start;

    while i + 1 < chars.len() {
        if chars[i] == c1 && chars[i + 1] == c2 {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
            i += 2;
        } else if chars[i] == '$'
            && i + 2 < chars.len()
            && chars[i + 1] == '{'
            && chars[i + 2] == '{'
        {
            depth += 1;
            i += 3;
        } else {
            i += 1;
        }
    }

    None
}

fn find_closing_single(chars: &[char], start: usize, closing: char) -> Option<usize> {
    let opening = if closing == ')' { '(' } else { '[' };
    let mut depth = 1;
    let mut i = start;
    let mut in_string = false;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\'' {
            in_string = !in_string;
        } else if !in_string {
            if ch == opening {
                depth += 1;
            } else if ch == closing {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }

        i += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_simple_tokens() {
        let mut lexer = Lexer::new("+ - * / ( )");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::LParen,
                Token::RParen,
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_lexer_comparison_operators() {
        let mut lexer = Lexer::new("== != < <= > >=");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::Eq,
                Token::Ne,
                Token::Lt,
                Token::Le,
                Token::Gt,
                Token::Ge,
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_lexer_logical_operators() {
        let mut lexer = Lexer::new("&& || !");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens, vec![Token::And, Token::Or, Token::Not, Token::Eof]);
    }

    #[test]
    fn test_lexer_string() {
        let mut lexer = Lexer::new("'hello world'");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![Token::String("hello world".to_string()), Token::Eof]
        );
    }

    #[test]
    fn test_lexer_escaped_string() {
        let mut lexer = Lexer::new("'it''s a test'");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![Token::String("it's a test".to_string()), Token::Eof]
        );
    }

    #[test]
    fn test_lexer_numbers() {
        let mut lexer = Lexer::new("42 3.14 0");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::Number(42.0),
                Token::Number(3.14),
                Token::Number(0.0),
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_lexer_identifiers() {
        let mut lexer = Lexer::new("foo bar_baz Build123");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::Identifier("foo".to_string()),
                Token::Identifier("bar_baz".to_string()),
                Token::Identifier("Build123".to_string()),
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_lexer_keywords() {
        let mut lexer = Lexer::new("null true false NULL TRUE FALSE");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::Null,
                Token::True,
                Token::False,
                Token::Null,
                Token::True,
                Token::False,
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_lexer_function_call() {
        let mut lexer = Lexer::new("eq(variables.foo, 'bar')");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::Identifier("eq".to_string()),
                Token::LParen,
                Token::Identifier("variables".to_string()),
                Token::Dot,
                Token::Identifier("foo".to_string()),
                Token::Comma,
                Token::String("bar".to_string()),
                Token::RParen,
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_lexer_complex_expression() {
        let mut lexer =
            Lexer::new("and(succeeded(), eq(variables['Build.SourceBranch'], 'refs/heads/main'))");
        let tokens = lexer.tokenize().unwrap();

        assert!(tokens.len() > 10);
        assert_eq!(tokens.first(), Some(&Token::Identifier("and".to_string())));
    }

    #[test]
    fn test_extract_compile_time_expression() {
        let exprs = extract_expressions("${{ variables.foo }}");

        assert_eq!(
            exprs,
            vec![ExpressionType::CompileTime("variables.foo".to_string())]
        );
    }

    #[test]
    fn test_extract_runtime_expression() {
        let exprs = extract_expressions("$[ succeeded() ]");

        assert_eq!(
            exprs,
            vec![ExpressionType::Runtime("succeeded()".to_string())]
        );
    }

    #[test]
    fn test_extract_macro() {
        let exprs = extract_expressions("$(Build.SourceBranch)");

        assert_eq!(
            exprs,
            vec![ExpressionType::Macro("Build.SourceBranch".to_string())]
        );
    }

    #[test]
    fn test_extract_mixed() {
        let exprs =
            extract_expressions("Branch: $(Build.SourceBranch) - Config: ${{ variables.config }}");

        assert_eq!(
            exprs,
            vec![
                ExpressionType::Text("Branch: ".to_string()),
                ExpressionType::Macro("Build.SourceBranch".to_string()),
                ExpressionType::Text(" - Config: ".to_string()),
                ExpressionType::CompileTime("variables.config".to_string()),
            ]
        );
    }

    #[test]
    fn test_extract_nested_macro() {
        let exprs = extract_expressions("$(variables[Build.Configuration])");

        assert_eq!(
            exprs,
            vec![ExpressionType::Macro(
                "variables[Build.Configuration]".to_string()
            )]
        );
    }
}
