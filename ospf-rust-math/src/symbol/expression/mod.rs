//! 运行时表达式系统
//! Runtime expression system

mod property_path;
mod operators;
mod value;
mod scalar;
mod boolean;
mod dsl;
mod evaluation;
mod normalize;
mod math_functions;

#[cfg(feature = "parser")]
mod scalar_parser;

pub use property_path::*;
pub use operators::*;
pub use value::*;
pub use scalar::*;
pub use boolean::*;
pub use dsl::*;
pub use evaluation::*;
pub use normalize::*;
pub use math_functions::*;

#[cfg(feature = "parser")]
mod parser_support {
    use super::*;
    use std::fmt::{Display, Formatter};

    /// 表达式解析错误。
    /// Expression parse error.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ExpressionParseError {
        message: String,
        position: usize,
    }

    impl ExpressionParseError {
        /// 创建表达式解析错误。
        /// Create an expression parse error.
        pub fn new(message: impl Into<String>, position: usize) -> Self {
            Self {
                message: message.into(),
                position,
            }
        }

        /// 获取错误消息。
        /// Get error message.
        pub fn message(&self) -> &str {
            &self.message
        }

        /// 获取错误位置。
        /// Get error position.
        pub fn position(&self) -> usize {
            self.position
        }
    }

    impl Display for ExpressionParseError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} at {}", self.message, self.position)
        }
    }

    impl std::error::Error for ExpressionParseError {}

    /// 表达式词法单元类型。
    /// Expression token type.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ExpressionTokenType {
        /// 真常量 / True constant
        True,
        /// 假常量 / False constant
        False,
        /// 空值 / Null value
        Null,
        /// 字符串字面量 / String literal
        String,
        /// 数字字面量 / Number literal
        Number,
        /// 标识符或属性路径 / Identifier or property path
        Identifier,
        /// 逻辑与 / Logical AND
        And,
        /// 逻辑或 / Logical OR
        Or,
        /// 逻辑非 / Logical NOT
        Not,
        /// 集合成员判断 / Set membership
        In,
        /// 空值判断关键字 / Null-check keyword
        Is,
        /// LIKE 模式匹配 / LIKE pattern match
        Like,
        /// 包含匹配 / Contains match
        Contains,
        /// 前缀匹配 / Prefix match
        Prefix,
        /// 后缀匹配 / Suffix match
        Suffix,
        /// 正则匹配 / Regex match
        Regex,
        /// 精确匹配 / Exact match
        Exact,
        /// 等于 / Equal
        Eq,
        /// 不等于 / Not equal
        Ne,
        /// 小于 / Less than
        Lt,
        /// 小于等于 / Less than or equal
        Le,
        /// 大于 / Greater than
        Gt,
        /// 大于等于 / Greater than or equal
        Ge,
        /// 左括号 / Left parenthesis
        LParen,
        /// 右括号 / Right parenthesis
        RParen,
        /// 逗号 / Comma
        Comma,
        /// 文件结束 / End of file
        Eof,
        /// 未知词法单元 / Unknown token
        Unknown,
    }

    impl ExpressionTokenType {
        fn comparison_operator(self) -> Option<ComparisonOperator> {
            match self {
                Self::Eq => Some(ComparisonOperator::Eq),
                Self::Ne => Some(ComparisonOperator::Ne),
                Self::Lt => Some(ComparisonOperator::Lt),
                Self::Le => Some(ComparisonOperator::Le),
                Self::Gt => Some(ComparisonOperator::Gt),
                Self::Ge => Some(ComparisonOperator::Ge),
                _ => None,
            }
        }

        fn pattern_match_mode(self) -> Option<PatternMatchMode> {
            match self {
                Self::Like => Some(PatternMatchMode::Like),
                Self::Contains => Some(PatternMatchMode::Contains),
                Self::Prefix => Some(PatternMatchMode::Prefix),
                Self::Suffix => Some(PatternMatchMode::Suffix),
                Self::Regex => Some(PatternMatchMode::Regex),
                Self::Exact => Some(PatternMatchMode::Exact),
                _ => None,
            }
        }

        fn is_comparison_operator(self) -> bool {
            self.comparison_operator().is_some()
        }

        fn is_pattern_operator(self) -> bool {
            self.pattern_match_mode().is_some()
        }
    }

    /// 表达式词法单元。
    /// Expression token.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ExpressionToken {
        /// 词法单元类型 / Token type
        pub token_type: ExpressionTokenType,
        /// 词法单元值 / Token value
        pub value: String,
        /// 词法单元位置 / Token position
        pub position: usize,
    }

    impl ExpressionToken {
        fn new(token_type: ExpressionTokenType, value: impl Into<String>, position: usize) -> Self {
            Self {
                token_type,
                value: value.into(),
                position,
            }
        }

        fn eof(position: usize) -> Self {
            Self::new(ExpressionTokenType::Eof, "", position)
        }

        fn unknown(value: impl Into<String>, position: usize) -> Self {
            Self::new(ExpressionTokenType::Unknown, value, position)
        }
    }

    /// 表达式词法分析器。
    /// Expression lexer.
    pub struct ExpressionLexer {
        input: Vec<char>,
        position: usize,
    }

    impl ExpressionLexer {
        /// 创建表达式词法分析器。
        /// Create an expression lexer.
        pub fn new(input: impl AsRef<str>) -> Self {
            Self {
                input: input.as_ref().chars().collect(),
                position: 0,
            }
        }

        /// 分析完整输入并返回词法单元列表。
        /// Tokenize the whole input and return token list.
        pub fn tokenize(&mut self) -> Vec<ExpressionToken> {
            let mut tokens = Vec::new();
            loop {
                let token = self.next_token();
                let is_eof = token.token_type == ExpressionTokenType::Eof;
                tokens.push(token);
                if is_eof {
                    break;
                }
            }
            tokens
        }

        /// 获取下一个词法单元。
        /// Get next token.
        pub fn next_token(&mut self) -> ExpressionToken {
            self.skip_whitespace();
            let start = self.position;
            let Some(current) = self.current_char() else {
                return ExpressionToken::eof(start);
            };

            if current == '\'' || current == '"' {
                return self.read_string(start);
            }
            if current.is_ascii_digit()
                || (current == '-'
                    && self
                        .peek_char(1)
                        .map(|ch| ch.is_ascii_digit())
                        .unwrap_or(false))
            {
                return self.read_number(start);
            }
            if current.is_alphabetic() || current == '_' {
                return self.read_identifier_or_keyword(start);
            }

            match current {
                '(' => {
                    self.advance();
                    ExpressionToken::new(ExpressionTokenType::LParen, "(", start)
                }
                ')' => {
                    self.advance();
                    ExpressionToken::new(ExpressionTokenType::RParen, ")", start)
                }
                ',' => {
                    self.advance();
                    ExpressionToken::new(ExpressionTokenType::Comma, ",", start)
                }
                '=' => {
                    self.advance();
                    ExpressionToken::new(ExpressionTokenType::Eq, "=", start)
                }
                '!' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        ExpressionToken::new(ExpressionTokenType::Ne, "!=", start)
                    } else {
                        ExpressionToken::unknown("!", start)
                    }
                }
                '<' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        ExpressionToken::new(ExpressionTokenType::Le, "<=", start)
                    } else if self.current_char() == Some('>') {
                        self.advance();
                        ExpressionToken::new(ExpressionTokenType::Ne, "<>", start)
                    } else {
                        ExpressionToken::new(ExpressionTokenType::Lt, "<", start)
                    }
                }
                '>' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        ExpressionToken::new(ExpressionTokenType::Ge, ">=", start)
                    } else {
                        ExpressionToken::new(ExpressionTokenType::Gt, ">", start)
                    }
                }
                _ => {
                    self.advance();
                    ExpressionToken::unknown(current.to_string(), start)
                }
            }
        }

        fn current_char(&self) -> Option<char> {
            self.input.get(self.position).copied()
        }

        fn peek_char(&self, offset: usize) -> Option<char> {
            self.input.get(self.position + offset).copied()
        }

        fn advance(&mut self) {
            self.position += 1;
        }

        fn skip_whitespace(&mut self) {
            while self
                .current_char()
                .map(|ch| ch.is_whitespace())
                .unwrap_or(false)
            {
                self.advance();
            }
        }

        fn read_string(&mut self, start: usize) -> ExpressionToken {
            let quote = self.current_char().unwrap_or('"');
            self.advance();
            let mut value = String::new();
            while let Some(current) = self.current_char() {
                if current == quote {
                    self.advance();
                    break;
                }
                if current == '\\' {
                    self.advance();
                    match self.current_char() {
                        Some('n') => value.push('\n'),
                        Some('t') => value.push('\t'),
                        Some('r') => value.push('\r'),
                        Some('\\') => value.push('\\'),
                        Some('\'') => value.push('\''),
                        Some('"') => value.push('"'),
                        Some(other) => {
                            value.push('\\');
                            value.push(other);
                        }
                        None => value.push('\\'),
                    }
                } else {
                    value.push(current);
                }
                self.advance();
            }
            ExpressionToken::new(ExpressionTokenType::String, value, start)
        }

        fn read_number(&mut self, start: usize) -> ExpressionToken {
            let mut value = String::new();
            if self.current_char() == Some('-') {
                value.push('-');
                self.advance();
            }
            while self
                .current_char()
                .map(|ch| ch.is_ascii_digit())
                .unwrap_or(false)
            {
                value.push(self.current_char().unwrap_or_default());
                self.advance();
            }
            if self.current_char() == Some('.')
                && self
                    .peek_char(1)
                    .map(|ch| ch.is_ascii_digit())
                    .unwrap_or(false)
            {
                value.push('.');
                self.advance();
                while self
                    .current_char()
                    .map(|ch| ch.is_ascii_digit())
                    .unwrap_or(false)
                {
                    value.push(self.current_char().unwrap_or_default());
                    self.advance();
                }
            }
            if matches!(self.current_char(), Some('e') | Some('E')) {
                value.push(self.current_char().unwrap_or_default());
                self.advance();
                if matches!(self.current_char(), Some('+') | Some('-')) {
                    value.push(self.current_char().unwrap_or_default());
                    self.advance();
                }
                while self
                    .current_char()
                    .map(|ch| ch.is_ascii_digit())
                    .unwrap_or(false)
                {
                    value.push(self.current_char().unwrap_or_default());
                    self.advance();
                }
            }
            ExpressionToken::new(ExpressionTokenType::Number, value, start)
        }

        fn read_identifier_or_keyword(&mut self, start: usize) -> ExpressionToken {
            let mut value = String::new();
            while self
                .current_char()
                .map(|ch| ch.is_alphanumeric() || ch == '_')
                .unwrap_or(false)
            {
                value.push(self.current_char().unwrap_or_default());
                self.advance();
            }
            while self.current_char() == Some('.')
                && self
                    .peek_char(1)
                    .map(|ch| ch.is_alphabetic() || ch == '_')
                    .unwrap_or(false)
            {
                value.push('.');
                self.advance();
                while self
                    .current_char()
                    .map(|ch| ch.is_alphanumeric() || ch == '_')
                    .unwrap_or(false)
                {
                    value.push(self.current_char().unwrap_or_default());
                    self.advance();
                }
            }

            let token_type = match value.to_ascii_lowercase().as_str() {
                "and" => ExpressionTokenType::And,
                "or" => ExpressionTokenType::Or,
                "not" => ExpressionTokenType::Not,
                "in" => ExpressionTokenType::In,
                "is" => ExpressionTokenType::Is,
                "like" => ExpressionTokenType::Like,
                "contains" => ExpressionTokenType::Contains,
                "prefix" => ExpressionTokenType::Prefix,
                "suffix" => ExpressionTokenType::Suffix,
                "regex" | "matches" => ExpressionTokenType::Regex,
                "exact" => ExpressionTokenType::Exact,
                "null" => ExpressionTokenType::Null,
                "true" => ExpressionTokenType::True,
                "false" => ExpressionTokenType::False,
                _ => ExpressionTokenType::Identifier,
            };
            ExpressionToken::new(token_type, value, start)
        }
    }

    /// 表达式解析器。
    /// Expression parser.
    pub struct ExpressionParser {
        tokens: Vec<ExpressionToken>,
        position: usize,
    }

    impl ExpressionParser {
        /// 创建表达式解析器。
        /// Create an expression parser.
        pub fn new(tokens: Vec<ExpressionToken>) -> Self {
            Self {
                tokens,
                position: 0,
            }
        }

        /// 解析布尔表达式。
        /// Parse a boolean expression.
        pub fn parse(&mut self) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            if self.tokens.is_empty()
                || (self.tokens.len() == 1
                    && self.current_token().token_type == ExpressionTokenType::Eof)
            {
                return Err(ExpressionParseError::new("empty expression", 0));
            }
            let expression = self.parse_or_expression()?;
            if self.current_token().token_type != ExpressionTokenType::Eof {
                return Err(ExpressionParseError::new(
                    format!("unexpected token: {}", self.current_token().value),
                    self.current_token().position,
                ));
            }
            Ok(expression)
        }

        fn parse_or_expression(&mut self) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            let mut left = self.parse_and_expression()?;
            while self.current_token().token_type == ExpressionTokenType::Or {
                self.advance();
                let right = self.parse_and_expression()?;
                left = merge_or(left, right);
            }
            Ok(left)
        }

        fn parse_and_expression(
            &mut self,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            let mut left = self.parse_not_expression()?;
            while self.current_token().token_type == ExpressionTokenType::And {
                self.advance();
                let right = self.parse_not_expression()?;
                left = merge_and(left, right);
            }
            Ok(left)
        }

        fn parse_not_expression(
            &mut self,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            if self.current_token().token_type == ExpressionTokenType::Not {
                self.advance();
                return Ok(BooleanExpression::not_expr(self.parse_not_expression()?));
            }
            self.parse_primary_expression()
        }

        fn parse_primary_expression(
            &mut self,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            match self.current_token().token_type {
                ExpressionTokenType::LParen => {
                    self.advance();
                    let expression = self.parse_or_expression()?;
                    self.expect(ExpressionTokenType::RParen, "expected ')'")?;
                    Ok(expression)
                }
                ExpressionTokenType::True => {
                    self.advance();
                    Ok(BooleanExpression::true_constant())
                }
                ExpressionTokenType::False => {
                    self.advance();
                    Ok(BooleanExpression::false_constant())
                }
                ExpressionTokenType::Identifier => self.parse_path_expression(),
                _ => Err(ExpressionParseError::new(
                    format!("unexpected token: {}", self.current_token().value),
                    self.current_token().position,
                )),
            }
        }

        fn parse_path_expression(
            &mut self,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            let path = self.parse_path()?;
            match self.current_token().token_type {
                ExpressionTokenType::Is => self.parse_null_check(path),
                ExpressionTokenType::Not => {
                    self.advance();
                    if self.current_token().token_type == ExpressionTokenType::In {
                        self.advance();
                        self.parse_in_expression(path, true)
                    } else if self.current_token().token_type.is_pattern_operator() {
                        let mode = self
                            .current_token()
                            .token_type
                            .pattern_match_mode()
                            .expect("token is pattern operator / token 是模式操作符");
                        self.advance();
                        self.parse_pattern_match(path, mode, true)
                    } else {
                        Err(ExpressionParseError::new(
                            "expected 'in' or pattern operator after 'not'",
                            self.current_token().position,
                        ))
                    }
                }
                ExpressionTokenType::In => {
                    self.advance();
                    self.parse_in_expression(path, false)
                }
                token_type if token_type.is_pattern_operator() => {
                    let mode = token_type.pattern_match_mode().expect("token is pattern operator / token 是模式操作符");
                    self.advance();
                    self.parse_pattern_match(path, mode, false)
                }
                token_type if token_type.is_comparison_operator() => self.parse_comparison(path),
                _ => Err(ExpressionParseError::new(
                    format!("expected comparison operator, 'in', or 'is' after '{path}'"),
                    self.current_token().position,
                )),
            }
        }

        fn parse_null_check(
            &mut self,
            path: PropertyPath,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            self.advance();
            let null_check_type = if self.current_token().token_type == ExpressionTokenType::Not {
                self.advance();
                NullCheckType::IsNotNull
            } else {
                NullCheckType::IsNull
            };
            self.expect(ExpressionTokenType::Null, "expected 'null' after 'is'")?;
            Ok(BooleanExpression::null_check(path, null_check_type))
        }

        fn parse_in_expression(
            &mut self,
            path: PropertyPath,
            negated: bool,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            self.expect(ExpressionTokenType::LParen, "expected '(' after 'in'")?;
            let mut candidates = Vec::new();
            loop {
                candidates.push(self.parse_scalar_value()?);
                if self.current_token().token_type != ExpressionTokenType::Comma {
                    break;
                }
                self.advance();
            }
            self.expect(ExpressionTokenType::RParen, "expected ')' after 'in' list")?;
            Ok(BooleanExpression::in_expr(
                ScalarExpression::reference(path),
                candidates,
                negated,
            ))
        }

        fn parse_pattern_match(
            &mut self,
            path: PropertyPath,
            mode: PatternMatchMode,
            negated: bool,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            let pattern = self.parse_scalar_value()?;
            Ok(BooleanExpression::pattern_match(
                ScalarExpression::reference(path),
                pattern,
                mode,
                negated,
            ))
        }

        fn parse_comparison(
            &mut self,
            left_path: PropertyPath,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            let operator = self
                .current_token()
                .token_type
                .comparison_operator()
                .ok_or_else(|| {
                    ExpressionParseError::new(
                        "expected comparison operator",
                        self.current_token().position,
                    )
                })?;
            self.advance();
            let right = self.parse_scalar_value()?;
            Ok(BooleanExpression::comparison(
                operator,
                ScalarExpression::reference(left_path),
                right,
            ))
        }

        fn parse_scalar_value(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
            match self.current_token().token_type {
                ExpressionTokenType::String => {
                    let value = self.current_token().value.clone();
                    self.advance();
                    Ok(ScalarExpression::constant(ExpressionValue::String(value)))
                }
                ExpressionTokenType::Number => {
                    let token = self.current_token();
                    self.advance();
                    let value = token.value.parse::<f64>().map_err(|_| {
                        ExpressionParseError::new("invalid number literal", token.position)
                    })?;
                    Ok(ScalarExpression::constant(ExpressionValue::Number(value)))
                }
                ExpressionTokenType::True => {
                    self.advance();
                    Ok(ScalarExpression::constant(ExpressionValue::Boolean(true)))
                }
                ExpressionTokenType::False => {
                    self.advance();
                    Ok(ScalarExpression::constant(ExpressionValue::Boolean(false)))
                }
                ExpressionTokenType::Null => {
                    self.advance();
                    Ok(ScalarExpression::constant(ExpressionValue::Null))
                }
                ExpressionTokenType::Identifier => {
                    Ok(ScalarExpression::reference(self.parse_path()?))
                }
                _ => Err(ExpressionParseError::new(
                    format!("expected scalar value, got: {}", self.current_token().value),
                    self.current_token().position,
                )),
            }
        }

        fn parse_path(&mut self) -> Result<PropertyPath, ExpressionParseError> {
            if self.current_token().token_type != ExpressionTokenType::Identifier {
                return Err(ExpressionParseError::new(
                    "expected identifier",
                    self.current_token().position,
                ));
            }
            let path = PropertyPath::parse(self.current_token().value.as_str());
            self.advance();
            Ok(path)
        }

        fn current_token(&self) -> ExpressionToken {
            self.tokens
                .get(self.position)
                .cloned()
                .unwrap_or_else(|| ExpressionToken::eof(self.position))
        }

        fn advance(&mut self) -> ExpressionToken {
            let token = self.current_token();
            self.position += 1;
            token
        }

        fn expect(
            &mut self,
            token_type: ExpressionTokenType,
            message: &'static str,
        ) -> Result<ExpressionToken, ExpressionParseError> {
            if self.current_token().token_type != token_type {
                return Err(ExpressionParseError::new(
                    message,
                    self.current_token().position,
                ));
            }
            Ok(self.advance())
        }
    }

    /// 将输入字符串切分为表达式词法单元。
    /// Tokenize input string into expression tokens.
    pub fn tokenize_expression(input: impl AsRef<str>) -> Vec<ExpressionToken> {
        ExpressionLexer::new(input).tokenize()
    }

    /// 解析布尔表达式字符串。
    /// Parse a boolean expression string.
    pub fn parse_boolean_expression(
        input: impl AsRef<str>,
    ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
        let tokens = tokenize_expression(input);
        ExpressionParser::new(tokens).parse()
    }

    /// 尝试解析布尔表达式字符串。
    /// Try to parse a boolean expression string.
    pub fn parse_boolean_expression_or_none(
        input: impl AsRef<str>,
    ) -> Option<ParsedBooleanExpression> {
        parse_boolean_expression(input).ok()
    }

    fn merge_or(
        left: ParsedBooleanExpression,
        right: ParsedBooleanExpression,
    ) -> ParsedBooleanExpression {
        let mut operands = Vec::new();
        if let BooleanExpression::Or(items) = left {
            operands.extend(items);
        } else {
            operands.push(left);
        }
        if let BooleanExpression::Or(items) = right {
            operands.extend(items);
        } else {
            operands.push(right);
        }
        BooleanExpression::Or(operands)
    }

    fn merge_and(
        left: ParsedBooleanExpression,
        right: ParsedBooleanExpression,
    ) -> ParsedBooleanExpression {
        let mut operands = Vec::new();
        if let BooleanExpression::And(items) = left {
            operands.extend(items);
        } else {
            operands.push(left);
        }
        if let BooleanExpression::And(items) = right {
            operands.extend(items);
        } else {
            operands.push(right);
        }
        BooleanExpression::And(operands)
    }
}

