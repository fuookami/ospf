//! 词法分析器 / Lexer
//!
//! 将输入字符串转换为 token 序列。
//! Converts input string to token sequence.

use super::error::{ParseError, ParseResult};

/// Token 类型 / Token kind
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    /// 标识符（变量名）/ Identifier (variable name)
    Ident(String),
    /// 数字 / Number
    Number(String),
    /// 加号 / Plus
    Plus,
    /// 减号 / Minus
    Minus,
    /// 乘号 / Asterisk
    Asterisk,
    /// 除号 / Slash
    Slash,
    /// 幂运算符 / Power operator
    Caret,
    /// 左括号 / Left parenthesis
    LParen,
    /// 右括号 / Right parenthesis
    RParen,
    /// 小于等于 / Less than or equal
    LessEqual,
    /// 大于等于 / Greater than or equal
    GreaterEqual,
    /// 小于 / Less than
    Less,
    /// 大于 / Greater than
    Greater,
    /// 等于 / Equal
    Equal,
    /// 逗号 / Comma
    Comma,
    /// 文件结束 / End of file
    Eof,
}

/// Token / Token
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    /// Token 类型 / Token kind
    pub kind: TokenKind,
    /// 起始位置 / Start position
    pub start: usize,
    /// 结束位置 / End position
    pub end: usize,
}

impl Token {
    /// 创建新 Token / Create a new token
    pub fn new(kind: TokenKind, start: usize, end: usize) -> Self {
        Self { kind, start, end }
    }

    /// 获取 Token 文本长度 / Get token text length
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// 检查 Token 是否为空 / Check if token is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 词法分析器 / Lexer
pub struct Lexer<'a> {
    /// 输入字符串 / Input string
    input: &'a str,
    /// 当前位置 / Current position
    position: usize,
    /// 当前字符 / Current character
    current: Option<char>,
}

