//! 标量表达式解析器
//! Scalar expression parser
//!
//! 将标量表达式字符串解析为 `ScalarExpression` AST。
//! Parses scalar expression strings into `ScalarExpression` AST.
//!
//! # 支持的语法 / Supported syntax
//!
//! - 算术操作: `+`, `-`, `*`, `/`, `%`, `^`, `**`
//! - 比较操作: `>`, `<`, `>=`, `<=`, `==`, `!=`
//! - 逻辑操作: `&&`, `||`, `!`, `and`, `or`, `not`
//! - 三元条件: `? :`
//! - if/then/else/fi 条件
//! - 函数调用: `name(args)`
//! - math.* 函数与常量
//!
//! # 优先级（从高到低）/ Precedence (high to low)
//!
//! 1. 原子（数字、标识符、括号）
//! 2. 幂运算 `^`, `**`（右结合）
//! 3. 一元正号 `+`
//! 4. 乘除模 `*`, `/`, `%`
//! 5. 加减 `+`, `-`（一元负号在此层处理）
//! 6. 比较 `>`, `<`, `>=`, `<=`, `==`, `!=`
//! 7. 逻辑与 `&&`, `and`
//! 8. 逻辑或 `||`, `or`
//! 9. 三元条件 `? :`, `if/then/else/fi`

use super::parser_support::ExpressionParseError;
use super::*;

// ============================================================================
// ScalarTokenType - 标量表达式词法单元类型
// ============================================================================

/// 标量表达式词法单元类型。
/// Scalar expression token type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ScalarTokenType {
    // 字面量 / Literals
    /// 数字字面量 / Number literal
    Number,
    /// 字符串字面量 / String literal
    String,
    /// 标识符或属性路径 / Identifier or property path
    Identifier,
    /// 真常量 / True constant
    True,
    /// 假常量 / False constant
    False,
    /// 空值 / Null value
    Null,

    // 算术操作符 / Arithmetic operators
    /// 加号 / Plus
    Plus,
    /// 减号 / Minus
    Minus,
    /// 乘号 / Star
    Star,
    /// 除号 / Slash
    Slash,
    /// 取模 / Percent
    Percent,
    /// 幂运算符 ^ / Caret (power)
    Caret,
    /// 幂运算符 ** / Double star (power)
    DoubleStar,

    // 比较操作符 / Comparison operators
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

    // 逻辑操作符 / Logical operators
    /// 逻辑与 && / Logical AND
    AmpersandAmpersand,
    /// 逻辑或 || / Logical OR
    PipePipe,
    /// 逻辑非 ! / Logical NOT
    Bang,
    /// 逻辑与关键字 / Logical AND keyword
    And,
    /// 逻辑或关键字 / Logical OR keyword
    Or,
    /// 逻辑非关键字 / Logical NOT keyword
    Not,

    // 条件关键字 / Conditional keywords
    /// if 关键字 / if keyword
    If,
    /// then 关键字 / then keyword
    Then,
    /// else 关键字 / else keyword
    Else,
    /// fi 关键字 / fi keyword
    Fi,

    // 条件操作符 / Conditional operators
    /// 问号（三元条件） / Question mark (ternary)
    Question,
    /// 冒号（三元条件） / Colon (ternary)
    Colon,

    // 其他符号 / Other symbols
    /// 左括号 / Left parenthesis
    LParen,
    /// 右括号 / Right parenthesis
    RParen,
    /// 逗号 / Comma
    Comma,
    /// 未知字符 / Unknown character
    Unknown,
    /// 文件结束 / End of file
    Eof,
}

impl ScalarTokenType {
    /// 转换为比较操作符。
    /// Convert to comparison operator.
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

    /// 判断是否是比较操作符。
    /// Check whether this is a comparison operator.
    fn is_comparison_operator(self) -> bool {
        self.comparison_operator().is_some()
    }
}

// ============================================================================
// ScalarToken - 标量表达式词法单元
// ============================================================================

/// 标量表达式词法单元。
/// Scalar expression token.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScalarToken {
    /// 词法单元类型 / Token type
    token_type: ScalarTokenType,
    /// 词法单元值 / Token value
    value: String,
    /// 词法单元位置 / Token position
    position: usize,
}

impl ScalarToken {
    fn new(token_type: ScalarTokenType, value: impl Into<String>, position: usize) -> Self {
        Self {
            token_type,
            value: value.into(),
            position,
        }
    }

    fn eof(position: usize) -> Self {
        Self::new(ScalarTokenType::Eof, "", position)
    }
}

// ============================================================================
// ScalarLexer - 标量表达式词法分析器
// ============================================================================

/// 标量表达式词法分析器。
/// Scalar expression lexer.
///
/// 与布尔表达式词法分析器不同，负号始终产生独立的 Minus token，
/// 不合并到数字字面量中。
/// Unlike the boolean expression lexer, the minus sign always produces
/// a separate Minus token, not merged into the number literal.
struct ScalarLexer {
    input: Vec<char>,
    position: usize,
}

