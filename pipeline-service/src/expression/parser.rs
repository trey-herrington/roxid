// Expression Engine Parser
// Parses tokens into an AST for Azure DevOps expressions

use crate::expression::lexer::{LexError, Lexer, Token};

use std::fmt;

/// Abstract Syntax Tree node for expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Null literal
    Null,

    /// Boolean literal
    Bool(bool),

    /// Number literal
    Number(f64),

    /// String literal
    String(String),

    /// Variable/property reference: variables.foo, parameters['key']
    Reference(Reference),

    /// Function call: eq(a, b), contains(str, 'substr')
    FunctionCall { name: String, args: Vec<Expr> },

    /// Index access: arr[0], obj['key']
    Index { object: Box<Expr>, index: Box<Expr> },

    /// Member access: obj.property
    Member { object: Box<Expr>, property: String },

    /// Unary operation: !expr
    Unary { op: UnaryOp, expr: Box<Expr> },

    /// Binary operation: a == b, a && b
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Ternary/conditional: condition ? then : else
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    /// Array literal: [1, 2, 3]
    Array(Vec<Expr>),

    /// Object literal: { key: value }
    Object(Vec<(String, Expr)>),
}

/// Reference to a context value (variables, parameters, etc.)
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub parts: Vec<ReferencePart>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReferencePart {
    /// Property access by name
    Property(String),
    /// Index access by key/index
    Index(Box<Expr>),
}

impl Reference {
    pub fn new(name: String) -> Self {
        Self {
            parts: vec![ReferencePart::Property(name)],
        }
    }

    pub fn with_property(mut self, name: String) -> Self {
        self.parts.push(ReferencePart::Property(name));
        self
    }

    pub fn with_index(mut self, index: Expr) -> Self {
        self.parts.push(ReferencePart::Index(Box::new(index)));
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not, // !
    Neg, // - (unary minus)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Mod, // %

    // Comparison
    Eq, // ==
    Ne, // !=
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=

    // Logical
    And, // &&
    Or,  // ||
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Mod => write!(f, "%"),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::Ne => write!(f, "!="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Le => write!(f, "<="),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Ge => write!(f, ">="),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
        }
    }
}

/// Parser error
#[derive(Debug, Clone)]
pub struct ParseExprError {
    pub message: String,
    pub position: usize,
}

impl fmt::Display for ParseExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "parse error at position {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for ParseExprError {}

impl From<LexError> for ParseExprError {
    fn from(err: LexError) -> Self {
        Self {
            message: err.message,
            position: err.position,
        }
    }
}

/// Recursive descent parser for Azure DevOps expressions
pub struct ExprParser {
    tokens: Vec<Token>,
    position: usize,
}

