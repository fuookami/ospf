//! 语法分析器 / Parser
//!
//! 将 token 序列转换为表达式树。
//! Converts token sequence to expression tree.

use num_traits::{One, Zero};
use std::any::Any;
use std::fmt::{Debug, Display, Formatter, Result};
use std::ops::{Add, Mul, Neg, Sub};

use super::error::{ParseError, ParseResult};
use super::expr::{Expr, ExprKind};
use super::lexer::{Lexer, Token, TokenKind};
use crate::operator::Exponent;
use crate::symbol::inequality::{Comparison, LinearInequality, QuadraticInequality};
use crate::symbol::{
    Canonical, CanonicalMonomial, DynSymbol, Linear, LinearMonomial, OwnedSymbol, Quadratic,
    QuadraticMonomial, SymbolDynId,
};
use std::collections::HashMap;

// ============================================================================
// ParserSymbol - 解析器内部使用的符号类型
// ============================================================================

/// 解析器内部使用的符号类型 / Symbol type used internally by parser
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserSymbol {
    name: String,
}

impl ParserSymbol {
    /// 创建新的解析器符号
    /// Create a new parser symbol
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Display for ParserSymbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.name)
    }
}

impl DynSymbol for ParserSymbol {
    fn name(&self) -> &str {
        &self.name
    }

    fn display_name(&self) -> &str {
        &self.name
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        // 使用名称的哈希作为 ID（简单实现）
        // Use hash of name as ID (simple implementation)
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        self.name.hash(&mut hasher);
        SymbolDynId::standalone(hasher.finish() as usize)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Parser - 语法分析器
// ============================================================================

/// 语法分析器 / Parser
pub struct Parser<'a> {
    /// 词法分析器 / Lexer
    lexer: Lexer<'a>,
    /// 当前 Token / Current token
    current: Token,
    /// 下一个 Token / Next token
    peek: Token,
}

impl<'a> Parser<'a> {
    /// 创建新的语法分析器 / Create a new parser
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let current = lexer
            .next_token()
            .unwrap_or_else(|_| Token::new(TokenKind::Eof, 0, 0));
        let peek = lexer
            .next_token()
            .unwrap_or_else(|_| Token::new(TokenKind::Eof, 0, 0));
        Self {
            lexer,
            current,
            peek,
        }
    }

    /// 前进到下一个 Token / Advance to next token
    fn advance(&mut self) -> ParseResult<Token> {
        let old = std::mem::replace(&mut self.current, self.peek.clone());
        self.peek = self.lexer.next_token()?;
        Ok(old)
    }

