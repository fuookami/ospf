//! # Error - 错误类型定义
//!
//! ## Overview / 概述
//!
//! This module defines error types used throughout the multiarray library.
//! Errors cover various failure scenarios including:
//! - Invalid dummy index operations / 无效虚拟索引操作
//! - Dimension mismatches / 维度不匹配
//! - Out-of-bounds access / 越界访问
//! - Mapping index errors / 映射索引错误
//!
//! 本模块定义了整个 multiarray 库中使用的错误类型。
//! 错误涵盖了各种失败场景，包括：
//! - 无效虚拟索引操作
//! - 维度不匹配
//! - 越界访问
//! - 映射索引错误
//!
//! ## Error Types / 错误类型
//!
//! - `InvalidDummyIndexError` - Invalid dummy index / 无效虚拟索引
//! - `ExInvalidDummyIndexError<E>` - Invalid dummy index with origin error / 带有原始错误的无效虚拟索引
//! - `DimensionMismatchingError` - Dimension mismatch / 维度不匹配
//! - `OutOfShapeError` - Index out of bounds / 索引越界
//! - `IndexCalculationError` - Index calculation errors / 索引计算错误
//! - `RepeatMappingIndexError` - Repeated mapping index / 重复映射索引
//! - `MappingIndexError` - Mapping index errors / 映射索引错误

use ospf_rust_base::error::*;
use std::fmt::{Debug, Display, Formatter};

/// # InvalidDummyIndexError
///
/// Error returned when a dummy index is invalid.
///
/// 当虚拟索引无效时返回的错误。
error_type! {
    #[derive(Clone)]
    pub struct InvalidDummyIndexError {}
}

/// # ExInvalidDummyIndexError
///
/// Error returned when a dummy index conversion fails, containing the original error.
///
/// 当虚拟索引转换失败时返回的错误，包含原始错误。
///
/// ## Type Parameters / 类型参数
///
/// - `E` - The original error type / 原始错误类型
///
/// ## Fields / 字段
///
/// - `origin` - The original error that caused this error / 导致此错误的原始错误
error_type! {
    pub struct ExInvalidDummyIndexError<E> {
        pub origin: E
    }
}

impl<E: Clone> Clone for ExInvalidDummyIndexError<E> {
    fn clone(&self) -> Self {
        Self {
            origin: self.origin.clone(),
            position: self.position.clone(),
        }
    }
}

impl Debug for InvalidDummyIndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index")
    }
}

impl<E> Debug for ExInvalidDummyIndexError<E> {
    default fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index")
    }
}

impl<E: Debug> Debug for ExInvalidDummyIndexError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index, origin: {:?}", self.origin)
    }
}

impl Display for InvalidDummyIndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index")
    }
}

impl<E> Display for ExInvalidDummyIndexError<E> {
    default fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index")
    }
}

impl<E: Debug + Display> Display for ExInvalidDummyIndexError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal dummy index, origin: {:?}", self.origin)
    }
}

impl Error for InvalidDummyIndexError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        "illegal dummy index".to_string()
    }
}

impl<E> Error for ExInvalidDummyIndexError<E> {
    default fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    default fn msg(&self) -> String {
        "illegal dummy index".to_string()
    }
}

impl<E: Debug + Display> Error for ExInvalidDummyIndexError<E> {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!("illegal dummy index, origin: {:?}", self.origin)
    }
}

impl<E> ExError<E> for ExInvalidDummyIndexError<E> {
    fn arg(&self) -> Option<&E> {
        Some(&self.origin)
    }
}

/// # DimensionMismatchingError
///
/// Error returned when a vector's dimension doesn't match the expected dimension.
///
/// 当向量的维度与预期维度不匹配时返回的错误。
///
/// ## Fields / 字段
///
/// - `dimension` - The expected dimension / 预期的维度
/// - `vector_dimension` - The actual vector dimension / 实际的向量维度
error_type! {
    #[derive(Clone, Copy)]
    pub struct DimensionMismatchingError {
        pub dimension: usize,
        pub vector_dimension: usize
    }
}

impl Display for DimensionMismatchingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "dimension should be {}, not {}",
            self.dimension, self.vector_dimension
        )
    }
}

impl Debug for DimensionMismatchingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DimensionMismatchingError {{ dimension: {}, vector_dimension: {} }}",
            self.dimension, self.vector_dimension
        )
    }
}

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

/// # OutOfShapeError
///
/// Error returned when an index is out of the valid range for a dimension.
///
/// 当索引超出维度的有效范围时返回的错误。
///
/// ## Fields / 字段
///
/// - `dimension` - The dimension index / 维度索引
/// - `len` - The valid length of the dimension / 维度的有效长度
/// - `index` - The attempted index / 尝试的索引
error_type! {
    #[derive(Clone, Copy)]
    pub struct OutOfShapeError {
        pub dimension: usize,
        pub len: usize,
        pub index: isize
    }
}

impl Display for OutOfShapeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "length of dimension {} is {}, but try to get {}",
            self.dimension, self.len, self.index
        )
    }
}

impl Debug for OutOfShapeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OutOfShapeError {{ dimension: {}, len: {}, index: {} }}",
            self.dimension, self.len, self.index
        )
    }
}

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

/// # IndexCalculationError
///
/// Error type for index calculation failures.
///
/// 索引计算失败的错误类型。
///
/// ## Variants / 变体
///
/// - `DimensionMismatching` - Vector dimension doesn't match / 向量维度不匹配
/// - `OutOfShape` - Index is out of bounds / 索引越界
error_enum! {
    #[derive(Clone, Copy)]
    pub enum IndexCalculationError {
        DimensionMismatching(DimensionMismatchingError),
        OutOfShape(OutOfShapeError)
    }
}

/// # RepeatMappingIndexError
///
/// Error returned when a dimension is mapped multiple times in a mapping vector.
///
/// 当维度在映射向量中被多次映射时返回的错误。
///
/// ## Fields / 字段
///
/// - `index` - The repeated dimension index / 重复的维度索引
error_type! {
    #[derive(Clone, Copy)]
    pub struct RepeatMappingIndexError {
        pub index: usize
    }
}

impl Display for RepeatMappingIndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "repeat mapping index {}", self.index)
    }
}

impl Debug for RepeatMappingIndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "repeat mapping index {{ index: {} }}", self.index)
    }
}

impl Error for RepeatMappingIndexError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!("repeat mapping index {}", self.index)
    }
}

/// # MappingIndexError
///
/// Error type for mapping index operations.
///
/// 映射索引操作的错误类型。
///
/// ## Variants / 变体
///
/// - `RepeatMappingIndex` - A dimension is mapped multiple times / 维度被多次映射
/// - `DimensionMismatching` - Dimension mismatch in mapping / 映射中的维度不匹配
error_enum! {
    #[derive(Clone, Copy)]
    pub enum MappingIndexError {
        RepeatMappingIndex(RepeatMappingIndexError),
        DimensionMismatching(DimensionMismatchingError)
    }
}
