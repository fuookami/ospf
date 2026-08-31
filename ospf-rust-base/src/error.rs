//! 错误处理模块。
//! Error handling module.

use std::fmt::{Debug, Display, Formatter};
use paste::paste;
use strum::{Display, EnumString};

/// 错误码枚举。
/// Error code enumeration.
#[repr(u8)]
#[derive(EnumString, Clone, Copy, Display, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorCode {
    #[strum(serialize = "none")]
    None = 0u8,
    #[strum(serialize = "authentication error")]
    AuthenticationError = 1u8,

    #[strum(serialize = "not a file")]
    NotAFile = 0x10u8,
    #[strum(serialize = "not a directory")]
    NotADirectory = 0x11u8,
    #[strum(serialize = "file not found")]
    FileNotFound = 0x12u8,
    #[strum(serialize = "directory unusable")]
    DirectoryUnusable = 0x13u8,
    #[strum(serialize = "file extension not matched")]
    FileExtensionNotMatched = 0x14u8,
    #[strum(serialize = "data not found")]
    DataNotFound = 0x15u8,
    #[strum(serialize = "data empty")]
    DataEmpty = 0x16u8,
    #[strum(disabled)]
    EnumVisitorEmpty = 0x17u8,
    #[strum(disabled)]
    UniqueBoxLocked = 0x18u8,
    #[strum(disabled)]
    UniqueRefLocked = 0x19u8,
    #[strum(serialize = "serialization failed")]
    SerializationFailed = 0x1au8,
    #[strum(serialize = "deserialization failed")]
    DeserializationFailed = 0x1bu8,

    #[strum(serialize = "token existed")]
    TokenExisted = 0x20u8,
    #[strum(serialize = "symbol repetitive")]
    SymbolRepetitive = 0x21u8,
    #[strum(serialize = "lack of pipelines")]
    LackOfPipelines = 0x22u8,
    #[strum(serialize = "solver not found")]
    SolverNotFound = 0x23u8,
    #[strum(serialize = "OR engine environment lost")]
    OREngineEnvironmentLost = 0x24u8,
    #[strum(serialize = "OR engine connection overtime")]
    OREngineConnectionOvertime = 0x25u8,
    #[strum(serialize = "OR engine modeling exception")]
    OREngineModelingException = 0x26u8,
    #[strum(serialize = "OR engine solving exception")]
    OREngineSolvingException = 0x27u8,
    #[strum(serialize = "OR engine terminated")]
    OREngineTerminated = 0x28u8,
    #[strum(serialize = "OR model infeasible")]
    ORModelInfeasible = 0x29u8,
    #[strum(serialize = "OR model unbounded")]
    ORModelUnbounded = 0x2au8,
    #[strum(serialize = "OR model infeasible or unbounded")]
    ORModelInfeasibleOrUnbounded = 0x2bu8,
    #[strum(serialize = "OR solution invalid")]
    ORSolutionInvalid = 0x2cu8,

    #[strum(serialize = "application failed")]
    ApplicationFailed = 0x30u8,
    #[strum(serialize = "application error")]
    ApplicationError = 0x31u8,
    #[strum(serialize = "application exception")]
    ApplicationException = 0x32u8,
    #[strum(serialize = "application stopped")]
    ApplicationStopped = 0x33u8,
    #[strum(serialize = "illegal argument")]
    IllegalArgument = 0x34u8,

    #[strum(serialize = "other")]
    Other = u8::MAX - 1,
    #[strum(serialize = "unknown")]
    Unknown = u8::MAX,
}

