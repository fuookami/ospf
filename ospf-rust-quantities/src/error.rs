//! 量的错误类型 / Error types for quantities.

use std::fmt::{Debug, Display, Formatter};
use ospf_rust_base::{Error, ErrorCode, ErrorPosition, WithErrorPosition, error_type};

error_type!(
    /// 量纲不匹配错误 / Dimension mismatch error
    ///
    /// 当操作中两个量的量纲不一致时返回此错误。
    /// This error is returned when the dimensions of two quantities
    /// do not match in an operation.
    #[derive(Clone)]
    pub struct DimensionMismatchError {
        /// 期望的量纲符号 / Expected dimension symbol
        pub expected: String,
        /// 实际的量纲符号 / Actual dimension symbol
        pub actual: String,
        /// 正在执行的操作名称 / Name of the operation being performed
        pub operation: &'static str,
    }
);

impl Display for DimensionMismatchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Dimension mismatch in {}: expected '{}', got '{}'",
            self.operation, self.expected, self.actual
        )
    }
}

impl Debug for DimensionMismatchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DimensionMismatchError {{ expected: '{}', actual: '{}', operation: '{}' }}",
            self.expected, self.actual, self.operation
        )
    }
}

impl Error for DimensionMismatchError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!(
            "Dimension mismatch in {}: expected '{}', got '{}'",
            self.operation, self.expected, self.actual
        )
    }
}

error_type!(
    /// 单位转换错误 / Unit conversion error
    ///
    /// 当单位转换无法完成时返回此错误。
    /// This error is returned when a unit conversion cannot be completed.
    #[derive(Clone)]
    pub struct UnitConversionError {
        /// 源单位 / Source unit
        pub from_unit: String,
        /// 目标单位 / Target unit
        pub to_unit: String,
        /// 转换失败的原因 / Reason for the conversion failure
        pub reason: &'static str,
    }
);

impl Display for UnitConversionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cannot convert unit '{}' to '{}': {}",
            self.from_unit, self.to_unit, self.reason
        )
    }
}

impl Debug for UnitConversionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UnitConversionError {{ from: '{}', to: '{}', reason: '{}' }}",
            self.from_unit, self.to_unit, self.reason
        )
    }
}

impl Error for UnitConversionError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!(
            "Cannot convert unit '{}' to '{}': {}",
            self.from_unit, self.to_unit, self.reason
        )
    }
}

error_type!(
    /// 符号维度注册表错误 / Symbol dimension registry error
    ///
    /// 当符号维度注册表中出现错误时返回此错误。
    /// This error is returned when an error occurs in the symbol dimension registry.
    #[derive(Clone)]
    pub struct SymbolRegistryError {
        /// 符号名称 / Symbol name
        pub symbol: String,
        /// 错误原因 / Reason for the error
        pub reason: &'static str,
    }
);

impl Display for SymbolRegistryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Symbol registry error for '{}': {}",
            self.symbol, self.reason
        )
    }
}

impl Debug for SymbolRegistryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SymbolRegistryError {{ symbol: '{}', reason: '{}' }}",
            self.symbol, self.reason
        )
    }
}

impl Error for SymbolRegistryError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!(
            "Symbol registry error for '{}': {}",
            self.symbol, self.reason
        )
    }
}
