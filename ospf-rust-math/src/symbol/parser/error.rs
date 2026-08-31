//! 解析错误类型 / Parse Error Types
//!
//! 提供解析过程中的错误处理。
//! Provides error handling during parsing.

use std::fmt;

/// 解析错误 / Parse error
#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    /// 错误消息 / Error message
    pub message: String,
    /// 错误位置（字节偏移）/ Error position (byte offset)
    pub position: usize,
    /// 错误长度 / Error length
    pub length: usize,
}

impl ParseError {
    /// 创建新的解析错误 / Create a new parse error
    pub fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
            length: 1,
        }
    }

    /// 创建带长度的解析错误 / Create a parse error with length
    pub fn with_length(message: impl Into<String>, position: usize, length: usize) -> Self {
        Self {
            message: message.into(),
            position,
            length,
        }
    }

    /// 获取错误消息 / Get error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 获取错误位置 / Get error position
    pub fn position(&self) -> usize {
        self.position
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at position {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for ParseError {}

/// 解析结果类型 / Parse result type
pub type ParseResult<T> = Result<T, ParseError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_display() {
        let err = ParseError::new("unexpected token", 10);
        assert_eq!(
            format!("{}", err),
            "Parse error at position 10: unexpected token"
        );
    }

    #[test]
    fn test_parse_error_with_length() {
        let err = ParseError::with_length("invalid number", 5, 3);
        assert_eq!(err.position, 5);
        assert_eq!(err.length, 3);
    }
}
