//! 错误类型模块
//! Error types module
//!
//! 本模块定义了多维数组库中使用的错误类型：
//! This module defines error types used in the multi-dimensional array library:
//!
//! - `InvalidDummyIndexError`: 无效虚拟索引错误
//!   Invalid dummy index error
//! - `ExInvalidDummyIndexError`: 带原始错误的无效虚拟索引错误
//!   Invalid dummy index error with original error
//! - `DimensionMismatchingError`: 维度不匹配错误
//!   Dimension mismatching error
//! - `OutOfShapeError`: 超出形状范围错误
//!   Out of shape error
//! - `IndexCalculationError`: 索引计算错误枚举
//!   Index calculation error enum
//! - `RepeatMappingIndexError`: 重复映射索引错误
//!   Repeat mapping index error
//! - `MappingIndexError`: 映射索引错误枚举
//!   Mapping index error enum

use std::fmt::{Debug, Display, Formatter};
use ospf_rust_base::error::*;

/// 无效虚拟索引错误
/// Invalid dummy index error
///
/// 当虚拟索引无法转换或解析时返回此错误。
/// This error is returned when a dummy index cannot be converted or parsed.
error_type! {
    #[derive(Clone)]
    pub struct InvalidDummyIndexError {}
}

/// 带原始错误的无效虚拟索引错误
/// Invalid dummy index error with original error
///
/// 与 `InvalidDummyIndexError` 类似，但保留原始错误信息。
/// Similar to `InvalidDummyIndexError`, but preserves original error information.
///
/// ## 类型参数 / Type Parameters
///
/// - `E`: 原始错误类型
///   Original error type
error_type! {
    pub struct ExInvalidDummyIndexError<E> {
        pub origin: E
    }
}

/// `ExInvalidDummyIndexError` 的克隆实现
/// Clone implementation for `ExInvalidDummyIndexError`
impl<E: Clone> Clone for ExInvalidDummyIndexError<E> {
    fn clone(&self) -> Self {
        Self {
            origin: self.origin.clone(),
            position: self.position.clone(),
        }
    }
}

/// `InvalidDummyIndexError` 的 Debug 实现
/// Debug implementation for `InvalidDummyIndexError`
impl Debug for InvalidDummyIndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index")
    }
}

/// `ExInvalidDummyIndexError` 的默认 Debug 实现
/// Default Debug implementation for `ExInvalidDummyIndexError`
impl<E> Debug for ExInvalidDummyIndexError<E> {
    default fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index")
    }
}

/// `ExInvalidDummyIndexError` 的特化 Debug 实现（当 `E: Debug` 时）
/// Specialized Debug implementation for `ExInvalidDummyIndexError` (when `E: Debug`)
impl<E: Debug> Debug for ExInvalidDummyIndexError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index, origin: {:?}", self.origin)
    }
}

/// `InvalidDummyIndexError` 的 Display 实现
/// Display implementation for `InvalidDummyIndexError`
impl Display for InvalidDummyIndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index")
    }
}

/// `ExInvalidDummyIndexError` 的默认 Display 实现
/// Default Display implementation for `ExInvalidDummyIndexError`
impl<E> Display for ExInvalidDummyIndexError<E> {
    default fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index")
    }
}

/// `ExInvalidDummyIndexError` 的特化 Display 实现（当 `E: Debug + Display` 时）
/// Specialized Display implementation for `ExInvalidDummyIndexError` (when `E: Debug + Display`)
impl<E: Debug + Display> Display for ExInvalidDummyIndexError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index, origin: {:?}", self.origin)
    }
}

/// `InvalidDummyIndexError` 的 Error 实现
/// Error implementation for `InvalidDummyIndexError`
impl Error for InvalidDummyIndexError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        "illegal dummy index".to_string()
    }
}

/// `ExInvalidDummyIndexError` 的默认 Error 实现
/// Default Error implementation for `ExInvalidDummyIndexError`
impl<E> Error for ExInvalidDummyIndexError<E> {
    default fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    default fn msg(&self) -> String {
        "illegal dummy index".to_string()
    }
}

/// `ExInvalidDummyIndexError` 的特化 Error 实现（当 `E: Debug + Display` 时）
/// Specialized Error implementation for `ExInvalidDummyIndexError` (when `E: Debug + Display`)
impl<E: Debug + Display> Error for ExInvalidDummyIndexError<E> {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!("illegal dummy index, origin: {:?}", self.origin)
    }
}

/// `ExInvalidDummyIndexError` 的 ExError 实现
/// ExError implementation for `ExInvalidDummyIndexError`
impl<E> ExError<E> for ExInvalidDummyIndexError<E> {
    fn arg(&self) -> Option<&E> {
        Some(&self.origin)
    }
}

