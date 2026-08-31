//! Error types for quantities.

use std::fmt::{Debug, Display, Formatter};
use ospf_rust_base::{Error, ErrorCode, ErrorPosition, WithErrorPosition, error_type};

error_type!(
    #[derive(Clone)]
    pub struct DimensionMismatchError {
        pub expected: String,
        pub actual: String,
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
    #[derive(Clone)]
    pub struct UnitConversionError {
        pub from_unit: String,
        pub to_unit: String,
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
    #[derive(Clone)]
    pub struct SymbolRegistryError {
        pub symbol: String,
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