impl ScalarLexer {
    /// 创建标量表达式词法分析器。
    /// Create a scalar expression lexer.
    pub fn new(input: impl AsRef<str>) -> Self {
        Self {
            input: input.as_ref().chars().collect(),
            position: 0,
        }
    }

    /// 分析完整输入并返回词法单元列表。
    /// Tokenize the whole input and return token list.
    pub fn tokenize(&mut self) -> Vec<ScalarToken> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.token_type == ScalarTokenType::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
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

    /// 获取下一个词法单元。
    /// Get next token.
    pub fn next_token(&mut self) -> ScalarToken {
        self.skip_whitespace();
        let start = self.position;
        let Some(current) = self.current_char() else {
            return ScalarToken::eof(start);
        };

        // 字符串字面量 / String literals
        if current == '\'' || current == '"' {
            return self.read_string(start);
        }

        // 数字字面量（标量模式：负号不合并到数字）/ Number literals (scalar mode: minus not merged)
        if current.is_ascii_digit() {
            return self.read_number(start);
        }

        // 标识符和关键字 / Identifiers and keywords
        if current.is_alphabetic() || current == '_' {
            return self.read_identifier_or_keyword(start);
        }

        // 操作符和分隔符 / Operators and delimiters
        match current {
            '(' => {
                self.advance();
                ScalarToken::new(ScalarTokenType::LParen, "(", start)
            }
            ')' => {
                self.advance();
                ScalarToken::new(ScalarTokenType::RParen, ")", start)
            }
            ',' => {
                self.advance();
                ScalarToken::new(ScalarTokenType::Comma, ",", start)
            }
            '+' => {
                self.advance();
                ScalarToken::new(ScalarTokenType::Plus, "+", start)
            }
            '-' => {
                self.advance();
                ScalarToken::new(ScalarTokenType::Minus, "-", start)
            }
            '*' => {
                self.advance();
                if self.current_char() == Some('*') {
                    self.advance();
                    ScalarToken::new(ScalarTokenType::DoubleStar, "**", start)
                } else {
                    ScalarToken::new(ScalarTokenType::Star, "*", start)
                }
            }
            '/' => {
                self.advance();
                ScalarToken::new(ScalarTokenType::Slash, "/", start)
            }
            '%' => {
                self.advance();
                ScalarToken::new(ScalarTokenType::Percent, "%", start)
            }
            '^' => {
                self.advance();
                ScalarToken::new(ScalarTokenType::Caret, "^", start)
            }
            '?' => {
                self.advance();
                ScalarToken::new(ScalarTokenType::Question, "?", start)
            }
            ':' => {
                self.advance();
                ScalarToken::new(ScalarTokenType::Colon, ":", start)
            }
            '&' => {
                self.advance();
                if self.current_char() == Some('&') {
                    self.advance();
                    ScalarToken::new(ScalarTokenType::AmpersandAmpersand, "&&", start)
                } else {
                    ScalarToken::new(ScalarTokenType::Unknown, "&", start)
                }
            }
            '|' => {
                self.advance();
                if self.current_char() == Some('|') {
                    self.advance();
                    ScalarToken::new(ScalarTokenType::PipePipe, "||", start)
                } else {
                    ScalarToken::new(ScalarTokenType::Unknown, "|", start)
                }
            }
            '!' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    ScalarToken::new(ScalarTokenType::Ne, "!=", start)
                } else {
                    ScalarToken::new(ScalarTokenType::Bang, "!", start)
                }
            }
            '=' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    ScalarToken::new(ScalarTokenType::Eq, "==", start)
                } else {
                    ScalarToken::new(ScalarTokenType::Eq, "=", start)
                }
            }
            '<' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    ScalarToken::new(ScalarTokenType::Le, "<=", start)
                } else if self.current_char() == Some('>') {
                    self.advance();
                    ScalarToken::new(ScalarTokenType::Ne, "<>", start)
                } else {
                    ScalarToken::new(ScalarTokenType::Lt, "<", start)
                }
            }
            '>' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    ScalarToken::new(ScalarTokenType::Ge, ">=", start)
                } else {
                    ScalarToken::new(ScalarTokenType::Gt, ">", start)
                }
            }
            _ => {
                let ch = current;
                self.advance();
                ScalarToken::new(ScalarTokenType::Unknown, ch.to_string(), start)
            }
        }
    }

    fn read_string(&mut self, start: usize) -> ScalarToken {
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
        ScalarToken::new(ScalarTokenType::String, value, start)
    }

    fn read_number(&mut self, start: usize) -> ScalarToken {
        let mut value = String::new();
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
        ScalarToken::new(ScalarTokenType::Number, value, start)
    }

    fn read_identifier_or_keyword(&mut self, start: usize) -> ScalarToken {
        let mut value = String::new();
        while self
            .current_char()
            .map(|ch| ch.is_alphanumeric() || ch == '_')
            .unwrap_or(false)
        {
            value.push(self.current_char().unwrap_or_default());
            self.advance();
        }
        // 点分隔路径（如 math.sqrt）/ Dot-separated paths (e.g., math.sqrt)
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
            "and" => ScalarTokenType::And,
            "or" => ScalarTokenType::Or,
            "not" => ScalarTokenType::Not,
            "true" => ScalarTokenType::True,
            "false" => ScalarTokenType::False,
            "null" => ScalarTokenType::Null,
            "if" => ScalarTokenType::If,
            "then" => ScalarTokenType::Then,
            "else" => ScalarTokenType::Else,
            "fi" => ScalarTokenType::Fi,
            _ => ScalarTokenType::Identifier,
        };
        ScalarToken::new(token_type, value, start)
    }
}