impl ErrorCode {
    /// 从 u8 错误码安全转换，未知值返回 Unknown。
    /// Safely convert from a u8 error code, returning Unknown for unrecognized values.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0x00 => Self::None,
            0x01 => Self::AuthenticationError,
            0x10 => Self::NotAFile,
            0x11 => Self::NotADirectory,
            0x12 => Self::FileNotFound,
            0x13 => Self::DirectoryUnusable,
            0x14 => Self::FileExtensionNotMatched,
            0x15 => Self::DataNotFound,
            0x16 => Self::DataEmpty,
            0x17 => Self::EnumVisitorEmpty,
            0x18 => Self::UniqueBoxLocked,
            0x19 => Self::UniqueRefLocked,
            0x1a => Self::SerializationFailed,
            0x1b => Self::DeserializationFailed,
            0x20 => Self::TokenExisted,
            0x21 => Self::SymbolRepetitive,
            0x22 => Self::LackOfPipelines,
            0x23 => Self::SolverNotFound,
            0x24 => Self::OREngineEnvironmentLost,
            0x25 => Self::OREngineConnectionOvertime,
            0x26 => Self::OREngineModelingException,
            0x27 => Self::OREngineSolvingException,
            0x28 => Self::OREngineTerminated,
            0x29 => Self::ORModelInfeasible,
            0x2a => Self::ORModelUnbounded,
            0x2b => Self::ORModelInfeasibleOrUnbounded,
            0x2c => Self::ORSolutionInvalid,
            0x30 => Self::ApplicationFailed,
            0x31 => Self::ApplicationError,
            0x32 => Self::ApplicationException,
            0x33 => Self::ApplicationStopped,
            0x34 => Self::IllegalArgument,
            0xfe => Self::Other,
            0xff => Self::Unknown,
            _ => Self::Unknown,
        }
    }

    /// 从 u64 错误码安全转换，未知值返回 Unknown。
    /// Safely convert from a u64 error code, returning Unknown for unrecognized values.
    pub fn from_u64(value: u64) -> Self {
        u8::try_from(value)
            .map(Self::from_u8)
            .unwrap_or(Self::Unknown)
    }

    /// 转为 u8 错误码。
    /// Convert to a u8 error code.
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// 转为 u64 错误码。
    /// Convert to a u64 error code.
    pub fn to_u64(self) -> u64 {
        self.to_u8().into()
    }
}

impl From<u8> for ErrorCode {
    fn from(value: u8) -> ErrorCode {
        Self::from_u8(value)
    }
}

impl From<u16> for ErrorCode {
    fn from(value: u16) -> ErrorCode {
        u8::try_from(value)
            .map(Self::from_u8)
            .unwrap_or(Self::Unknown)
    }
}

impl From<u32> for ErrorCode {
    fn from(value: u32) -> ErrorCode {
        u8::try_from(value)
            .map(Self::from_u8)
            .unwrap_or(Self::Unknown)
    }
}

impl From<u64> for ErrorCode {
    fn from(value: u64) -> ErrorCode {
        Self::from_u64(value)
    }
}

impl From<u128> for ErrorCode {
    fn from(value: u128) -> ErrorCode {
        u64::try_from(value)
            .map(Self::from_u64)
            .unwrap_or(Self::Unknown)
    }
}

impl From<usize> for ErrorCode {
    fn from(value: usize) -> ErrorCode {
        u64::try_from(value)
            .map(Self::from_u64)
            .unwrap_or(Self::Unknown)
    }
}

impl From<ErrorCode> for u8 {
    fn from(value: ErrorCode) -> u8 {
        value as u8
    }
}

impl From<ErrorCode> for u16 {
    fn from(value: ErrorCode) -> u16 {
        (value as u8).into()
    }
}

impl From<ErrorCode> for u32 {
    fn from(value: ErrorCode) -> u32 {
        (value as u8).into()
    }
}

impl From<ErrorCode> for u64 {
    fn from(value: ErrorCode) -> u64 {
        (value as u8).into()
    }
}

impl From<ErrorCode> for u128 {
    fn from(value: ErrorCode) -> u128 {
        (value as u8).into()
    }
}

impl From<ErrorCode> for usize {
    fn from(value: ErrorCode) -> usize {
        (value as u8).into()
    }
}