#[cfg(feature = "parser")]
pub use parser_support::{
    ExpressionLexer, ExpressionParseError, ExpressionParser, ExpressionToken, ExpressionTokenType,
    parse_boolean_expression, parse_boolean_expression_or_none, tokenize_expression,
};

#[cfg(feature = "serde")]
mod serde_support {
    use super::*;
    use std::any::Any;
    use std::fmt::{Display, Formatter};
    use crate::Trivalent;
    use crate::symbol::{DynSymbol, OwnedSymbol, SymbolDynId};
    use super::property_path::{PropertyPath, path_symbol_id, path_owned_symbol, stable_path_symbol_hash};
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};

    /// 表达式 JSON 错误。
    /// Expression JSON error.
    #[derive(Debug)]
    pub enum ExpressionJsonError {
        /// JSON 编码或解码失败。
        /// JSON encoding or decoding failed.
        Json(serde_json::Error),
        /// 操作符名称无效。
        /// Operator name is invalid.
        InvalidOperator {
            /// 操作符类别 / Operator kind
            kind: &'static str,
            /// 操作符名称 / Operator name
            value: String,
        },
    }

    impl Display for ExpressionJsonError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Json(error) => write!(f, "json error: {error}"),
                Self::InvalidOperator { kind, value } => {
                    write!(f, "invalid {kind} operator: {value}")
                }
            }
        }
    }

    impl std::error::Error for ExpressionJsonError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Self::Json(error) => Some(error),
                Self::InvalidOperator { .. } => None,
            }
        }
    }

    impl From<serde_json::Error> for ExpressionJsonError {
        fn from(error: serde_json::Error) -> Self {
            Self::Json(error)
        }
    }

    /// 轻量符号，用于恢复非路径符号引用。
    /// Lightweight symbol used to restore non-path symbol references.
    #[derive(Debug, Clone)]
    struct PlainExpressionSymbol {
        name: String,
        id: usize,
    }

    impl PlainExpressionSymbol {
        fn new(name: impl Into<String>) -> Self {
            let name = name.into();
            let id = stable_path_symbol_hash(name.as_bytes());
            Self { name, id }
        }
    }

    impl Display for PlainExpressionSymbol {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name)
        }
    }

    impl DynSymbol for PlainExpressionSymbol {
        fn name(&self) -> &str {
            &self.name
        }

        fn display_name(&self) -> &str {
            &self.name
        }

        fn dyn_id(&self) -> SymbolDynId<'_> {
            SymbolDynId::standalone(self.id)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type")]
    enum ScalarExpressionData<T> {
        #[serde(rename = "Constant")]
        Constant { value: T },
        #[serde(rename = "Reference")]
        Reference { path: String },
        #[serde(rename = "SymbolReference")]
        SymbolReference { identifier: String },
        #[serde(rename = "Unary")]
        Unary {
            operator: String,
            operand: Box<ScalarExpressionData<T>>,
        },
        #[serde(rename = "Binary")]
        Binary {
            operator: String,
            left: Box<ScalarExpressionData<T>>,
            right: Box<ScalarExpressionData<T>>,
        },
        #[serde(rename = "Function")]
        Function {
            name: String,
            arguments: Vec<ScalarExpressionData<T>>,
        },
        #[serde(rename = "Custom")]
        Custom {
            payload: Option<String>,
            description: Option<String>,
        },
        #[serde(rename = "Conditional")]
        Conditional {
            condition: Box<BooleanExpressionData<T>>,
            then_branch: Box<ScalarExpressionData<T>>,
            else_branch: Box<ScalarExpressionData<T>>,
        },
        #[serde(rename = "Boolean")]
        Boolean {
            expr: Box<BooleanExpressionData<T>>,
        },
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type")]
    enum BooleanExpressionData<T> {
        #[serde(rename = "BooleanConstant")]
        BooleanConstant { value: String },
        #[serde(rename = "Comparison")]
        Comparison {
            operator: String,
            left: ScalarExpressionData<T>,
            right: ScalarExpressionData<T>,
        },
        #[serde(rename = "In")]
        In {
            value: ScalarExpressionData<T>,
            candidates: Vec<ScalarExpressionData<T>>,
            #[serde(default)]
            negated: bool,
        },
        #[serde(rename = "PatternMatch")]
        PatternMatch {
            value: ScalarExpressionData<T>,
            pattern: ScalarExpressionData<T>,
            mode: String,
            #[serde(default)]
            negated: bool,
        },
        #[serde(rename = "NullCheck")]
        NullCheck {
            path: String,
            #[serde(rename = "nullCheckType")]
            null_check_type: String,
        },
        #[serde(rename = "And")]
        And {
            operands: Vec<BooleanExpressionData<T>>,
        },
        #[serde(rename = "Or")]
        Or {
            operands: Vec<BooleanExpressionData<T>>,
        },
        #[serde(rename = "Not")]
        Not {
            operand: Box<BooleanExpressionData<T>>,
        },
        #[serde(rename = "Custom")]
        Custom {
            payload: Option<String>,
            description: Option<String>,
        },
    }

    impl<T: Clone> From<&ScalarExpression<T>> for ScalarExpressionData<T> {
        fn from(value: &ScalarExpression<T>) -> Self {
            match value {
                ScalarExpression::Constant(value) => Self::Constant {
                    value: value.clone(),
                },
                ScalarExpression::Reference(path) => Self::Reference {
                    path: path.value().to_string(),
                },
                ScalarExpression::SymbolReference(symbol) => Self::SymbolReference {
                    identifier: symbol_identifier(symbol),
                },
                ScalarExpression::Unary { operator, operand } => Self::Unary {
                    operator: unary_operator_name(*operator).to_string(),
                    operand: Box::new(ScalarExpressionData::from(operand.as_ref())),
                },
                ScalarExpression::Binary {
                    operator,
                    left,
                    right,
                } => Self::Binary {
                    operator: binary_operator_name(*operator).to_string(),
                    left: Box::new(ScalarExpressionData::from(left.as_ref())),
                    right: Box::new(ScalarExpressionData::from(right.as_ref())),
                },
                ScalarExpression::Function { name, arguments } => Self::Function {
                    name: name.clone(),
                    arguments: arguments.iter().map(ScalarExpressionData::from).collect(),
                },
                ScalarExpression::Custom {
                    payload,
                    description,
                } => Self::Custom {
                    payload: Some(payload.clone()),
                    description: description.clone(),
                },
                ScalarExpression::Conditional {
                    condition,
                    then_branch,
                    else_branch,
                } => Self::Conditional {
                    condition: Box::new(BooleanExpressionData::from(condition.as_ref())),
                    then_branch: Box::new(ScalarExpressionData::from(then_branch.as_ref())),
                    else_branch: Box::new(ScalarExpressionData::from(else_branch.as_ref())),
                },
                ScalarExpression::Boolean(expr) => Self::Boolean {
                    expr: Box::new(BooleanExpressionData::from(expr.as_ref())),
                },
            }
        }
    }

    impl<T> TryFrom<ScalarExpressionData<T>> for ScalarExpression<T> {
        type Error = ExpressionJsonError;

        fn try_from(value: ScalarExpressionData<T>) -> Result<Self, Self::Error> {
            match value {
                ScalarExpressionData::Constant { value } => Ok(Self::Constant(value)),
                ScalarExpressionData::Reference { path } => {
                    Ok(Self::Reference(PropertyPath::parse(path)))
                }
                ScalarExpressionData::SymbolReference { identifier } => {
                    Ok(Self::SymbolReference(symbol_from_identifier(&identifier)))
                }
                ScalarExpressionData::Unary { operator, operand } => Ok(Self::Unary {
                    operator: parse_unary_operator(&operator)?,
                    operand: Box::new(ScalarExpression::try_from(*operand)?),
                }),
                ScalarExpressionData::Binary {
                    operator,
                    left,
                    right,
                } => Ok(Self::Binary {
                    operator: parse_binary_operator(&operator)?,
                    left: Box::new(ScalarExpression::try_from(*left)?),
                    right: Box::new(ScalarExpression::try_from(*right)?),
                }),
                ScalarExpressionData::Function { name, arguments } => Ok(Self::Function {
                    name,
                    arguments: arguments
                        .into_iter()
                        .map(ScalarExpression::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                }),
                ScalarExpressionData::Custom {
                    payload,
                    description,
                } => Ok(Self::Custom {
                    payload: payload.unwrap_or_default(),
                    description,
                }),
                ScalarExpressionData::Conditional {
                    condition,
                    then_branch,
                    else_branch,
                } => Ok(Self::Conditional {
                    condition: Box::new(BooleanExpression::try_from(*condition)?),
                    then_branch: Box::new(ScalarExpression::try_from(*then_branch)?),
                    else_branch: Box::new(ScalarExpression::try_from(*else_branch)?),
                }),
                ScalarExpressionData::Boolean { expr } => Ok(Self::Boolean(
                    Box::new(BooleanExpression::try_from(*expr)?),
                )),
            }
        }
    }

    impl<T: Clone> From<&BooleanExpression<T>> for BooleanExpressionData<T> {
        fn from(value: &BooleanExpression<T>) -> Self {
            match value {
                BooleanExpression::Constant(value) => Self::BooleanConstant {
                    value: trivalent_name(*value).to_string(),
                },
                BooleanExpression::Comparison {
                    operator,
                    left,
                    right,
                } => Self::Comparison {
                    operator: comparison_operator_name(*operator).to_string(),
                    left: ScalarExpressionData::from(left),
                    right: ScalarExpressionData::from(right),
                },
                BooleanExpression::In {
                    value,
                    candidates,
                    negated,
                } => Self::In {
                    value: ScalarExpressionData::from(value),
                    candidates: candidates.iter().map(ScalarExpressionData::from).collect(),
                    negated: *negated,
                },
                BooleanExpression::PatternMatch {
                    value,
                    pattern,
                    mode,
                    negated,
                } => Self::PatternMatch {
                    value: ScalarExpressionData::from(value),
                    pattern: ScalarExpressionData::from(pattern),
                    mode: pattern_match_mode_name(*mode).to_string(),
                    negated: *negated,
                },
                BooleanExpression::NullCheck {
                    path,
                    null_check_type,
                } => Self::NullCheck {
                    path: path.value().to_string(),
                    null_check_type: null_check_type_name(*null_check_type).to_string(),
                },
                BooleanExpression::And(operands) => Self::And {
                    operands: operands.iter().map(BooleanExpressionData::from).collect(),
                },
                BooleanExpression::Or(operands) => Self::Or {
                    operands: operands.iter().map(BooleanExpressionData::from).collect(),
                },
                BooleanExpression::Not(operand) => Self::Not {
                    operand: Box::new(BooleanExpressionData::from(operand.as_ref())),
                },
                BooleanExpression::Custom {
                    payload,
                    description,
                } => Self::Custom {
                    payload: Some(payload.clone()),
                    description: description.clone(),
                },
            }
        }
    }

    impl<T> TryFrom<BooleanExpressionData<T>> for BooleanExpression<T> {
        type Error = ExpressionJsonError;

        fn try_from(value: BooleanExpressionData<T>) -> Result<Self, Self::Error> {
            match value {
                BooleanExpressionData::BooleanConstant { value } => {
                    Ok(Self::Constant(parse_trivalent(&value)))
                }
                BooleanExpressionData::Comparison {
                    operator,
                    left,
                    right,
                } => Ok(Self::Comparison {
                    operator: parse_comparison_operator(&operator)?,
                    left: ScalarExpression::try_from(left)?,
                    right: ScalarExpression::try_from(right)?,
                }),
                BooleanExpressionData::In {
                    value,
                    candidates,
                    negated,
                } => Ok(Self::In {
                    value: ScalarExpression::try_from(value)?,
                    candidates: candidates
                        .into_iter()
                        .map(ScalarExpression::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                    negated,
                }),
                BooleanExpressionData::PatternMatch {
                    value,
                    pattern,
                    mode,
                    negated,
                } => Ok(Self::PatternMatch {
                    value: ScalarExpression::try_from(value)?,
                    pattern: ScalarExpression::try_from(pattern)?,
                    mode: parse_pattern_match_mode(&mode)?,
                    negated,
                }),
                BooleanExpressionData::NullCheck {
                    path,
                    null_check_type,
                } => Ok(Self::NullCheck {
                    path: PropertyPath::parse(path),
                    null_check_type: parse_null_check_type(&null_check_type)?,
                }),
                BooleanExpressionData::And { operands } => Ok(Self::and(
                    operands
                        .into_iter()
                        .map(BooleanExpression::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                )),
                BooleanExpressionData::Or { operands } => Ok(Self::or(
                    operands
                        .into_iter()
                        .map(BooleanExpression::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                )),
                BooleanExpressionData::Not { operand } => {
                    Ok(Self::not_expr(BooleanExpression::try_from(*operand)?))
                }
                BooleanExpressionData::Custom {
                    payload,
                    description,
                } => Ok(Self::Custom {
                    payload: payload.unwrap_or_default(),
                    description,
                }),
            }
        }
    }

    impl<T: Clone + Serialize> ScalarExpression<T> {
        /// 转换为紧凑 JSON 字符串。
        /// Convert to a compact JSON string.
        pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string(&ScalarExpressionData::from(self))
        }

        /// 转换为格式化 JSON 字符串。
        /// Convert to a pretty JSON string.
        pub fn to_json_string_pretty(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string_pretty(&ScalarExpressionData::from(self))
        }
    }

    impl<T: Clone + Serialize> BooleanExpression<T> {
        /// 转换为紧凑 JSON 字符串。
        /// Convert to a compact JSON string.
        pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string(&BooleanExpressionData::from(self))
        }

        /// 转换为格式化 JSON 字符串。
        /// Convert to a pretty JSON string.
        pub fn to_json_string_pretty(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string_pretty(&BooleanExpressionData::from(self))
        }
    }

    /// 从 JSON 字符串反序列化标量表达式。
    /// Deserialize a scalar expression from a JSON string.
    pub fn scalar_expression_from_json<T>(
        json: &str,
    ) -> Result<ScalarExpression<T>, ExpressionJsonError>
    where
        T: DeserializeOwned,
    {
        let data: ScalarExpressionData<T> = serde_json::from_str(json)?;
        ScalarExpression::try_from(data)
    }

    /// 尝试从 JSON 字符串反序列化标量表达式。
    /// Try to deserialize a scalar expression from a JSON string.
    pub fn scalar_expression_from_json_or_none<T>(json: &str) -> Option<ScalarExpression<T>>
    where
        T: DeserializeOwned,
    {
        scalar_expression_from_json(json).ok()
    }

    /// 从 JSON 字符串反序列化布尔表达式。
    /// Deserialize a boolean expression from a JSON string.
    pub fn boolean_expression_from_json<T>(
        json: &str,
    ) -> Result<BooleanExpression<T>, ExpressionJsonError>
    where
        T: DeserializeOwned,
    {
        let data: BooleanExpressionData<T> = serde_json::from_str(json)?;
        BooleanExpression::try_from(data)
    }

    /// 尝试从 JSON 字符串反序列化布尔表达式。
    /// Try to deserialize a boolean expression from a JSON string.
    pub fn boolean_expression_from_json_or_none<T>(json: &str) -> Option<BooleanExpression<T>>
    where
        T: DeserializeOwned,
    {
        boolean_expression_from_json(json).ok()
    }

    fn symbol_identifier(symbol: &OwnedSymbol) -> String {
        if let Some(path) = property_path_from_owned_symbol(symbol) {
            path_symbol_id(path)
        } else {
            symbol.name().to_string()
        }
    }

    fn symbol_from_identifier(identifier: &str) -> OwnedSymbol {
        if let Some(path) = identifier.strip_prefix("path:") {
            path_owned_symbol(path)
        } else {
            OwnedSymbol::new(PlainExpressionSymbol::new(identifier))
        }
    }

    fn invalid_operator(kind: &'static str, value: &str) -> ExpressionJsonError {
        ExpressionJsonError::InvalidOperator {
            kind,
            value: value.to_string(),
        }
    }

    fn unary_operator_name(operator: UnaryOperator) -> &'static str {
        match operator {
            UnaryOperator::Negate => "Negate",
            UnaryOperator::Positive => "Positive",
            UnaryOperator::Abs => "Abs",
        }
    }

    fn parse_unary_operator(value: &str) -> Result<UnaryOperator, ExpressionJsonError> {
        match value {
            "Negate" => Ok(UnaryOperator::Negate),
            "Positive" => Ok(UnaryOperator::Positive),
            "Abs" => Ok(UnaryOperator::Abs),
            _ => Err(invalid_operator("unary", value)),
        }
    }

    fn binary_operator_name(operator: BinaryOperator) -> &'static str {
        match operator {
            BinaryOperator::Add => "Add",
            BinaryOperator::Subtract => "Subtract",
            BinaryOperator::Multiply => "Multiply",
            BinaryOperator::Divide => "Divide",
            BinaryOperator::Modulo => "Modulo",
            BinaryOperator::Power => "Power",
        }
    }

    fn parse_binary_operator(value: &str) -> Result<BinaryOperator, ExpressionJsonError> {
        match value {
            "Add" => Ok(BinaryOperator::Add),
            "Subtract" => Ok(BinaryOperator::Subtract),
            "Multiply" => Ok(BinaryOperator::Multiply),
            "Divide" => Ok(BinaryOperator::Divide),
            "Modulo" => Ok(BinaryOperator::Modulo),
            "Power" => Ok(BinaryOperator::Power),
            _ => Err(invalid_operator("binary", value)),
        }
    }

    fn comparison_operator_name(operator: ComparisonOperator) -> &'static str {
        match operator {
            ComparisonOperator::Eq => "Eq",
            ComparisonOperator::Ne => "Ne",
            ComparisonOperator::Lt => "Lt",
            ComparisonOperator::Le => "Le",
            ComparisonOperator::Gt => "Gt",
            ComparisonOperator::Ge => "Ge",
        }
    }

    fn parse_comparison_operator(value: &str) -> Result<ComparisonOperator, ExpressionJsonError> {
        match value {
            "Eq" => Ok(ComparisonOperator::Eq),
            "Ne" => Ok(ComparisonOperator::Ne),
            "Lt" => Ok(ComparisonOperator::Lt),
            "Le" => Ok(ComparisonOperator::Le),
            "Gt" => Ok(ComparisonOperator::Gt),
            "Ge" => Ok(ComparisonOperator::Ge),
            _ => Err(invalid_operator("comparison", value)),
        }
    }

    fn pattern_match_mode_name(mode: PatternMatchMode) -> &'static str {
        match mode {
            PatternMatchMode::Exact => "Exact",
            PatternMatchMode::Prefix => "Prefix",
            PatternMatchMode::Suffix => "Suffix",
            PatternMatchMode::Contains => "Contains",
            PatternMatchMode::Like => "Like",
            PatternMatchMode::Regex => "Regex",
        }
    }

    fn parse_pattern_match_mode(value: &str) -> Result<PatternMatchMode, ExpressionJsonError> {
        match value {
            "Exact" => Ok(PatternMatchMode::Exact),
            "Prefix" => Ok(PatternMatchMode::Prefix),
            "Suffix" => Ok(PatternMatchMode::Suffix),
            "Contains" => Ok(PatternMatchMode::Contains),
            "Like" => Ok(PatternMatchMode::Like),
            "Regex" => Ok(PatternMatchMode::Regex),
            _ => Err(invalid_operator("pattern match", value)),
        }
    }

    fn null_check_type_name(null_check_type: NullCheckType) -> &'static str {
        match null_check_type {
            NullCheckType::IsNull => "IsNull",
            NullCheckType::IsNotNull => "IsNotNull",
        }
    }

    fn parse_null_check_type(value: &str) -> Result<NullCheckType, ExpressionJsonError> {
        match value {
            "IsNull" => Ok(NullCheckType::IsNull),
            "IsNotNull" => Ok(NullCheckType::IsNotNull),
            _ => Err(invalid_operator("null check", value)),
        }
    }

    fn trivalent_name(value: Trivalent) -> &'static str {
        match value {
            Trivalent::True => "true",
            Trivalent::False => "false",
            Trivalent::Unknown => "unknown",
        }
    }

    fn parse_trivalent(value: &str) -> Trivalent {
        match value.to_ascii_lowercase().as_str() {
            "true" => Trivalent::True,
            "false" => Trivalent::False,
            _ => Trivalent::Unknown,
        }
    }
}