// ============================================================================
// ScalarParser - 标量表达式语法分析器
// ============================================================================

/// 标量表达式语法分析器。
/// Scalar expression parser.
struct ScalarParser {
    tokens: Vec<ScalarToken>,
    position: usize,
}

impl ScalarParser {
    /// 创建标量表达式语法分析器。
    /// Create a scalar expression parser.
    pub fn new(tokens: Vec<ScalarToken>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// 解析标量表达式。
    /// Parse scalar expression.
    pub fn parse(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        if self.tokens.is_empty()
            || (self.tokens.len() == 1 && self.current_token().token_type == ScalarTokenType::Eof)
        {
            return Err(ExpressionParseError::new("empty expression", 0));
        }

        let expression = self.parse_ternary()?;
        if self.current_token().token_type != ScalarTokenType::Eof {
            if self.current_token().token_type == ScalarTokenType::Unknown {
                return Err(ExpressionParseError::new(
                    format!("unexpected character: {}", self.current_token().value),
                    self.current_token().position,
                ));
            }
            return Err(ExpressionParseError::new(
                format!("unexpected token: {}", self.current_token().value),
                self.current_token().position,
            ));
        }
        Ok(expression)
    }

    // ========== 优先级 9: 三元条件 / Priority 9: Ternary ==========

    fn parse_ternary(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        // if/then/else/fi 形式 / if/then/else/fi form
        if self.current_token().token_type == ScalarTokenType::If {
            return self.parse_if_then_else();
        }

        // 普通表达式（可能后接 ? : 三元）/ Normal expression (possibly followed by ? : ternary)
        let expression = self.parse_logical_or()?;

        if self.current_token().token_type == ScalarTokenType::Question {
            let question_pos = self.current_token().position;
            self.advance();

            let then_branch = self.parse_ternary()?;
            self.expect(ScalarTokenType::Colon, "expected ':' in ternary expression")?;
            let else_branch = self.parse_ternary()?;

            let condition = self.extract_boolean_condition(&expression).ok_or_else(|| {
                ExpressionParseError::new(
                    "ternary condition must be a boolean expression",
                    question_pos,
                )
            })?;

            return Ok(ScalarExpression::Conditional {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            });
        }

        Ok(expression)
    }

    fn parse_if_then_else(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        let if_pos = self.current_token().position;
        self.advance(); // skip if

        // 条件解析到逻辑或层（允许 && 和 ||）
        // Condition parsed at logical OR level (allows && and ||)
        let condition_expr = self.parse_logical_or()?;
        let condition = self
            .extract_boolean_condition(&condition_expr)
            .ok_or_else(|| {
                ExpressionParseError::new("if condition must be a boolean expression", if_pos)
            })?;

        self.expect(ScalarTokenType::Then, "expected 'then' after if condition")?;
        let then_branch = self.parse_ternary()?;
        self.expect(ScalarTokenType::Else, "expected 'else' in if expression")?;
        let else_branch = self.parse_ternary()?;
        self.expect(ScalarTokenType::Fi, "expected 'fi' to close if expression")?;

        Ok(ScalarExpression::Conditional {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        })
    }

    // ========== 优先级 8: 逻辑或 / Priority 8: Logical OR ==========

    fn parse_logical_or(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        let mut left = self.parse_logical_and()?;

        while self.current_token().token_type == ScalarTokenType::Or
            || self.current_token().token_type == ScalarTokenType::PipePipe
        {
            self.advance();
            let right = self.parse_logical_and()?;
            let left_bool = self.unwrap_boolean(&left).ok_or_else(|| {
                ExpressionParseError::new(
                    "expected boolean expression before 'or'",
                    self.current_token().position,
                )
            })?;
            let right_bool = self.unwrap_boolean(&right).ok_or_else(|| {
                ExpressionParseError::new(
                    "expected boolean expression after 'or'",
                    self.current_token().position,
                )
            })?;
            left = ScalarExpression::Boolean(Box::new(merge_or(left_bool, right_bool)));
        }

        Ok(left)
    }

    // ========== 优先级 7: 逻辑与 / Priority 7: Logical AND ==========

    fn parse_logical_and(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        let mut left = self.parse_comparison()?;

        while self.current_token().token_type == ScalarTokenType::And
            || self.current_token().token_type == ScalarTokenType::AmpersandAmpersand
        {
            self.advance();
            let right = self.parse_comparison()?;
            let left_bool = self.unwrap_boolean(&left).ok_or_else(|| {
                ExpressionParseError::new(
                    "expected boolean expression before 'and'",
                    self.current_token().position,
                )
            })?;
            let right_bool = self.unwrap_boolean(&right).ok_or_else(|| {
                ExpressionParseError::new(
                    "expected boolean expression after 'and'",
                    self.current_token().position,
                )
            })?;
            left = ScalarExpression::Boolean(Box::new(merge_and(left_bool, right_bool)));
        }

        Ok(left)
    }

    // ========== 优先级 6: 比较 / Priority 6: Comparison ==========

    fn parse_comparison(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        let mut left = self.parse_additive()?;

        if self.current_token().token_type.is_comparison_operator() {
            let operator = self
                .current_token()
                .token_type
                .comparison_operator()
                .unwrap();
            self.advance();
            let right = self.parse_additive()?;
            left = ScalarExpression::Boolean(Box::new(BooleanExpression::Comparison {
                operator,
                left,
                right,
            }));
        }

        Ok(left)
    }

    // ========== 优先级 5: 加减 / Priority 5: Additive ==========

    fn parse_additive(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        let mut left = self.parse_unary_minus()?;

        loop {
            if self.current_token().token_type == ScalarTokenType::Plus {
                self.advance();
                let right = self.parse_unary_for_operand()?;
                left = ScalarExpression::binary(BinaryOperator::Add, left, right);
            } else if self.current_token().token_type == ScalarTokenType::Minus {
                self.advance();
                let right = self.parse_unary_for_operand()?;
                left = ScalarExpression::binary(BinaryOperator::Subtract, left, right);
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// 解析一元负号。
    /// Parse unary minus.
    ///
    /// 在加减层处理一元负号，使 `-x^2` 解析为 `-(x^2)`（而非 `(-x)^2`）。
    /// Handles unary minus at additive level so `-x^2` parses as `-(x^2)` (not `(-x)^2`).
    ///
    /// 递归目标是 parse_multiplicative 而非 parse_additive，避免把后续 +/- 吞入操作数。
    /// Recursion target is parse_multiplicative, not parse_additive, to avoid
    /// swallowing subsequent +/- into the operand.
    fn parse_unary_minus(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        if self.current_token().token_type == ScalarTokenType::Minus {
            self.advance();
            let operand = self.parse_unary_minus()?;
            return Ok(ScalarExpression::unary(UnaryOperator::Negate, operand));
        }
        if self.current_token().token_type == ScalarTokenType::Plus {
            self.advance();
            let operand = self.parse_unary_minus()?;
            return Ok(ScalarExpression::unary(UnaryOperator::Positive, operand));
        }
        self.parse_multiplicative()
    }

    /// 解析操作数前缀一元操作符（用于二元操作的右操作数）。
    /// Parse prefix unary operators for operands (used as right-hand side of binary operations).
    ///
    /// 与 `parse_unary_minus` 不同，此方法的递归目标是 `parse_multiplicative`，
    /// 但不会被加减操作符误匹配（因为由各二元操作层显式调用）。
    /// Unlike `parse_unary_minus`, this method's recursion target is `parse_multiplicative`,
    /// but it won't be confused by add/subtract operators (called explicitly by each binary operation level).
    fn parse_unary_for_operand(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        if self.current_token().token_type == ScalarTokenType::Minus {
            self.advance();
            let operand = self.parse_unary_for_operand()?;
            return Ok(ScalarExpression::unary(UnaryOperator::Negate, operand));
        }
        if self.current_token().token_type == ScalarTokenType::Plus {
            self.advance();
            let operand = self.parse_unary_for_operand()?;
            return Ok(ScalarExpression::unary(UnaryOperator::Positive, operand));
        }
        self.parse_multiplicative()
    }

    // ========== 优先级 4: 乘除模 / Priority 4: Multiplicative ==========

    fn parse_multiplicative(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        let mut left = self.parse_power()?;

        loop {
            let operator = match self.current_token().token_type {
                ScalarTokenType::Star => BinaryOperator::Multiply,
                ScalarTokenType::Slash => BinaryOperator::Divide,
                ScalarTokenType::Percent => BinaryOperator::Modulo,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary_for_operand()?;
            left = ScalarExpression::binary(operator, left, right);
        }

        Ok(left)
    }

    // ========== 优先级 2: 幂 / Priority 2: Power ==========

    fn parse_power(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        let base = self.parse_unary_plus()?;

        if self.current_token().token_type == ScalarTokenType::Caret
            || self.current_token().token_type == ScalarTokenType::DoubleStar
        {
            self.advance();
            // 右结合：右操作数允许一元负号（如 x^-2），递归调用自身
            // Right-associative: right operand allows unary minus (e.g., x^-2), recursive call
            let exponent = self.parse_unary_for_operand()?;
            return Ok(ScalarExpression::binary(
                BinaryOperator::Power,
                base,
                exponent,
            ));
        }

        Ok(base)
    }

    // ========== 优先级 3: 一元正号 / Priority 3: Unary plus ==========

    fn parse_unary_plus(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        if self.current_token().token_type == ScalarTokenType::Plus {
            self.advance();
            let operand = self.parse_unary_plus()?;
            return Ok(ScalarExpression::unary(UnaryOperator::Positive, operand));
        }
        self.parse_primary()
    }

    // ========== 优先级 1: 原子 / Priority 1: Primary ==========

    fn parse_primary(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
        match self.current_token().token_type {
            ScalarTokenType::Number => {
                let token = self.current_token();
                let value = token.value.parse::<f64>().map_err(|_| {
                    ExpressionParseError::new(
                        format!("invalid number: {}", token.value),
                        token.position,
                    )
                })?;
                self.advance();
                Ok(ScalarExpression::constant(ExpressionValue::Number(value)))
            }

            ScalarTokenType::String => {
                let value = self.current_token().value.clone();
                self.advance();
                Ok(ScalarExpression::constant(ExpressionValue::String(value)))
            }

            ScalarTokenType::Identifier => self.parse_identifier_or_function(),

            ScalarTokenType::LParen => {
                self.advance();
                let expression = self.parse_ternary()?;
                self.expect(ScalarTokenType::RParen, "expected ')'")?;
                Ok(expression)
            }

            ScalarTokenType::True => {
                self.advance();
                Ok(ScalarExpression::Boolean(Box::new(
                    BooleanExpression::true_constant(),
                )))
            }

            ScalarTokenType::False => {
                self.advance();
                Ok(ScalarExpression::Boolean(Box::new(
                    BooleanExpression::false_constant(),
                )))
            }

            ScalarTokenType::Null => {
                self.advance();
                Ok(ScalarExpression::constant(ExpressionValue::Null))
            }

            ScalarTokenType::Not | ScalarTokenType::Bang => {
                // 逻辑非作为前缀 / Logical NOT as prefix
                self.advance();
                let operand = self.parse_comparison()?;
                let bool_expr = self.extract_boolean_condition(&operand).ok_or_else(|| {
                    ExpressionParseError::new(
                        "! operator requires boolean operand",
                        self.current_token().position,
                    )
                })?;
                Ok(ScalarExpression::Boolean(Box::new(
                    BooleanExpression::not_expr(bool_expr),
                )))
            }

            ScalarTokenType::Unknown => Err(ExpressionParseError::new(
                format!("unexpected character: {}", self.current_token().value),
                self.current_token().position,
            )),

            _ => Err(ExpressionParseError::new(
                format!("unexpected token: {}", self.current_token().value),
                self.current_token().position,
            )),
        }
    }

    /// 解析标识符或函数调用。
    /// Parse identifier or function call.
    fn parse_identifier_or_function(
        &mut self,
    ) -> Result<ParsedScalarExpression, ExpressionParseError> {
        let identifier = self.current_token().value.clone();
        self.advance();

        // 检查 math.PI 和 math.E 常量 / Check math.PI and math.E constants
        if identifier == "math.PI" {
            return Ok(ScalarExpression::constant(ExpressionValue::Number(
                std::f64::consts::PI,
            )));
        }
        if identifier == "math.E" {
            return Ok(ScalarExpression::constant(ExpressionValue::Number(
                std::f64::consts::E,
            )));
        }

        // 检查函数调用 / Check function call
        if self.current_token().token_type == ScalarTokenType::LParen {
            self.advance(); // skip (

            // 函数名归一化：剥离 math. 前缀 / Function name normalization: strip math. prefix
            let function_name = if let Some(stripped) = identifier.strip_prefix("math.") {
                stripped.to_string()
            } else {
                identifier
            };

            let mut arguments = Vec::new();
            if self.current_token().token_type != ScalarTokenType::RParen {
                arguments.push(self.parse_ternary()?);
                while self.current_token().token_type == ScalarTokenType::Comma {
                    self.advance();
                    arguments.push(self.parse_ternary()?);
                }
            }
            self.expect(
                ScalarTokenType::RParen,
                "expected ')' after function arguments",
            )?;

            return Ok(ScalarExpression::function(function_name, arguments));
        }

        // 普通引用 / Simple reference
        Ok(ScalarExpression::reference(PropertyPath::parse(
            &identifier,
        )))
    }

    // ========== 辅助方法 / Helper Methods ==========

    fn current_token(&self) -> ScalarToken {
        self.tokens
            .get(self.position)
            .cloned()
            .unwrap_or_else(|| ScalarToken::eof(self.position))
    }

    fn advance(&mut self) -> ScalarToken {
        let token = self.current_token();
        self.position += 1;
        token
    }

    fn expect(
        &mut self,
        token_type: ScalarTokenType,
        message: &'static str,
    ) -> Result<ScalarToken, ExpressionParseError> {
        if self.current_token().token_type != token_type {
            return Err(ExpressionParseError::new(
                message,
                self.current_token().position,
            ));
        }
        Ok(self.advance())
    }

    /// 从 ScalarExpression 中提取 BooleanExpression（如果是 ScalarBoolean 包装）。
    /// Extract BooleanExpression from ScalarExpression (if it's a ScalarBoolean wrapper).
    fn extract_boolean_condition(
        &self,
        expr: &ParsedScalarExpression,
    ) -> Option<ParsedBooleanExpression> {
        match expr {
            ScalarExpression::Boolean(inner) => Some((**inner).clone()),
            _ => None,
        }
    }

    /// 从 ScalarExpression 中解包 BooleanExpression（用于逻辑操作符和条件）。
    /// Unwrap BooleanExpression from ScalarExpression (for logical operators and conditionals).
    fn unwrap_boolean(&self, expr: &ParsedScalarExpression) -> Option<ParsedBooleanExpression> {
        match expr {
            ScalarExpression::Boolean(inner) => Some((**inner).clone()),
            _ => None,
        }
    }
}

// ========== 合并辅助函数 / Merge Helper Functions ==========

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

// ========== 公共入口函数 / Public Entry Points ==========

/// 解析标量表达式字符串。
/// Parse scalar expression string.
///
/// # 示例 / Examples
///
/// ```ignore
/// use ospf_rust_math::symbol::expression::parse_scalar_expression;
///
/// let expr = parse_scalar_expression("x + y * 2").unwrap();
/// ```
pub fn parse_scalar_expression(
    input: &str,
) -> Result<ParsedScalarExpression, ExpressionParseError> {
    let tokens = ScalarLexer::new(input).tokenize();
    ScalarParser::new(tokens).parse()
}

/// 尝试解析标量表达式字符串，失败时返回 None。
/// Try to parse scalar expression string, returning None on failure.
pub fn parse_scalar_expression_or_none(input: &str) -> Option<ParsedScalarExpression> {
    parse_scalar_expression(input).ok()
}

#[cfg(test)]
mod tests {
    use super::super::{MapEvaluationContext, MathFunctionEvaluator, evaluate_scalar_expression};
    use super::*;

    fn context() -> MapEvaluationContext {
        MapEvaluationContext::from_string_map([
            ("x", ExpressionValue::Number(3.0)),
            ("y", ExpressionValue::Number(4.0)),
            ("a", ExpressionValue::Number(3.0)),
            ("b", ExpressionValue::Number(4.0)),
            ("w", ExpressionValue::Number(800.0)),
        ])
    }

    fn eval(expr: &ParsedScalarExpression, ctx: &MapEvaluationContext) -> Option<ExpressionValue> {
        evaluate_scalar_expression(expr, ctx, &MathFunctionEvaluator)
    }

    // ========== 算术测试 / Arithmetic Tests ==========

    #[test]
    fn test_addition() {
        let expr = parse_scalar_expression("x + y").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(7.0)));
    }

    #[test]
    fn test_multiplication() {
        let expr = parse_scalar_expression("x * y").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(12.0)));
    }

    #[test]
    fn test_division() {
        let expr = parse_scalar_expression("x / y").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(0.75)));
    }

    #[test]
    fn test_modulo() {
        let expr = parse_scalar_expression("x % y").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(3.0)));
    }

    #[test]
    fn test_power_caret() {
        let expr = parse_scalar_expression("x ^ 2").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(9.0)));
    }

    #[test]
    fn test_power_double_star() {
        let expr = parse_scalar_expression("x ** 2").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(9.0)));
    }

    #[test]
    fn test_unary_negate() {
        let expr = parse_scalar_expression("-x").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(-3.0)));
    }

    #[test]
    fn test_unary_minus_precedence() {
        // -x^2 = -(x^2) = -9
        let expr = parse_scalar_expression("-x ^ 2").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(-9.0)));
    }

    #[test]
    fn test_unary_minus_with_trailing_add() {
        // -x^2+1 = -(x^2)+1 = -8
        let expr = parse_scalar_expression("-x ^ 2 + 1").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(-8.0)));
    }

    #[test]
    fn test_unary_minus_with_trailing_subtract() {
        // -2-3 = (-2)-3 = -5
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("-2 - 3").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Number(-5.0)));
    }

    #[test]
    fn test_consecutive_unary_minus() {
        // --x = -(-x) = x = 3
        let expr = parse_scalar_expression("--x").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(3.0)));
    }

    #[test]
    fn test_arithmetic_precedence() {
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("2 + 3 * 4").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Number(14.0)));
    }

    #[test]
    fn test_parentheses() {
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("(2 + 3) * 4").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Number(20.0)));
    }

    // ========== 数学函数测试 / Math Function Tests ==========

    #[test]
    fn test_math_sqrt() {
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("math.sqrt(16)").unwrap();
        assert!(matches!(expr, ScalarExpression::Function { .. }));
        if let ScalarExpression::Function { name, .. } = &expr {
            assert_eq!(name, "sqrt");
        }
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Number(4.0)));
    }

    #[test]
    fn test_math_pow_sum() {
        let expr = parse_scalar_expression("math.pow(x, 2) + math.pow(y, 2)").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(25.0)));
    }

    #[test]
    fn test_math_pi() {
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("math.PI").unwrap();
        assert!(matches!(
            expr,
            ScalarExpression::Constant(ExpressionValue::Number(_))
        ));
    }

    #[test]
    fn test_math_e() {
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("math.E").unwrap();
        assert!(matches!(
            expr,
            ScalarExpression::Constant(ExpressionValue::Number(_))
        ));
    }

    // ========== 条件测试 / Conditional Tests ==========

    #[test]
    fn test_ternary() {
        let expr = parse_scalar_expression("x > 0 ? x : y").unwrap();
        assert!(matches!(expr, ScalarExpression::Conditional { .. }));
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(3.0)));
    }

    #[test]
    fn test_if_then_else() {
        let expr = parse_scalar_expression("if (w > 787) then x else y fi").unwrap();
        assert!(matches!(expr, ScalarExpression::Conditional { .. }));
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(3.0)));
    }

    #[test]
    fn test_if_with_and_condition() {
        let expr = parse_scalar_expression("if x > 0 && y > 0 then x else y fi").unwrap();
        assert!(matches!(expr, ScalarExpression::Conditional { .. }));
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(3.0)));
    }

    #[test]
    fn test_if_with_or_condition() {
        let ctx = MapEvaluationContext::from_string_map([
            ("a", ExpressionValue::Number(-1.0)),
            ("b", ExpressionValue::Number(5.0)),
            ("r", ExpressionValue::Number(10.0)),
        ]);
        let expr = parse_scalar_expression("if a > 0 || b > 0 then r else 0 fi").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Number(10.0)));
    }

    // ========== 布尔标量测试 / Boolean-as-Scalar Tests ==========

    #[test]
    fn test_boolean_and() {
        let expr = parse_scalar_expression("x > 0 && y > 0").unwrap();
        assert!(matches!(expr, ScalarExpression::Boolean(_)));
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Boolean(true)));
    }

    #[test]
    fn test_true_constant() {
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("true").unwrap();
        assert!(matches!(expr, ScalarExpression::Boolean(_)));
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Boolean(true)));
    }

    #[test]
    fn test_false_constant() {
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("false").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Boolean(false)));
    }

    #[test]
    fn test_comparison_as_scalar() {
        let expr = parse_scalar_expression("x > 0").unwrap();
        assert!(matches!(expr, ScalarExpression::Boolean(_)));
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Boolean(true)));
    }

    // ========== 错误用例测试 / Error Tests ==========

    #[test]
    fn test_division_by_zero() {
        let ctx = MapEvaluationContext::from_string_map([
            ("a", ExpressionValue::Number(1.0)),
            ("b", ExpressionValue::Number(0.0)),
        ]);
        let expr = parse_scalar_expression("a / b").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, None);
    }

    #[test]
    fn test_unknown_function() {
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("unknownFunc(1)").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, None);
    }

    #[test]
    fn test_mismatched_if() {
        let result = parse_scalar_expression("if x > 0 then x fi");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_expression() {
        let result = parse_scalar_expression("");
        assert!(result.is_err());
    }

    // ========== AST 结构测试 / AST Structure Tests ==========

    #[test]
    fn test_function_namespace_stripping() {
        let expr = parse_scalar_expression("math.sqrt(16)").unwrap();
        if let ScalarExpression::Function { name, .. } = &expr {
            assert_eq!(name, "sqrt");
        } else {
            panic!("expected Function expression");
        }
    }

    #[test]
    fn test_unary_minus_power_ast() {
        // -x^2 should be Negate(Power(x, 2))
        let expr = parse_scalar_expression("-x ^ 2").unwrap();
        assert!(matches!(
            expr,
            ScalarExpression::Unary {
                operator: UnaryOperator::Negate,
                operand: _,
            }
        ));
        if let ScalarExpression::Unary { operand, .. } = &expr {
            assert!(matches!(
                operand.as_ref(),
                ScalarExpression::Binary {
                    operator: BinaryOperator::Power,
                    ..
                }
            ));
        }
    }

    #[test]
    fn test_conditional_ast() {
        let expr = parse_scalar_expression("if x > 0 then x else y fi").unwrap();
        assert!(matches!(expr, ScalarExpression::Conditional { .. }));
        if let ScalarExpression::Conditional { condition, .. } = &expr {
            assert!(matches!(
                condition.as_ref(),
                BooleanExpression::Comparison { .. }
            ));
        }
    }

    // ========== 求值器测试 / Evaluator Tests ==========

    #[test]
    fn test_logical_not() {
        let expr = parse_scalar_expression("!(x > 0)").unwrap();
        assert!(matches!(expr, ScalarExpression::Boolean(_)));
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Boolean(false)));
    }

    #[test]
    fn test_nested_ternary() {
        let expr = parse_scalar_expression("x > 0 ? (y > 0 ? 1 : 2) : 3").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(1.0)));
    }

    #[test]
    fn test_ternary_false_branch() {
        let expr = parse_scalar_expression("x > 10 ? 10 : 20").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(20.0)));
    }

    #[test]
    fn test_boolean_in_arithmetic_fails() {
        // (x > 0) + 1 — boolean not coerced to number
        let expr = parse_scalar_expression("(x > 0) + 1").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, None);
    }

    // ========== 边界测试 / Boundary Tests ==========

    #[test]
    fn test_unknown_character_rejected() {
        // 非法字符应报错，而非静默截断
        // Unknown characters should error, not silently truncate
        let result = parse_scalar_expression("x $ y");
        assert!(result.is_err(), "expected error for unknown character '$'");

        let result = parse_scalar_expression("x # 3");
        assert!(result.is_err(), "expected error for unknown character '#'");
    }

    #[test]
    fn test_single_ampersand_rejected() {
        // 单个 & 应报错
        // Single & should error
        let result = parse_scalar_expression("x > 0 & y > 0");
        assert!(result.is_err(), "expected error for single '&'");
    }

    #[test]
    fn test_single_pipe_rejected() {
        // 单个 | 应报错
        // Single | should error
        let result = parse_scalar_expression("x > 0 | y > 0");
        assert!(result.is_err(), "expected error for single '|'");
    }

    #[test]
    fn test_math_function_in_boolean_condition() {
        // math.sqrt(x) > 2 应通过 MathFunctionEvaluator 求值
        // math.sqrt(x) > 2 should evaluate via MathFunctionEvaluator
        let ctx = MapEvaluationContext::from_string_map([("x", ExpressionValue::Number(16.0))]);
        let expr = parse_scalar_expression("math.sqrt(x) > 2").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Boolean(true)));
    }

    #[test]
    fn test_math_function_in_conditional() {
        // if math.sqrt(x) > 2 then x else 0 fi
        let ctx = MapEvaluationContext::from_string_map([("x", ExpressionValue::Number(16.0))]);
        let expr = parse_scalar_expression("if math.sqrt(x) > 2 then x else 0 fi").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Number(16.0)));
    }

    #[test]
    fn test_diamond_operator_not_equal() {
        // <> 不等号
        // <> not-equal operator
        let expr = parse_scalar_expression("x <> y").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Boolean(true)));

        let expr = parse_scalar_expression("x <> x").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Boolean(false)));
    }

    #[test]
    fn test_addition_with_negative_right() {
        // 2 + -3 = -1
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("2 + -3").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Number(-1.0)));
    }

    #[test]
    fn test_multiplication_with_negative_right() {
        // 2 * -3 = -6
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("2 * -3").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Number(-6.0)));
    }

    #[test]
    fn test_power_with_negative_exponent() {
        // 2 ^ -1 = 0.5
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let expr = parse_scalar_expression("2 ^ -1").unwrap();
        let result = eval(&expr, &ctx);
        assert_eq!(result, Some(ExpressionValue::Number(0.5)));
    }

    #[test]
    fn test_variable_with_negative_exponent() {
        // x ^ -1 = 1/3
        let expr = parse_scalar_expression("x ^ -1").unwrap();
        let result = eval(&expr, &context());
        assert_eq!(result, Some(ExpressionValue::Number(1.0 / 3.0)));
    }

    #[test]
    fn test_two_arg_function_rejects_extra_args() {
        // math.pow(2, 3, 4) 应返回 None（参数过多）
        // math.pow(2, 3, 4) should return None (too many arguments)
        let ctx = MapEvaluationContext::from_string_map::<_, &str>([]);
        let result = MathFunctionEvaluator.evaluate(
            "pow",
            &[
                Some(ExpressionValue::Number(2.0)),
                Some(ExpressionValue::Number(3.0)),
                Some(ExpressionValue::Number(4.0)),
            ],
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_single_arg_function_rejects_extra_args() {
        // math.sqrt(4, 2) 应返回 None（参数过多）
        // math.sqrt(4, 2) should return None (too many arguments)
        let result = MathFunctionEvaluator.evaluate(
            "sqrt",
            &[
                Some(ExpressionValue::Number(4.0)),
                Some(ExpressionValue::Number(2.0)),
            ],
        );
        assert_eq!(result, None);
    }
}