impl<'a> Lexer<'a> {
    /// 创建新的词法分析器 / Create a new lexer
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Self {
            input,
            position: 0,
            current: None,
        };
        lexer.advance();
        lexer
    }

    /// 前进到下一个字符 / Advance to next character
    fn advance(&mut self) {
        if self.position < self.input.len() {
            self.current = self.input[self.position..].chars().next();
            self.position += self.current.map_or(0, |c| c.len_utf8());
        } else {
            self.current = None;
        }
    }

    /// 预览下一个字符 / Peek next character
    fn peek(&self) -> Option<char> {
        let next_pos = self.position + self.current.map_or(0, |c| c.len_utf8());
        if next_pos < self.input.len() {
            self.input[next_pos..].chars().next()
        } else {
            None
        }
    }

    /// 跳过空白字符 / Skip whitespace
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// 读取数字 / Read number
    fn read_number(&mut self) -> Token {
        let start = self.position - self.current.map_or(0, |c| c.len_utf8());
        let mut s = String::new();

        // 读取整数部分 / Read integer part
        while let Some(c) = self.current {
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // 读取小数部分 / Read fractional part
        if self.current == Some('.') {
            s.push('.');
            self.advance();
            while let Some(c) = self.current {
                if c.is_ascii_digit() {
                    s.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // 读取指数部分 / Read exponent part
        if self.current == Some('e') || self.current == Some('E') {
            s.push(self.current.unwrap());
            self.advance();
            if self.current == Some('+') || self.current == Some('-') {
                s.push(self.current.unwrap());
                self.advance();
            }
            while let Some(c) = self.current {
                if c.is_ascii_digit() {
                    s.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        Token::new(TokenKind::Number(s), start, self.position)
    }

    /// 检查字符是否为 Unicode 上标数字 / Check if char is Unicode superscript digit
    fn is_superscript_digit(c: char) -> bool {
        matches!(c, '⁰' | '¹' | '²' | '³' | '⁴' | '⁵' | '⁶' | '⁷' | '⁸' | '⁹')
    }

    /// 读取标识符 / Read identifier
    fn read_identifier(&mut self) -> Token {
        let start = self.position - self.current.map_or(0, |c| c.len_utf8());
        let mut s = String::new();

        while let Some(c) = self.current {
            // 排除 Unicode 上标数字，它们应该被单独处理
            // Exclude Unicode superscript digits, they should be handled separately
            if (c.is_alphanumeric() || c == '_') && !Self::is_superscript_digit(c) {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }

        Token::new(TokenKind::Ident(s), start, self.position)
    }

    /// 获取下一个 Token / Get next token
    pub fn next_token(&mut self) -> ParseResult<Token> {
        self.skip_whitespace();

        let start = self.position;

        match self.current {
            Some(c) => {
                match c {
                    // 运算符和分隔符 / Operators and delimiters
                    '+' => {
                        self.advance();
                        Ok(Token::new(TokenKind::Plus, start, self.position))
                    }
                    '-' => {
                        self.advance();
                        Ok(Token::new(TokenKind::Minus, start, self.position))
                    }
                    '*' => {
                        self.advance();
                        Ok(Token::new(TokenKind::Asterisk, start, self.position))
                    }
                    '/' => {
                        self.advance();
                        Ok(Token::new(TokenKind::Slash, start, self.position))
                    }
                    '^' => {
                        self.advance();
                        Ok(Token::new(TokenKind::Caret, start, self.position))
                    }
                    '(' => {
                        self.advance();
                        Ok(Token::new(TokenKind::LParen, start, self.position))
                    }
                    ')' => {
                        self.advance();
                        Ok(Token::new(TokenKind::RParen, start, self.position))
                    }
                    ',' => {
                        self.advance();
                        Ok(Token::new(TokenKind::Comma, start, self.position))
                    }
                    '=' => {
                        self.advance();
                        Ok(Token::new(TokenKind::Equal, start, self.position))
                    }
                    '<' => {
                        self.advance();
                        if self.current == Some('=') {
                            self.advance();
                            Ok(Token::new(TokenKind::LessEqual, start, self.position))
                        } else {
                            Ok(Token::new(TokenKind::Less, start, self.position))
                        }
                    }
                    '>' => {
                        self.advance();
                        if self.current == Some('=') {
                            self.advance();
                            Ok(Token::new(TokenKind::GreaterEqual, start, self.position))
                        } else {
                            Ok(Token::new(TokenKind::Greater, start, self.position))
                        }
                    }
                    // 数字 / Numbers
                    '0'..='9' | '.' => Ok(self.read_number()),
                    // 标识符 / Identifiers
                    'a'..='z' | 'A'..='Z' | '_' => Ok(self.read_identifier()),
                    // Unicode 上标数字 / Unicode superscript digits
                    '²' => {
                        self.advance();
                        Ok(Token::new(
                            TokenKind::Number("2".to_string()),
                            start,
                            self.position,
                        ))
                    }
                    '³' => {
                        self.advance();
                        Ok(Token::new(
                            TokenKind::Number("3".to_string()),
                            start,
                            self.position,
                        ))
                    }
                    // 不支持的字符 / Unsupported character
                    _ => {
                        let pos = self.position;
                        let c = self.current;
                        self.advance();
                        Err(ParseError::new(
                            format!("Unexpected character: {:?}", c),
                            pos,
                        ))
                    }
                }
            }
            None => Ok(Token::new(TokenKind::Eof, start, start)),
        }
    }

    /// 收集所有 Token / Collect all tokens
    pub fn collect_tokens(mut self) -> ParseResult<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_simple() {
        let mut lexer = Lexer::new("2x + 3y");
        let tokens = lexer.collect_tokens().unwrap();
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].kind, TokenKind::Number("2".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Ident("x".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::Plus);
        assert_eq!(tokens[3].kind, TokenKind::Number("3".to_string()));
        assert_eq!(tokens[4].kind, TokenKind::Ident("y".to_string()));
        assert_eq!(tokens[5].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lexer_comparison() {
        let mut lexer = Lexer::new("x <= 5");
        let tokens = lexer.collect_tokens().unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].kind, TokenKind::Ident("x".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::LessEqual);
        assert_eq!(tokens[2].kind, TokenKind::Number("5".to_string()));
    }

    #[test]
    fn test_lexer_decimal() {
        let mut lexer = Lexer::new("2.5 * x");
        let tokens = lexer.collect_tokens().unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].kind, TokenKind::Number("2.5".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Asterisk);
        assert_eq!(tokens[2].kind, TokenKind::Ident("x".to_string()));
    }

    #[test]
    fn test_lexer_scientific_notation() {
        let mut lexer = Lexer::new("1.5e-10");
        let tokens = lexer.collect_tokens().unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::Number("1.5e-10".to_string()));
    }

    #[test]
    fn test_lexer_unicode_power() {
        let lexer = Lexer::new("x² + y³");
        let tokens = lexer.collect_tokens().unwrap();
        // x² + y³ 应该产生: x, 2, +, y, 3, Eof
        // x² + y³ should produce: x, 2, +, y, 3, Eof
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].kind, TokenKind::Ident("x".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Number("2".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::Plus);
        assert_eq!(tokens[3].kind, TokenKind::Ident("y".to_string()));
        assert_eq!(tokens[4].kind, TokenKind::Number("3".to_string()));
        assert_eq!(tokens[5].kind, TokenKind::Eof);
    }
}