#[cfg(feature = "serde")]
pub use serde_support::{
    ExpressionJsonError, boolean_expression_from_json, boolean_expression_from_json_or_none,
    scalar_expression_from_json, scalar_expression_from_json_or_none,
};

pub use math_functions::{
    CompositeFunctionEvaluator, DefaultScalarFunctionEvaluator, MathFunctionEvaluator,
    ScalarFunctionEvaluator,
};

#[cfg(feature = "parser")]
pub use scalar_parser::{parse_scalar_expression, parse_scalar_expression_or_none};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Trivalent;
    use crate::symbol::DynSymbol;

    #[test]
    fn property_path_matches_kotlin_shape() {
        let path = PropertyPath::parse(" user.address.city ");

        assert_eq!(path.value(), "user.address.city");
        assert_eq!(path.segments(), vec!["user", "address", "city"]);
        assert_eq!(path.depth(), 3);
        assert_eq!(path.root(), Some("user"));
        assert_eq!(path.leaf(), Some("city"));
        assert_eq!(path.parent(), Some(PropertyPath::parse("user.address")));
        assert_eq!(path.child(), Some(PropertyPath::parse("address.city")));
        assert!(path.is_sub_path_of(&PropertyPath::parse("user.address")));
        assert!(PropertyPath::parse("user").is_parent_path_of(&path));
        assert_eq!(
            PropertyPath::parse("user").concat_segment("name"),
            PropertyPath::parse("user.name")
        );
    }

    #[test]
    fn property_path_validates_identifiers() {
        assert_eq!(
            PropertyPath::parse_or_none("user.address_1"),
            Some(PropertyPath::parse("user.address_1"))
        );
        assert_eq!(PropertyPath::parse_or_none("1user.address"), None);
        assert_eq!(PropertyPath::parse_or_none("user..address"), None);
        assert_eq!(PropertyPath::parse_or_none(""), None);
    }

    #[test]
    fn path_symbol_bridges_to_owned_symbol() {
        let path = PropertyPath::parse("user.age");
        let symbol = PathSymbol::from_path(path.clone());
        let owned = symbol.clone().into_owned_symbol();

        assert_eq!(symbol.name(), "user.age");
        assert_eq!(symbol.display_name(), "user.age");
        assert_eq!(symbol.symbol_id(), "path:user.age");
        assert_eq!(path_symbol_id(&path), "path:user.age");
        assert_eq!(property_path_from_owned_symbol(&owned), Some(&path));
        assert_eq!(owned.name(), "user.age");
    }

    #[test]
    fn scalar_expression_reports_references_and_depth() {
        let expression = ScalarExpression::<i32>::add_expr(
            ScalarExpression::reference("user.age"),
            ScalarExpression::function(
                ScalarFunctionNames::ABS,
                vec![ScalarExpression::symbol_reference(path_owned_symbol(
                    "order.price",
                ))],
            ),
        );

        let references = expression.collect_references();
        assert_eq!(expression.type_name(), "Binary");
        assert!(!expression.is_constant());
        assert!(expression.contains_reference());
        assert_eq!(expression.depth(), 3);
        assert!(references.contains(&PropertyPath::parse("user.age")));
        assert!(references.contains(&PropertyPath::parse("order.price")));
    }

    #[test]
    fn boolean_expression_reports_structure() {
        let age = ScalarExpression::<i32>::reference("user.age");
        let adult = BooleanExpression::ge(age, ScalarExpression::constant(18));
        let named = BooleanExpression::is_not_null("user.name");
        let expression = BooleanExpression::and(vec![adult, BooleanExpression::not_expr(named)]);

        assert_eq!(expression.type_name(), "And");
        assert_eq!(expression.logical_operator_count(), 3);
        assert_eq!(expression.depth(), 3);
        assert!(!expression.is_constant());
        assert!(!expression.is_pure_logical());

        let references = expression.collect_references();
        assert!(references.contains(&PropertyPath::parse("user.age")));
        assert!(references.contains(&PropertyPath::parse("user.name")));
    }

    #[test]
    fn operator_symbols_and_inverse_are_kotlin_compatible() {
        assert_eq!(UnaryOperator::Abs.symbol(), "abs");
        assert_eq!(BinaryOperator::Power.symbol(), "^");
        assert_eq!(ComparisonOperator::Le.symbol(), "<=");
        assert_eq!(ComparisonOperator::Lt.inverse(), ComparisonOperator::Gt);
        assert_eq!(BooleanOperator::And.symbol(), "and");
        assert_eq!(NullCheckType::IsNotNull.symbol(), "is not null");
    }

    #[test]
    fn path_builder_builds_runtime_expression() {
        let expression = boolean_expression(|| {
            path("age").ge(18) & path("status").eq("active") & !path("deleted_at").is_null()
        });

        let BooleanExpression::And(operands) = expression else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 3);
        assert_eq!(
            operands[0],
            BooleanExpression::ge(
                ScalarExpression::reference("age"),
                ScalarExpression::constant(ExpressionValue::Number(18.0)),
            )
        );
        assert_eq!(
            operands[1],
            BooleanExpression::eq(
                ScalarExpression::reference("status"),
                ScalarExpression::constant(ExpressionValue::String("active".to_string())),
            )
        );
        assert!(matches!(operands[2], BooleanExpression::Not(_)));
    }

    #[test]
    fn path_builder_builds_typed_expression() {
        let expression = typed_path::<i32>("age").ge(18)
            & typed_path::<i32>("score").lt(typed_path::<i32>("limit").as_scalar());

        let references = expression.collect_references();
        assert!(references.contains(&PropertyPath::parse("age")));
        assert!(references.contains(&PropertyPath::parse("score")));
        assert!(references.contains(&PropertyPath::parse("limit")));
        assert!(matches!(expression, BooleanExpression::And(_)));
    }

    #[test]
    fn quick_expression_constructors_use_runtime_values() {
        let expression: ParsedBooleanExpression = eq("age", 18)
            .and_expr(in_expr("status", ["active", "pending"]))
            .and_expr(is_not_null("profile.email"));

        let BooleanExpression::And(operands) = expression else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 3);
        assert!(matches!(operands[1], BooleanExpression::In { .. }));
        assert!(matches!(operands[2], BooleanExpression::NullCheck { .. }));
    }

    #[test]
    fn scalar_function_dsl_builds_comparable_expression() {
        let expression = abs(path("delta")).gt_expr(0);

        let BooleanExpression::Comparison {
            operator,
            left,
            right,
        } = expression
        else {
            panic!("expected comparison expression");
        };
        assert_eq!(operator, ComparisonOperator::Gt);
        assert_eq!(
            left,
            ScalarExpression::function(
                ScalarFunctionNames::ABS,
                vec![ScalarExpression::reference("delta")],
            )
        );
        assert_eq!(
            right,
            ScalarExpression::constant(ExpressionValue::Number(0.0))
        );
    }

    #[test]
    fn runtime_boolean_expression_evaluates_with_context() {
        let expression = path("age").ge(18)
            & lower(path("status").as_scalar()).eq_expr("active")
            & path("name").like("A%")
            & abs(path("delta").as_scalar()).ge_expr(3);
        let context = MapEvaluationContext::from_string_map([
            ("age", ExpressionValue::from(20)),
            ("status", ExpressionValue::from("ACTIVE")),
            ("name", ExpressionValue::from("Alice")),
            ("delta", ExpressionValue::from(-3)),
        ]);

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::True);
        assert_eq!(expression.evaluate_with_or_none(&context), Some(true));
    }

    #[test]
    fn runtime_boolean_expression_preserves_unknown_semantics() {
        let expression = path("deleted_at").is_null() | path("score").gt(90);
        let context = MapEvaluationContext::from_string_map([("score", ExpressionValue::from(80))]);

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::Unknown);

        let expression = path("code").regex("^A[0-9]+$");
        let context =
            MapEvaluationContext::from_string_map([("code", ExpressionValue::from("A12"))]);
        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::True);
    }

    #[test]
    fn boolean_expression_normalizes_like_kotlin_operation_layer() {
        let adult = path("age").ge(18);
        let duplicated = BooleanExpression::and(vec![
            BooleanExpression::true_constant(),
            adult.clone(),
            BooleanExpression::and(vec![adult.clone(), BooleanExpression::true_constant()]),
        ]);

        assert_eq!(duplicated.normalize(), adult);

        let demorgan = BooleanExpression::not_expr(BooleanExpression::and(vec![
            path("a").eq(1),
            path("b").eq(2),
        ]));
        let normalized = demorgan.normalize_with_config(NormalizeConfig {
            apply_de_morgan: true,
            ..NormalizeConfig::default()
        });
        assert!(matches!(normalized, BooleanExpression::Or(_)));
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_lexer_tokenizes_paths_literals_and_keywords() {
        let tokens = tokenize_expression(r#"user.name like "A\n%" and score >= -1.25e+2"#);
        let token_types = tokens
            .iter()
            .map(|token| token.token_type)
            .collect::<Vec<_>>();

        assert_eq!(
            token_types,
            vec![
                ExpressionTokenType::Identifier,
                ExpressionTokenType::Like,
                ExpressionTokenType::String,
                ExpressionTokenType::And,
                ExpressionTokenType::Identifier,
                ExpressionTokenType::Ge,
                ExpressionTokenType::Number,
                ExpressionTokenType::Eof,
            ]
        );
        assert_eq!(tokens[0].value, "user.name");
        assert_eq!(tokens[2].value, "A\n%");
        assert_eq!(tokens[6].value, "-1.25e+2");
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_parses_comparison() {
        let expression = parse_boolean_expression("age >= 18").unwrap();

        assert_eq!(
            expression,
            BooleanExpression::ge(
                ScalarExpression::reference("age"),
                ScalarExpression::constant(ExpressionValue::Number(18.0)),
            )
        );
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_preserves_logic_precedence() {
        let expression =
            parse_boolean_expression("age >= 18 and status = 'active' or vip = true").unwrap();

        let BooleanExpression::Or(or_operands) = expression else {
            panic!("expected top-level OR expression");
        };
        assert_eq!(or_operands.len(), 2);
        assert!(matches!(or_operands[0], BooleanExpression::And(_)));
        assert_eq!(
            or_operands[1],
            BooleanExpression::eq(
                ScalarExpression::reference("vip"),
                ScalarExpression::constant(ExpressionValue::Boolean(true)),
            )
        );
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_parses_parentheses_and_not() {
        let expression = parse_boolean_expression("not (age < 18 or banned = true)").unwrap();

        let BooleanExpression::Not(inner) = expression else {
            panic!("expected NOT expression");
        };
        let BooleanExpression::Or(operands) = inner.as_ref() else {
            panic!("expected grouped OR expression");
        };
        assert_eq!(operands.len(), 2);
        assert_eq!(
            operands[0],
            BooleanExpression::lt(
                ScalarExpression::reference("age"),
                ScalarExpression::constant(ExpressionValue::Number(18.0)),
            )
        );
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_parses_in_and_not_in() {
        let expression = parse_boolean_expression("status in ('active', 'pending')").unwrap();
        let BooleanExpression::In {
            value,
            candidates,
            negated,
        } = expression
        else {
            panic!("expected IN expression");
        };
        assert_eq!(value, ScalarExpression::reference("status"));
        assert_eq!(
            candidates,
            vec![
                ScalarExpression::constant(ExpressionValue::String("active".to_string())),
                ScalarExpression::constant(ExpressionValue::String("pending".to_string())),
            ]
        );
        assert!(!negated);

        let expression = parse_boolean_expression("status not in ('archived')").unwrap();
        assert!(matches!(
            expression,
            BooleanExpression::In { negated: true, .. }
        ));
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_parses_null_checks() {
        assert_eq!(
            parse_boolean_expression("deleted_at is null").unwrap(),
            BooleanExpression::is_null("deleted_at")
        );
        assert_eq!(
            parse_boolean_expression("profile.email is not null").unwrap(),
            BooleanExpression::is_not_null("profile.email")
        );
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_parses_pattern_matches() {
        let expression = parse_boolean_expression("name like 'A%'").unwrap();
        assert_eq!(
            expression,
            BooleanExpression::pattern_match(
                ScalarExpression::reference("name"),
                ScalarExpression::constant(ExpressionValue::String("A%".to_string())),
                PatternMatchMode::Like,
                false,
            )
        );

        let expression = parse_boolean_expression("name not regex '^A'").unwrap();
        assert_eq!(
            expression,
            BooleanExpression::pattern_match(
                ScalarExpression::reference("name"),
                ScalarExpression::constant(ExpressionValue::String("^A".to_string())),
                PatternMatchMode::Regex,
                true,
            )
        );
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_reports_invalid_input() {
        let error = parse_boolean_expression("age >").unwrap_err();

        assert!(error.message().contains("expected scalar value"));
        assert!(parse_boolean_expression_or_none("age >").is_none());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn scalar_expression_json_matches_kotlin_shape() {
        let expression = ScalarExpression::<i32>::add_expr(
            ScalarExpression::reference("user.age"),
            ScalarExpression::symbol_reference(path_owned_symbol("order.price")),
        );

        let json = expression.to_json_string().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "Binary");
        assert_eq!(value["operator"], "Add");
        assert_eq!(value["left"]["type"], "Reference");
        assert_eq!(value["left"]["path"], "user.age");
        assert_eq!(value["right"]["type"], "SymbolReference");
        assert_eq!(value["right"]["identifier"], "path:order.price");

        let restored = scalar_expression_from_json::<i32>(&json).unwrap();
        let references = restored.collect_references();
        assert!(references.contains(&PropertyPath::parse("user.age")));
        assert!(references.contains(&PropertyPath::parse("order.price")));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn boolean_expression_json_round_trips() {
        let age = ScalarExpression::<i32>::reference("user.age");
        let adult = BooleanExpression::ge(age, ScalarExpression::constant(18));
        let named = BooleanExpression::is_not_null("user.name");
        let expression = BooleanExpression::and(vec![adult, BooleanExpression::not_expr(named)]);

        let json = expression.to_json_string().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "And");
        assert_eq!(value["operands"][0]["type"], "Comparison");
        assert_eq!(value["operands"][0]["operator"], "Ge");
        assert_eq!(value["operands"][1]["type"], "Not");
        assert_eq!(value["operands"][1]["operand"]["type"], "NullCheck");
        assert_eq!(
            value["operands"][1]["operand"]["nullCheckType"],
            "IsNotNull"
        );

        let restored = boolean_expression_from_json::<i32>(&json).unwrap();
        assert_eq!(restored.logical_operator_count(), 3);
        assert_eq!(restored.depth(), 3);
        assert!(
            restored
                .collect_references()
                .contains(&PropertyPath::parse("user.name"))
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn boolean_expression_json_rejects_invalid_operator() {
        let json = r#"{"type":"Comparison","operator":"Bad","left":{"type":"Constant","value":1},"right":{"type":"Constant","value":2}}"#;
        assert!(boolean_expression_from_json::<i32>(json).is_err());
        assert!(boolean_expression_from_json_or_none::<i32>(json).is_none());
    }
}