/// 错误位置信息。
/// Error position information.
#[derive(Clone, Copy)]
pub struct ErrorPosition {
    /// 源文件名。
    /// Source file name.
    pub file: &'static str,
    /// 行号。
    /// Line number.
    pub line: u32,
}

impl Display for ErrorPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

impl Debug for ErrorPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

/// 带错误位置的trait。
/// Trait for types with error position information.
pub trait WithErrorPosition {
    /// 返回错误位置。
    /// Returns the error position.
    fn position(&self) -> &ErrorPosition;
}

/// 错误trait。
/// Error trait.
pub trait Error: Display + Debug {
    /// 返回错误码。
    /// Returns the error code.
    fn code(&self) -> ErrorCode;
    /// 返回错误消息。
    /// Returns the error message.
    fn msg(&self) -> String;
}

/// 带附加参数的错误trait。
/// Error trait with additional argument.
pub trait ExError<T: Sized>: Error {
    /// 返回附加参数引用。
    /// Returns reference to the additional argument.
    fn arg(&self) -> Option<&T>;
}

/// 成功标记类型。
/// Success marker type.
pub struct Ok {}
/// 成功标记常量。
/// Success marker constant.
pub const OK: Ok = Ok {};

impl<E> From<Ok> for std::result::Result<(), E> {
    fn from(_: Ok) -> Self {
        Ok(())
    }
}

/// 返回类型别名。
/// Return type alias.
pub type Ret<T> = Result<T, Box<dyn Error>>;
/// Try类型别名。
/// Try type alias.
pub type Try = Result<(), Box<dyn Error>>;
/// 带附加参数的返回类型别名。
/// Extended return type alias with additional argument.
pub type ExRet<T> = ExResult<T, Box<dyn Error>>;
/// 带附加参数的Try类型别名。
/// Extended try type alias with additional argument.
pub type ExTry = ExResult<(), Box<dyn Error>>;

/// 扩展结果类型，支持警告和致命错误。
/// Extended result type supporting warnings and fatal errors.
#[derive(Clone, Debug)]
pub enum ExResult<T, E> {
    Ok(T),
    Failed(E),
    Warn { value: T, warnings: Vec<E> },
    Fatal(Vec<E>),
}

impl<T, E> ExResult<T, E> {
    pub fn ok(value: T) -> Self {
        Self::Ok(value)
    }

    pub fn failed(error: E) -> Self {
        Self::Failed(error)
    }

    pub fn warning(value: T, warning: E) -> Self {
        Self::Warn {
            value,
            warnings: vec![warning],
        }
    }

    pub fn warn(value: T, warnings: Vec<E>) -> Self {
        Self::Warn { value, warnings }
    }

    pub fn fatal_error(error: E) -> Self {
        Self::Fatal(vec![error])
    }

    pub fn fatal(errors: Vec<E>) -> Self {
        Self::Fatal(errors)
    }

    #[deprecated(note = "Use fatal_error instead")]
    pub fn fetal_error(error: E) -> Self {
        Self::Fatal(vec![error])
    }