    /// 检查当前 Token 类型 / Check current token kind
    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current.kind) == std::mem::discriminant(kind)
    }

    /// 期望特定类型的 Token / Expect specific token kind
    fn expect(&mut self, kind: TokenKind) -> ParseResult<Token> {
        if self.check(&kind) {
            self.advance()
        } else {
            Err(ParseError::new(
                format!("Expected {:?}, found {:?}", kind, self.current.kind),
                self.current.start,
            ))
        }
    }

    /// 解析线性多项式 / Parse linear polynomial
    pub fn parse_linear<T>(&mut self) -> ParseResult<Linear<T>>
    where
        T: std::str::FromStr
            + Clone
            + Zero
            + One
            + Neg<Output = T>
            + Add<Output = T>
            + Sub<Output = T>
            + Debug
            + PartialEq,
    {
        let expr = self.parse_expr::<T>()?;
        expr_to_linear(&expr)
    }

    /// 解析二次多项式 / Parse quadratic polynomial
    pub fn parse_quadratic<T>(&mut self) -> ParseResult<Quadratic<T>>
    where
        T: std::str::FromStr
            + Clone
            + Zero
            + One
            + Neg<Output = T>
            + Add<Output = T>
            + Sub<Output = T>
            + Mul<Output = T>
            + Debug
            + PartialEq,
    {
        let expr = self.parse_expr::<T>()?;
        expr_to_quadratic(&expr)
    }

    /// 解析线性不等式 / Parse linear inequality
    pub fn parse_linear_inequality<T>(&mut self) -> ParseResult<LinearInequality<T>>
    where
        T: std::str::FromStr
            + Clone
            + Zero
            + One
            + Neg<Output = T>
            + Add<Output = T>
            + Sub<Output = T>
            + Debug
            + PartialEq,
    {
        let expr = self.parse_inequality::<T>()?;
        expr_to_linear_inequality(&expr)
    }

    /// 解析二次不等式 / Parse quadratic inequality
    pub fn parse_quadratic_inequality<T>(&mut self) -> ParseResult<QuadraticInequality<T>>
    where
        T: std::str::FromStr
            + Clone
            + Zero
            + One
            + Neg<Output = T>
            + Add<Output = T>
            + Sub<Output = T>
            + Mul<Output = T>
            + Debug
            + PartialEq,
    {
        let expr = self.parse_inequality::<T>()?;
        expr_to_quadratic_inequality(&expr)
    }

    /// 解析标准多项式 / Parse canonical polynomial
    ///
    /// 支持格式：`x^2 * y^3 + 2 * x + 1`, `x * x * y`, `x^3`
    /// Supported formats: `x^2 * y^3 + 2 * x + 1`, `x * x * y`, `x^3`
    ///
    /// # 示例 / Examples
    ///
    /// ```ignore
    /// use ospf_rust_math::symbol::parser::parse_canonical;
    ///
    /// let canonical = parse_canonical::<f64, i32>("x^2 * y + 2 * x + 1")?;
    /// ```
    pub fn parse_canonical<T, E>(&mut self) -> ParseResult<Canonical<T, E>>
    where
        T: std::str::FromStr
            + Clone
            + Zero
            + One
            + Neg<Output = T>
            + Add<Output = T>
            + Sub<Output = T>
            + Mul<Output = T>
            + Debug
            + PartialEq,
        E: Exponent
            + std::str::FromStr
            + Clone
            + Zero
            + One
            + std::ops::Add<Output = E>
            + PartialEq,
    {
        let expr = self.parse_expr::<T>()?;
        expr_to_canonical(&expr)
    }

    /// 解析表达式 / Parse expression
    fn parse_expr<T>(&mut self) -> ParseResult<Expr<T>>
    where
        T: std::str::FromStr + Clone,
    {
        self.parse_additive()
    }

    /// 解析不等式 / Parse inequality
    fn parse_inequality<T>(&mut self) -> ParseResult<Expr<T>>
    where
        T: std::str::FromStr + Clone,
    {
        let lhs = self.parse_expr::<T>()?;

        let comparison = if self.check(&TokenKind::LessEqual) {
            Comparison::LessEqual
        } else if self.check(&TokenKind::GreaterEqual) {
            Comparison::GreaterEqual
        } else if self.check(&TokenKind::Less) {
            Comparison::Less
        } else if self.check(&TokenKind::Greater) {
            Comparison::Greater
        } else if self.check(&TokenKind::Equal) {
            Comparison::Equal
        } else {
            return Err(ParseError::new(
                "Expected comparison operator",
                self.current.start,
            ));
        };

        self.advance()?;
        let rhs = self.parse_primary::<T>()?;

        // 提取右侧常数 / Extract right-hand side constant
        let rhs_value = match rhs.kind {
            ExprKind::Constant(v) => v,
            _ => {
                return Err(ParseError::new(
                    "Right-hand side of inequality must be a constant",
                    rhs.start,
                ));
            }
        };

        let start = lhs.start;
        let end = rhs.end;

        Ok(Expr::new(
            ExprKind::LinearInequality {
                lhs: Box::new(lhs),
                comparison,
                rhs: rhs_value,
            },
            start,
            end,
        ))
    }

    /// 解析加减法表达式 / Parse additive expression
    fn parse_additive<T>(&mut self) -> ParseResult<Expr<T>>
    where
        T: std::str::FromStr + Clone,
    {
        let mut left = self.parse_multiplicative::<T>()?;

        loop {
            if self.check(&TokenKind::Plus) {
                self.advance()?;
                let right = self.parse_multiplicative::<T>()?;
                let start = left.start;
                let end = right.end;
                left = Expr::new(ExprKind::Add(Box::new(left), Box::new(right)), start, end);
            } else if self.check(&TokenKind::Minus) {
                self.advance()?;
                let right = self.parse_multiplicative::<T>()?;
                let start = left.start;
                let end = right.end;
                left = Expr::new(ExprKind::Sub(Box::new(left), Box::new(right)), start, end);
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// 解析乘除法表达式 / Parse multiplicative expression
    fn parse_multiplicative<T>(&mut self) -> ParseResult<Expr<T>>
    where
        T: std::str::FromStr + Clone,
    {
        let mut left = self.parse_unary::<T>()?;

        loop {
            if self.check(&TokenKind::Asterisk) {
                self.advance()?;
                let right = self.parse_unary::<T>()?;
                let start = left.start;
                let end = right.end;
                left = Expr::new(ExprKind::Mul(Box::new(left), Box::new(right)), start, end);
            } else if self.check(&TokenKind::Slash) {
                self.advance()?;
                let right = self.parse_unary::<T>()?;
                let start = left.start;
                let end = right.end;
                left = Expr::new(ExprKind::Div(Box::new(left), Box::new(right)), start, end);
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// 解析一元表达式 / Parse unary expression
    fn parse_unary<T>(&mut self) -> ParseResult<Expr<T>>
    where
        T: std::str::FromStr + Clone,
    {
        if self.check(&TokenKind::Minus) {
            let op = self.advance()?;
            let expr = self.parse_unary::<T>()?;
            let start = op.start;
            let end = expr.end;
            return Ok(Expr::new(ExprKind::Neg(Box::new(expr)), start, end));
        }

        self.parse_power()
    }

    /// 解析幂运算表达式 / Parse power expression
    fn parse_power<T>(&mut self) -> ParseResult<Expr<T>>
    where
        T: std::str::FromStr + Clone,
    {
        let base = self.parse_primary::<T>()?;

        if self.check(&TokenKind::Caret) {
            self.advance()?;
            let exp = self.parse_primary::<T>()?;
            let start = base.start;
            let end = exp.end;
            return Ok(Expr::new(
                ExprKind::Pow(Box::new(base), Box::new(exp)),
                start,
                end,
            ));
        }

        Ok(base)
    }

    /// 解析基本表达式 / Parse primary expression
    fn parse_primary<T>(&mut self) -> ParseResult<Expr<T>>
    where
        T: std::str::FromStr + Clone,
    {
        match &self.current.kind.clone() {
            TokenKind::Number(s) => {
                let value = s.parse::<T>().map_err(|_| {
                    ParseError::new(format!("Invalid number: {}", s), self.current.start)
                })?;
                let token = self.advance()?;
                Ok(Expr::new(ExprKind::Constant(value), token.start, token.end))
            }
            TokenKind::Ident(name) => {
                let token = self.advance()?;
                Ok(Expr::new(
                    ExprKind::Symbol(name.clone()),
                    token.start,
                    token.end,
                ))
            }
            TokenKind::LParen => {
                self.advance()?;
                let expr = self.parse_expr::<T>()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            _ => Err(ParseError::new(
                format!("Unexpected token: {:?}", self.current.kind),
                self.current.start,
            )),
        }
    }
}

// ============================================================================
// 表达式转换函数 / Expression conversion functions
// ============================================================================

/// 创建符号 / Create symbol
fn create_symbol(name: &str) -> OwnedSymbol {
    OwnedSymbol::new(ParserSymbol::new(name))
}

/// 将表达式转换为线性多项式 / Convert expression to linear polynomial
fn expr_to_linear<T>(expr: &Expr<T>) -> ParseResult<Linear<T>>
where
    T: Clone + Zero + One + Neg<Output = T> + Add<Output = T> + Sub<Output = T> + Debug,
{
    match &expr.kind {
        ExprKind::Constant(c) => Ok(Linear::new(vec![], c.clone())),
        ExprKind::Symbol(name) => Ok(Linear::new(
            vec![LinearMonomial::new(T::one(), create_symbol(name))],
            T::zero(),
        )),
        ExprKind::LinearMonomial {
            coefficient,
            symbol,
        } => Ok(Linear::new(
            vec![LinearMonomial::new(
                coefficient.clone(),
                create_symbol(symbol),
            )],
            T::zero(),
        )),
        ExprKind::Add(left, right) => {
            let l = expr_to_linear(left)?;
            let r = expr_to_linear(right)?;
            Ok(l + r)
        }
        ExprKind::Sub(left, right) => {
            let l = expr_to_linear(left)?;
            let r = expr_to_linear(right)?;
            Ok(l - r)
        }
        ExprKind::Neg(inner) => {
            let inner = expr_to_linear(inner)?;
            Ok(-inner)
        }
        ExprKind::Mul(left, right) => {
            // 处理 系数 * 符号 的情况
            // Handle coefficient * symbol case
            match (&left.kind, &right.kind) {
                (ExprKind::Constant(c), ExprKind::Symbol(s)) => Ok(Linear::new(
                    vec![LinearMonomial::new(c.clone(), create_symbol(s))],
                    T::zero(),
                )),
                (ExprKind::Symbol(s), ExprKind::Constant(c)) => Ok(Linear::new(
                    vec![LinearMonomial::new(c.clone(), create_symbol(s))],
                    T::zero(),
                )),
                _ => Err(ParseError::new(
                    format!("Cannot convert to linear polynomial: {:?}", expr.kind),
                    expr.start,
                )),
            }
        }
        _ => Err(ParseError::new(
            format!("Cannot convert to linear polynomial: {:?}", expr.kind),
            expr.start,
        )),
    }
}

/// 将表达式转换为二次多项式 / Convert expression to quadratic polynomial
fn expr_to_quadratic<T>(expr: &Expr<T>) -> ParseResult<Quadratic<T>>
where
    T: Clone
        + Zero
        + One
        + Neg<Output = T>
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Debug
        + PartialEq,
{
    match &expr.kind {
        ExprKind::Constant(c) => Ok(Quadratic::new(vec![], c.clone())),
        ExprKind::Symbol(name) => Ok(Quadratic::new(
            vec![QuadraticMonomial::linear(T::one(), create_symbol(name))],
            T::zero(),
        )),
        ExprKind::LinearMonomial {
            coefficient,
            symbol,
        } => Ok(Quadratic::new(
            vec![QuadraticMonomial::linear(
                coefficient.clone(),
                create_symbol(symbol),
            )],
            T::zero(),
        )),
        ExprKind::QuadraticMonomial {
            coefficient,
            symbol1,
            symbol2,
        } => {
            let mono = if let Some(s2) = symbol2 {
                QuadraticMonomial::quadratic(
                    coefficient.clone(),
                    create_symbol(symbol1),
                    create_symbol(s2),
                )
            } else {
                QuadraticMonomial::linear(coefficient.clone(), create_symbol(symbol1))
            };
            Ok(Quadratic::new(vec![mono], T::zero()))
        }
        ExprKind::Add(left, right) => {
            let l = expr_to_quadratic(left)?;
            let r = expr_to_quadratic(right)?;
            Ok(l + r)
        }
        ExprKind::Sub(left, right) => {
            let l = expr_to_quadratic(left)?;
            let r = expr_to_quadratic(right)?;
            Ok(l - r)
        }
        ExprKind::Mul(left, right) => {
            // 处理 系数 * 符号 或 符号 * 符号 的情况
            // Handle coefficient * symbol or symbol * symbol cases
            match (&left.kind, &right.kind) {
                (ExprKind::Constant(c), ExprKind::Symbol(s)) => Ok(Quadratic::new(
                    vec![QuadraticMonomial::linear(c.clone(), create_symbol(s))],
                    T::zero(),
                )),
                (ExprKind::Symbol(s), ExprKind::Constant(c)) => Ok(Quadratic::new(
                    vec![QuadraticMonomial::linear(c.clone(), create_symbol(s))],
                    T::zero(),
                )),
                (ExprKind::Symbol(s1), ExprKind::Symbol(s2)) => Ok(Quadratic::new(
                    vec![QuadraticMonomial::quadratic(
                        T::one(),
                        create_symbol(s1),
                        create_symbol(s2),
                    )],
                    T::zero(),
                )),
                _ => Err(ParseError::new(
                    "Multiplication not yet supported in parser",
                    expr.start,
                )),
            }
        }
        ExprKind::Neg(inner) => {
            let inner = expr_to_quadratic(inner)?;
            Ok(-inner)
        }
        ExprKind::Pow(base, exp) => {
            // 只支持 x^2 形式
            // Only support x^2 form
            match (&base.kind, &exp.kind) {
                (ExprKind::Symbol(name), ExprKind::Constant(n)) if *n == T::one() + T::one() => {
                    Ok(Quadratic::new(
                        vec![QuadraticMonomial::quadratic(
                            T::one(),
                            create_symbol(name),
                            create_symbol(name),
                        )],
                        T::zero(),
                    ))
                }
                _ => Err(ParseError::new(
                    "Only x^2 power is supported for quadratic",
                    expr.start,
                )),
            }
        }
        _ => Err(ParseError::new(
            format!("Cannot convert to quadratic polynomial: {:?}", expr.kind),
            expr.start,
        )),
    }
}

/// 将表达式转换为线性不等式 / Convert expression to linear inequality
fn expr_to_linear_inequality<T>(expr: &Expr<T>) -> ParseResult<LinearInequality<T>>
where
    T: Clone + Zero + One + Neg<Output = T> + Add<Output = T> + Sub<Output = T> + Debug,
{
    match &expr.kind {
        ExprKind::LinearInequality {
            lhs,
            comparison,
            rhs,
        } => {
            let lhs_linear = expr_to_linear(lhs)?;
            Ok(LinearInequality::new(lhs_linear, *comparison, rhs.clone()))
        }
        _ => Err(ParseError::new("Expected linear inequality", expr.start)),
    }
}

/// 将表达式转换为二次不等式 / Convert expression to quadratic inequality
fn expr_to_quadratic_inequality<T>(expr: &Expr<T>) -> ParseResult<QuadraticInequality<T>>
where
    T: Clone
        + Zero
        + One
        + Neg<Output = T>
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Debug
        + PartialEq,
{
    match &expr.kind {
        ExprKind::LinearInequality {
            lhs,
            comparison,
            rhs,
        } => {
            let lhs_quad = expr_to_quadratic(lhs)?;
            Ok(QuadraticInequality::new(lhs_quad, *comparison, rhs.clone()))
        }
        _ => Err(ParseError::new("Expected quadratic inequality", expr.start)),
    }
}

/// 将表达式转换为标准多项式 / Convert expression to canonical polynomial
///
/// 支持幂运算和任意次多项式。
/// Supports power operations and polynomials of any degree.
fn expr_to_canonical<T, E>(expr: &Expr<T>) -> ParseResult<Canonical<T, E>>
where
    T: Clone
        + Zero
        + One
        + Neg<Output = T>
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Debug
        + PartialEq,
    E: Exponent + std::str::FromStr + Clone + Zero + One + std::ops::Add<Output = E> + PartialEq,
{
    match &expr.kind {
        ExprKind::Constant(c) => Ok(Canonical::new(vec![], c.clone())),

        ExprKind::Symbol(name) => {
            let mut powers = HashMap::new();
            powers.insert(create_symbol(name), E::one());
            Ok(Canonical::new(
                vec![CanonicalMonomial::new(T::one(), powers)],
                T::zero(),
            ))
        }

        ExprKind::CanonicalMonomial {
            coefficient,
            powers,
        } => {
            let mut mono_powers = HashMap::new();
            for (name, &power) in powers {
                // 尝试解析幂次
                // Try to parse power
                let exp = E::from_str(name).unwrap_or_else(|_| E::one());
                mono_powers.insert(create_symbol(name), exp);
            }
            Ok(Canonical::new(
                vec![CanonicalMonomial::new(coefficient.clone(), mono_powers)],
                T::zero(),
            ))
        }

        ExprKind::Add(left, right) => {
            let l = expr_to_canonical(left)?;
            let r = expr_to_canonical(right)?;
            Ok(l + r)
        }

        ExprKind::Sub(left, right) => {
            let l = expr_to_canonical(left)?;
            let r = expr_to_canonical(right)?;
            Ok(l - r)
        }

        ExprKind::Mul(left, right) => {
            let l = expr_to_canonical(left)?;
            let r = expr_to_canonical(right)?;
            Ok(l.multiply(r))
        }

        ExprKind::Neg(inner) => {
            let inner = expr_to_canonical(inner)?;
            Ok(-inner)
        }

        ExprKind::Pow(base, exp) => {
            // 支持 x^n 形式
            // Support x^n form
            match (&base.kind, &exp.kind) {
                (ExprKind::Symbol(name), ExprKind::Constant(n)) => {
                    // 尝试将常数转换为指数类型
                    // Try to convert constant to exponent type
                    let power = exponent_from_value(n).ok_or_else(|| {
                        ParseError::new(format!("Invalid exponent: {:?}", n), exp.start)
                    })?;

                    let mut powers = HashMap::new();
                    powers.insert(create_symbol(name), power);
                    Ok(Canonical::new(
                        vec![CanonicalMonomial::new(T::one(), powers)],
                        T::zero(),
                    ))
                }
                _ => Err(ParseError::new(
                    "Power operation requires symbol base and constant exponent",
                    expr.start,
                )),
            }
        }

        _ => Err(ParseError::new(
            format!("Cannot convert to canonical polynomial: {:?}", expr.kind),
            expr.start,
        )),
    }
}

/// 从 T 值提取指数 / Extract exponent from T value
fn exponent_from_value<T, E>(value: &T) -> Option<E>
where
    T: Clone + Debug + PartialEq,
    E: Exponent + std::str::FromStr + Zero + One,
{
    // 尝试通过 Debug 格式解析
    // Try to parse through Debug format
    let s = format!("{:?}", value);
    E::from_str(&s).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_linear_simple() {
        let mut parser = Parser::new("x");
        let result: ParseResult<Linear<f64>> = parser.parse_linear();
        assert!(result.is_ok());
        let linear = result.unwrap();
        assert_eq!(linear.monomials.len(), 1);
        assert_eq!(linear.constant, 0.0);
    }

    #[test]
    fn test_parse_linear_with_coefficient() {
        let mut parser = Parser::new("2 * x + 3 * y");
        let result: ParseResult<Linear<f64>> = parser.parse_linear();
        assert!(result.is_ok());
        let linear = result.unwrap();
        assert_eq!(linear.monomials.len(), 2);
        assert_eq!(linear.constant, 0.0);
    }

    #[test]
    fn test_parse_linear_with_constant() {
        let mut parser = Parser::new("2 * x + 3 * y + 1");
        let result: ParseResult<Linear<f64>> = parser.parse_linear();
        assert!(result.is_ok());
        let linear = result.unwrap();
        assert_eq!(linear.monomials.len(), 2);
        assert_eq!(linear.constant, 1.0);
    }

    #[test]
    fn test_parse_linear_inequality() {
        let mut parser = Parser::new("2 * x + 3 * y <= 5");
        let result: ParseResult<LinearInequality<f64>> = parser.parse_linear_inequality();
        assert!(result.is_ok());
        let ineq = result.unwrap();
        assert_eq!(ineq.comparison, Comparison::LessEqual);
        assert_eq!(ineq.rhs, 5.0);
    }

    #[test]
    fn test_parse_quadratic_power() {
        let mut parser = Parser::new("x ^ 2");
        let result: ParseResult<Quadratic<f64>> = parser.parse_quadratic();
        assert!(result.is_ok());
        let quad = result.unwrap();
        assert_eq!(quad.monomials.len(), 1);
    }

    #[test]
    fn test_parse_canonical_simple() {
        let mut parser = Parser::new("x");
        let result: ParseResult<Canonical<f64, i32>> = parser.parse_canonical();
        assert!(result.is_ok());
        let canonical = result.unwrap();
        assert_eq!(canonical.monomials.len(), 1);
        assert_eq!(canonical.constant, 0.0);
    }

    #[test]
    fn test_parse_canonical_power() {
        let mut parser = Parser::new("x ^ 3");
        let result: ParseResult<Canonical<f64, i32>> = parser.parse_canonical();
        assert!(result.is_ok());
        let canonical = result.unwrap();
        assert_eq!(canonical.monomials.len(), 1);
        // 检查幂次是否为 3
        // Check if power is 3
        let powers = &canonical.monomials[0].powers;
        let power_vals: Vec<&i32> = powers.values().collect();
        assert_eq!(power_vals.len(), 1);
        assert_eq!(*power_vals[0], 3);
    }

    #[test]
    fn test_parse_canonical_mixed() {
        let mut parser = Parser::new("x ^ 2 * y + 2 * x + 1");
        let result: ParseResult<Canonical<f64, i32>> = parser.parse_canonical();
        assert!(result.is_ok());
        let canonical = result.unwrap();
        // x^2 * y 是一个单项式，2*x 是一个单项式，常数 1
        // x^2 * y is one monomial, 2*x is one monomial, constant 1
        assert_eq!(canonical.monomials.len(), 2);
        assert_eq!(canonical.constant, 1.0);
    }

    #[test]
    fn test_parse_canonical_high_degree() {
        let mut parser = Parser::new("x ^ 5 + y ^ 3");
        let result: ParseResult<Canonical<f64, i32>> = parser.parse_canonical();
        assert!(result.is_ok());
        let canonical = result.unwrap();
        assert_eq!(canonical.monomials.len(), 2);
    }
}