impl ExprParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Parse expression from string
    pub fn parse_str(input: &str) -> Result<Expr, ParseExprError> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        let mut parser = Self::new(tokens);
        parser.parse()
    }

    /// Parse the token stream into an expression
    pub fn parse(&mut self) -> Result<Expr, ParseExprError> {
        let expr = self.parse_ternary()?;

        if !self.is_at_end() && self.peek() != &Token::Eof {
            return Err(self.error(&format!("unexpected token: {:?}", self.peek())));
        }

        Ok(expr)
    }

    // Precedence (lowest to highest):
    // 1. Ternary: ?:
    // 2. Or: ||
    // 3. And: &&
    // 4. Equality: == !=
    // 5. Comparison: < <= > >=
    // 6. Additive: + -
    // 7. Multiplicative: * / %
    // 8. Unary: ! -
    // 9. Postfix: . [] ()

    fn parse_ternary(&mut self) -> Result<Expr, ParseExprError> {
        let condition = self.parse_or()?;

        if self.check(&Token::Question) {
            self.advance();
            let then_expr = self.parse_ternary()?;
            self.expect(&Token::Colon, "expected ':' in ternary expression")?;
            let else_expr = self.parse_ternary()?;

            return Ok(Expr::Ternary {
                condition: Box::new(condition),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            });
        }

        Ok(condition)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseExprError> {
        let mut left = self.parse_and()?;

        while self.check(&Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseExprError> {
        let mut left = self.parse_equality()?;

        while self.check(&Token::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseExprError> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = match self.peek() {
                Token::Eq => BinaryOp::Eq,
                Token::Ne => BinaryOp::Ne,
                _ => break,
            };

            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseExprError> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.peek() {
                Token::Lt => BinaryOp::Lt,
                Token::Le => BinaryOp::Le,
                Token::Gt => BinaryOp::Gt,
                Token::Ge => BinaryOp::Ge,
                _ => break,
            };

            self.advance();
            let right = self.parse_additive()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseExprError> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.peek() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };

            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseExprError> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.peek() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Percent => BinaryOp::Mod,
                _ => break,
            };

            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseExprError> {
        if self.check(&Token::Not) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            });
        }

        if self.check(&Token::Minus) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseExprError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&Token::Dot) {
                self.advance();
                let Token::Identifier(property) = self.advance().clone() else {
                    return Err(self.error("expected property name after '.'"));
                };

                // Check if this is a method call
                if self.check(&Token::LParen) {
                    // Method call on object - convert to function call
                    let args = self.parse_args()?;
                    expr = Expr::FunctionCall {
                        name: property,
                        args: std::iter::once(expr).chain(args).collect(),
                    };
                } else {
                    expr = Expr::Member {
                        object: Box::new(expr),
                        property,
                    };
                }
            } else if self.check(&Token::LBracket) {
                self.advance();
                let index = self.parse_ternary()?;
                self.expect(&Token::RBracket, "expected ']'")?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseExprError> {
        match self.peek().clone() {
            Token::Null => {
                self.advance();
                Ok(Expr::Null)
            }
            Token::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            Token::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expr::String(s))
            }
            Token::Identifier(name) => {
                self.advance();

                // Check if this is a function call
                if self.check(&Token::LParen) {
                    let args = self.parse_args()?;
                    Ok(Expr::FunctionCall { name, args })
                } else {
                    // Build reference
                    let mut reference = Reference::new(name);

                    // Parse additional member/index accesses
                    while self.check(&Token::Dot) || self.check(&Token::LBracket) {
                        if self.check(&Token::Dot) {
                            self.advance();
                            let Token::Identifier(prop) = self.advance().clone() else {
                                return Err(self.error("expected property name after '.'"));
                            };
                            reference = reference.with_property(prop);
                        } else {
                            self.advance();
                            let index = self.parse_ternary()?;
                            self.expect(&Token::RBracket, "expected ']'")?;
                            reference = reference.with_index(index);
                        }
                    }

                    Ok(Expr::Reference(reference))
                }
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_ternary()?;
                self.expect(&Token::RParen, "expected ')'")?;
                Ok(expr)
            }
            Token::LBracket => {
                self.advance();
                let mut items = Vec::new();

                if !self.check(&Token::RBracket) {
                    items.push(self.parse_ternary()?);

                    while self.check(&Token::Comma) {
                        self.advance();
                        if self.check(&Token::RBracket) {
                            break; // trailing comma
                        }
                        items.push(self.parse_ternary()?);
                    }
                }

                self.expect(&Token::RBracket, "expected ']'")?;
                Ok(Expr::Array(items))
            }
            Token::LBrace => {
                self.advance();
                let mut pairs = Vec::new();

                if !self.check(&Token::RBrace) {
                    loop {
                        let key = match self.advance().clone() {
                            Token::Identifier(s) => s,
                            Token::String(s) => s,
                            _ => return Err(self.error("expected object key")),
                        };

                        self.expect(&Token::Colon, "expected ':' after object key")?;
                        let value = self.parse_ternary()?;
                        pairs.push((key, value));

                        if !self.check(&Token::Comma) {
                            break;
                        }
                        self.advance();
                        if self.check(&Token::RBrace) {
                            break; // trailing comma
                        }
                    }
                }

                self.expect(&Token::RBrace, "expected '}'")?;
                Ok(Expr::Object(pairs))
            }
            token => Err(self.error(&format!("unexpected token: {:?}", token))),
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, ParseExprError> {
        self.expect(&Token::LParen, "expected '('")?;

        let mut args = Vec::new();

        if !self.check(&Token::RParen) {
            args.push(self.parse_ternary()?);

            while self.check(&Token::Comma) {
                self.advance();
                if self.check(&Token::RParen) {
                    break; // trailing comma
                }
                args.push(self.parse_ternary()?);
            }
        }

        self.expect(&Token::RParen, "expected ')'")?;
        Ok(args)
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> &Token {
        let token = self.tokens.get(self.position).unwrap_or(&Token::Eof);
        self.position += 1;
        token
    }

    fn check(&self, token: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(token)
    }

    fn expect(&mut self, token: &Token, msg: &str) -> Result<(), ParseExprError> {
        if self.check(token) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(msg))
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len() || matches!(self.peek(), Token::Eof)
    }

    fn error(&self, message: &str) -> ParseExprError {
        ParseExprError {
            message: message.to_string(),
            position: self.position,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_literals() {
        assert_eq!(ExprParser::parse_str("null").unwrap(), Expr::Null);
        assert_eq!(ExprParser::parse_str("true").unwrap(), Expr::Bool(true));
        assert_eq!(ExprParser::parse_str("false").unwrap(), Expr::Bool(false));
        assert_eq!(ExprParser::parse_str("42").unwrap(), Expr::Number(42.0));
        assert_eq!(
            ExprParser::parse_str("'hello'").unwrap(),
            Expr::String("hello".to_string())
        );
    }

    #[test]
    fn test_parse_reference() {
        let expr = ExprParser::parse_str("variables.foo").unwrap();
        assert!(matches!(expr, Expr::Reference(_)));

        if let Expr::Reference(r) = expr {
            assert_eq!(r.parts.len(), 2);
            assert_eq!(r.parts[0], ReferencePart::Property("variables".to_string()));
            assert_eq!(r.parts[1], ReferencePart::Property("foo".to_string()));
        }
    }

    #[test]
    fn test_parse_index_access() {
        let expr = ExprParser::parse_str("variables['foo']").unwrap();
        assert!(matches!(expr, Expr::Reference(_)));
    }

    #[test]
    fn test_parse_function_call() {
        let expr = ExprParser::parse_str("eq(a, b)").unwrap();

        if let Expr::FunctionCall { name, args } = expr {
            assert_eq!(name, "eq");
            assert_eq!(args.len(), 2);
        } else {
            panic!("expected function call");
        }
    }

    #[test]
    fn test_parse_nested_function_call() {
        let expr = ExprParser::parse_str("and(eq(a, b), contains(c, 'd'))").unwrap();

        if let Expr::FunctionCall { name, args } = expr {
            assert_eq!(name, "and");
            assert_eq!(args.len(), 2);
        } else {
            panic!("expected function call");
        }
    }

    #[test]
    fn test_parse_binary_operators() {
        let expr = ExprParser::parse_str("a == b").unwrap();
        assert!(matches!(
            expr,
            Expr::Binary {
                op: BinaryOp::Eq,
                ..
            }
        ));

        let expr = ExprParser::parse_str("a && b").unwrap();
        assert!(matches!(
            expr,
            Expr::Binary {
                op: BinaryOp::And,
                ..
            }
        ));

        let expr = ExprParser::parse_str("a || b").unwrap();
        assert!(matches!(
            expr,
            Expr::Binary {
                op: BinaryOp::Or,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_unary_not() {
        let expr = ExprParser::parse_str("!succeeded()").unwrap();
        assert!(matches!(
            expr,
            Expr::Unary {
                op: UnaryOp::Not,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_ternary() {
        let expr = ExprParser::parse_str("condition ? 'yes' : 'no'").unwrap();
        assert!(matches!(expr, Expr::Ternary { .. }));
    }

    #[test]
    fn test_parse_array() {
        let expr = ExprParser::parse_str("[1, 2, 3]").unwrap();

        if let Expr::Array(items) = expr {
            assert_eq!(items.len(), 3);
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn test_parse_complex_expression() {
        // Real Azure DevOps expression
        let expr = ExprParser::parse_str(
            "and(succeeded(), eq(variables['Build.SourceBranch'], 'refs/heads/main'))",
        )
        .unwrap();

        if let Expr::FunctionCall { name, args } = expr {
            assert_eq!(name, "and");
            assert_eq!(args.len(), 2);
        } else {
            panic!("expected function call");
        }
    }

    #[test]
    fn test_parse_operator_precedence() {
        // && should bind tighter than ||
        let expr = ExprParser::parse_str("a || b && c").unwrap();

        if let Expr::Binary {
            op: BinaryOp::Or,
            right,
            ..
        } = expr
        {
            assert!(matches!(
                *right,
                Expr::Binary {
                    op: BinaryOp::And,
                    ..
                }
            ));
        } else {
            panic!("expected or expression");
        }
    }

    #[test]
    fn test_parse_arithmetic() {
        let expr = ExprParser::parse_str("1 + 2 * 3").unwrap();

        // * should bind tighter than +
        if let Expr::Binary {
            op: BinaryOp::Add,
            right,
            ..
        } = expr
        {
            assert!(matches!(
                *right,
                Expr::Binary {
                    op: BinaryOp::Mul,
                    ..
                }
            ));
        } else {
            panic!("expected add expression");
        }
    }

    #[test]
    fn test_parse_parentheses() {
        let expr = ExprParser::parse_str("(1 + 2) * 3").unwrap();

        // Parentheses should override precedence
        if let Expr::Binary {
            op: BinaryOp::Mul,
            left,
            ..
        } = expr
        {
            assert!(matches!(
                *left,
                Expr::Binary {
                    op: BinaryOp::Add,
                    ..
                }
            ));
        } else {
            panic!("expected mul expression");
        }
    }
}