    #[deprecated(note = "Use fatal instead")]
    pub fn fetal(errors: Vec<E>) -> Self {
        Self::Fatal(errors)
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_) | Self::Fatal(_))
    }

    pub fn is_warned(&self) -> bool {
        matches!(self, Self::Warn { .. })
    }

    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::Fatal(_))
    }

    #[deprecated(note = "Use is_fatal instead")]
    pub fn is_fetal(&self) -> bool {
        self.is_fatal()
    }

    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Ok(value) => Some(value),
            Self::Warn { value, .. } => Some(value),
            Self::Failed(_) | Self::Fatal(_) => None,
        }
    }

    pub fn value_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Ok(value) => Some(value),
            Self::Warn { value, .. } => Some(value),
            Self::Failed(_) | Self::Fatal(_) => None,
        }
    }

    pub fn failed_error(&self) -> Option<&E> {
        match self {
            Self::Failed(error) => Some(error),
            Self::Ok(_) | Self::Warn { .. } | Self::Fatal(_) => None,
        }
    }

    pub fn warnings(&self) -> Option<&[E]> {
        match self {
            Self::Warn { warnings, .. } => Some(warnings.as_slice()),
            Self::Ok(_) | Self::Failed(_) | Self::Fatal(_) => None,
        }
    }

    pub fn fatal_errors(&self) -> Option<&[E]> {
        match self {
            Self::Fatal(errors) => Some(errors.as_slice()),
            Self::Ok(_) | Self::Failed(_) | Self::Warn { .. } => None,
        }
    }

    #[deprecated(note = "Use fatal_errors instead")]
    pub fn fetal_errors(&self) -> Option<&[E]> {
        self.fatal_errors()
    }

    pub fn map<U, F>(self, transform: F) -> ExResult<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Ok(value) => ExResult::Ok(transform(value)),
            Self::Warn { value, warnings } => ExResult::Warn {
                value: transform(value),
                warnings,
            },
            Self::Failed(error) => ExResult::Failed(error),
            Self::Fatal(errors) => ExResult::Fatal(errors),
        }
    }
}

impl<T, E> From<std::result::Result<T, E>> for ExResult<T, E> {
    fn from(value: std::result::Result<T, E>) -> Self {
        match value {
            std::result::Result::Ok(value) => Self::Ok(value),
            std::result::Result::Err(error) => Self::Failed(error),
        }
    }
}

impl From<Ok> for ExTry {
    fn from(_: Ok) -> Self {
        ExResult::Ok(())
    }
}

// ============================================================================
// 惰性错误消息支持 / Lazy error message support
// ============================================================================

/// 惰性消息错误 / Lazy message error
///
/// 延迟消息构造到首次访问时，使用 `OnceLock` 和闭包。
/// Defers message construction until first access using `OnceLock` and closures.
pub struct LazyErr {
    code: ErrorCode,
    message: std::sync::OnceLock<String>,
    message_fn: Box<dyn Fn() -> String + Send + Sync>,
}

impl LazyErr {
    /// 创建新的惰性错误 / Create a new lazy error
    pub fn new(code: ErrorCode, message_fn: impl Fn() -> String + Send + Sync + 'static) -> Self {
        Self {
            code,
            message: std::sync::OnceLock::new(),
            message_fn: Box::new(message_fn),
        }
    }

    /// 获取惰性构造的消息 / Get the lazily constructed message
    pub fn message(&self) -> &str {
        self.message.get_or_init(|| (self.message_fn)())
    }
}

impl Display for LazyErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl Debug for LazyErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "LazyErr({:?}, {})", self.code, self.message())
    }
}

impl Error for LazyErr {
    fn code(&self) -> ErrorCode {
        self.code
    }

    fn msg(&self) -> String {
        self.message().to_string()
    }
}

/// 带附加参数的惰性消息错误 / Lazy message error with additional context
///
/// 对应 Kotlin `LazyExErr<C, T>`，延迟消息构造并携带额外上下文。
/// Corresponds to Kotlin `LazyExErr<C, T>`, defers message construction and carries context.
pub struct LazyExErr<C> {
    code: ErrorCode,
    arg: C,
    message: std::sync::OnceLock<String>,
    message_fn: Box<dyn Fn(&C) -> String + Send + Sync>,
}

impl<C: Send + Sync> LazyExErr<C> {
    /// 创建带上下文的惰性错误 / Create a lazy error with context
    pub fn new(
        code: ErrorCode,
        arg: C,
        message_fn: impl Fn(&C) -> String + Send + Sync + 'static,
    ) -> Self {
        Self {
            code,
            arg,
            message: std::sync::OnceLock::new(),
            message_fn: Box::new(message_fn),
        }
    }