/// 维度不匹配错误
/// Dimension mismatching error
///
/// 当操作期望的维度与实际维度不匹配时返回此错误。
/// This error is returned when the expected dimension doesn't match the actual dimension.
///
/// ## 字段 / Fields
///
/// - `dimension`: 期望的维度
///   Expected dimension
/// - `vector_dimension`: 实际的向量维度
///   Actual vector dimension
error_type! {
    #[derive(Clone, Copy)]
    pub struct DimensionMismatchingError {
        pub dimension: usize,
        pub vector_dimension: usize
    }
}

/// `DimensionMismatchingError` 的 Display 实现
/// Display implementation for `DimensionMismatchingError`
impl Display for DimensionMismatchingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "dimension should be {}, not {}",
            self.dimension, self.vector_dimension
        )
    }
}

/// `DimensionMismatchingError` 的 Debug 实现
/// Debug implementation for `DimensionMismatchingError`
impl Debug for DimensionMismatchingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DimensionMismatchingError {{ dimension: {}, vector_dimension: {} }}",
            self.dimension, self.vector_dimension
        )
    }
}

/// `DimensionMismatchingError` 的 Error 实现
/// Error implementation for `DimensionMismatchingError`
impl Error for DimensionMismatchingError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!(
            "dimension should be {}, not {}",
            self.dimension, self.vector_dimension
        )
    }
}

/// 超出形状范围错误
/// Out of shape error
///
/// 当索引超出维度的有效范围时返回此错误。
/// This error is returned when an index is out of the valid range of a dimension.
///
/// ## 字段 / Fields
///
/// - `dimension`: 维度索引
///   Dimension index
/// - `len`: 维度的长度
///   Length of the dimension
/// - `index`: 尝试访问的索引值
///   Index value being accessed
error_type! {
    #[derive(Clone, Copy)]
    pub struct OutOfShapeError {
        pub dimension: usize,
        pub len: usize,
        pub index: isize
    }
}

/// `OutOfShapeError` 的 Display 实现
/// Display implementation for `OutOfShapeError`
impl Display for OutOfShapeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "length of dimension {} is {}, but try to get {}",
            self.dimension, self.len, self.index
        )
    }
}

/// `OutOfShapeError` 的 Debug 实现
/// Debug implementation for `OutOfShapeError`
impl Debug for OutOfShapeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OutOfShapeError {{ dimension: {}, len: {}, index: {} }}",
            self.dimension, self.len, self.index
        )
    }
}

/// `OutOfShapeError` 的 Error 实现
/// Error implementation for `OutOfShapeError`
impl Error for OutOfShapeError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!(
            "length of dimension {} is {}, but try to get {}",
            self.dimension, self.len, self.index
        )
    }
}

/// 索引计算错误枚举
/// Index calculation error enum
///
/// 索引计算过程中可能发生的错误。
/// Errors that may occur during index calculation.
///
/// ## 变体 / Variants
///
/// - `DimensionMismatching`: 维度不匹配错误
///   Dimension mismatching error
/// - `OutOfShape`: 超出形状范围错误
///   Out of shape error
error_enum! {
    #[derive(Clone, Copy)]
    pub enum IndexCalculationError {
        DimensionMismatching(DimensionMismatchingError),
        OutOfShape(OutOfShapeError)
    }
}

/// 重复映射索引错误
/// Repeat mapping index error
///
/// 当映射索引中存在重复的占位符时返回此错误。
/// This error is returned when duplicate placeholders exist in mapping indices.
///
/// ## 字段 / Fields
///
/// - `index`: 重复的占位符索引
///   Duplicate placeholder index
error_type! {
    #[derive(Clone, Copy)]
    pub struct RepeatMappingIndexError {
        pub index: usize
    }
}

/// `RepeatMappingIndexError` 的 Display 实现
/// Display implementation for `RepeatMappingIndexError`
impl Display for RepeatMappingIndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "repeat mapping index {}", self.index)
    }
}

/// `RepeatMappingIndexError` 的 Debug 实现
/// Debug implementation for `RepeatMappingIndexError`
impl Debug for RepeatMappingIndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "repeat mapping index {{ index: {} }}", self.index)
    }
}

/// `RepeatMappingIndexError` 的 Error 实现
/// Error implementation for `RepeatMappingIndexError`
impl Error for RepeatMappingIndexError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!("repeat mapping index {}", self.index)
    }
}

/// 映射索引错误枚举
/// Mapping index error enum
///
/// 映射索引处理过程中可能发生的错误。
/// Errors that may occur during mapping index processing.
///
/// ## 变体 / Variants
///
/// - `RepeatMappingIndex`: 重复映射索引错误
///   Repeat mapping index error
/// - `DimensionMismatching`: 维度不匹配错误
///   Dimension mismatching error
error_enum! {
    #[derive(Clone, Copy)]
    pub enum MappingIndexError {
        RepeatMappingIndex(RepeatMappingIndexError),
        DimensionMismatching(DimensionMismatchingError)
    }
}