    /// 获取附加参数引用 / Get reference to the additional argument
    pub fn arg(&self) -> &C {
        &self.arg
    }

    /// 获取惰性构造的消息 / Get the lazily constructed message
    pub fn message(&self) -> &str {
        let arg = &self.arg;
        self.message.get_or_init(|| (self.message_fn)(arg))
    }
}

impl<C: Send + Sync> Display for LazyExErr<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl<C: Send + Sync> Debug for LazyExErr<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "LazyExErr({:?}, {})", self.code, self.message())
    }
}

impl<C: Send + Sync> Error for LazyExErr<C> {
    fn code(&self) -> ErrorCode {
        self.code
    }

    fn msg(&self) -> String {
        self.message().to_string()
    }
}

/// 定义错误结构体类型的宏。
/// Macro for defining error struct types.
///
/// 自动添加`position`字段并实现`WithErrorPosition`trait。
/// Automatically adds a `position` field and implements the `WithErrorPosition` trait.
#[macro_export]
macro_rules! error_type {
    ($(#[$derive:meta])* $vis:vis struct $name:ident $(< $( $param:tt ),* >)? { $( $(#[$fieldMeta:meta])* $fieldVis:vis $field:ident: $type:ty ),* $(,)? }) => {
        $(#[$derive])*
        $vis struct $name $(< $( $param ),* >)? {
            $( $(#[$fieldMeta])* $fieldVis $field: $type, )*
            pub position: ErrorPosition
        }

        impl $(< $( $param ),* >)? WithErrorPosition for $name $(< $( $param ),* >)? {
            fn position(&self) -> &ErrorPosition {
                &self.position
            }
        }
    };
}

/// 定义错误枚举类型的宏。
/// Macro for defining error enum types.
///
/// 自动实现`From`、`WithErrorPosition`、`Debug`、`Display`和`Error`trait。
/// Automatically implements `From`, `WithErrorPosition`, `Debug`, `Display`, and `Error` traits.
#[macro_export]
macro_rules! error_enum {
    ($(#[$derive:meta])* $vis:vis enum $name:ident {
        $($visVariant:vis $variant:ident($errorType:ty)),* $(,)?
    }) => {
        $(#[$derive])*
        $vis enum $name {
            $($visVariant $variant($errorType),)*
        }

        $(impl From<$errorType> for $name {
            fn from(value: $errorType) -> Self {
                Self::$variant(value)
            }
        })*

        ::paste::paste! {
            impl $name {
                $(pub fn [<$variant:snake>](err: $errorType) -> Self {
                    Self::$variant(err)
                })*
            }
        }

        impl WithErrorPosition for $name {
            fn position(&self) -> &ErrorPosition {
                match self {
                    $(Self::$variant(err) => err.position(),)*
                }
            }
        }

        impl Debug for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(Self::$variant(err) => Debug::fmt(err, f),)*
                }
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(Self::$variant(err) => Display::fmt(err, f),)*
                }
            }
        }

        impl Error for $name {
            fn code(&self) -> ErrorCode {
                match self {
                    $(Self::$variant(err) => err.code(),)*
                }
            }

            fn msg(&self) -> String {
                match self {
                    $(Self::$variant(err) => err.msg(),)*
                }
            }
        }
    };
}

/// 创建错误实例的宏。
/// Macro for creating error instances.
///
/// 自动填充`position`字段为当前文件和行号。
/// Automatically fills the `position` field with current file and line number.
#[macro_export]
macro_rules! error {
    ($name:ident { $($field:ident: $val:expr),* }) => {
        $name {
            $($field: $val,)*
            position: ErrorPosition {
                file: file!(),
                line: line!(),
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    error_type!(
        #[derive(Clone, Copy)]
        pub struct TestError {
            pub message: &'static str,
        }
    );

    impl Display for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl Debug for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl Error for TestError {
        fn code(&self) -> ErrorCode {
            ErrorCode::Other
        }

        fn msg(&self) -> String {
            self.message.to_string()
        }
    }

    error_type!(
        #[derive(Clone, Copy)]
        pub struct AnotherError {
            pub code: u32,
        }
    );

    impl Display for AnotherError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "Error code: {}", self.code)
        }
    }

    impl Debug for AnotherError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "Error code: {}", self.code)
        }
    }

    impl Error for AnotherError {
        fn code(&self) -> ErrorCode {
            ErrorCode::IllegalArgument
        }

        fn msg(&self) -> String {
            format!("Error code: {}", self.code)
        }
    }

    error_enum!(
        #[derive(Clone, Copy)]
        pub enum TestErrorEnum {
            Test(TestError),
            Another(AnotherError),
        }
    );

    #[test]
    fn test_file_line() {
        let error = error!(TestError {
            message: "test error"
        });
        assert_eq!(error.position().file, file!());
        assert_eq!(error.position().line, line!() - 4);
    }

    #[test]
    fn test_error_enum() {
        let test_error = error!(TestError {
            message: "test error"
        });
        let another_error = error!(AnotherError { code: 404 });

        let enum1: TestErrorEnum = test_error.into();
        let enum2: TestErrorEnum = another_error.into();

        let enum3 = TestErrorEnum::Test(error!(TestError {
            message: "another test"
        }));
        let enum4 = TestErrorEnum::Another(error!(AnotherError { code: 500 }));
    }

    #[test]
    fn test_ex_result_warn() {
        let result = ExResult::warning(42u32, error!(TestError { message: "warning" }));

        assert!(!result.is_ok());
        assert!(!result.is_failed());
        assert!(result.is_warned());
        assert_eq!(result.value(), Some(&42u32));

        let warnings = result.warnings().unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].msg(), "warning".to_string());
    }

    #[test]
    fn test_ex_result_fatal_primary() {
        let result: ExResult<u32, TestError> = ExResult::fatal(vec![
            error!(TestError { message: "fatal-1" }),
            error!(TestError { message: "fatal-2" }),
        ]);

        assert!(!result.is_ok());
        assert!(result.is_failed());
        assert!(result.is_fatal());

        let errors = result.fatal_errors().unwrap();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].msg(), "fatal-1".to_string());
        assert_eq!(errors[1].msg(), "fatal-2".to_string());
    }

    #[test]
    #[allow(deprecated)]
    fn test_ex_result_fetal_alias_compat() {
        let result: ExResult<u32, TestError> = ExResult::fetal(vec![
            error!(TestError { message: "fatal-1" }),
        ]);

        assert!(result.is_fetal());

        let errors = result.fetal_errors().unwrap();
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_ex_result_map() {
        let warned = ExResult::warning(10u32, error!(TestError { message: "warn" }));
        let mapped = warned.map(|value| value + 5);
        assert!(mapped.is_warned());
        assert_eq!(mapped.value(), Some(&15u32));

        let fatal: ExResult<u32, TestError> =
            ExResult::fatal_error(error!(TestError { message: "fatal" }));
        let mapped_fatal = fatal.map(|value| value + 5);
        assert!(mapped_fatal.is_fatal());
        assert!(mapped_fatal.value().is_none());
    }

    #[test]
    fn test_error_code_safe_conversion() {
        assert_eq!(
            ErrorCode::from_u8(0x2b),
            ErrorCode::ORModelInfeasibleOrUnbounded
        );
        assert_eq!(ErrorCode::from_u8(0x7f), ErrorCode::Unknown);
        assert_eq!(ErrorCode::from_u64(0x1_0000), ErrorCode::Unknown);
        assert_eq!(ErrorCode::IllegalArgument.to_u8(), 0x34);
        assert_eq!(ErrorCode::Other.to_u64(), 0xfe);
    }
}

pub use crate::error;
pub use crate::error_enum;
pub use crate::error_type;
