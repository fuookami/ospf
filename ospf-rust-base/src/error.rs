use paste::paste;
use std::fmt::{Debug, Display, Formatter};
use strum::{Display, EnumString};

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

impl From<u8> for ErrorCode {
    fn from(value: u8) -> ErrorCode {
        unsafe { std::mem::transmute(value) }
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

#[derive(Clone, Copy)]
pub struct ErrorPosition {
    pub file: &'static str,
    pub line: u32
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

pub trait WithErrorPosition {
    fn position(&self) -> &ErrorPosition;
}

pub trait Error: Display + Debug {
    fn code(&self) -> ErrorCode;
    fn msg(&self) -> String;
}

pub trait ExError<T: Sized>: Error {
    fn arg(&self) -> Option<&T>;
}

pub struct Ok {}
pub const OK: Ok = Ok {};

impl<E> From<Ok> for std::result::Result<(), E> {
    fn from(_: Ok) -> Self {
        Ok(())
    }
}

pub type Ret<T> = Result<T, Box<dyn Error>>;
pub type Try = Result<(), Box<dyn Error>>;

#[macro_export]
macro_rules! error_type {
    ($(#[$derive:meta])* $vis:vis struct $name:ident $(< $( $param:tt ),* >)? { $($fieldVis:vis $field:ident: $type:ty),* }) => {
        $(#[$derive])*
        $vis struct $name $(< $( $param ),* >)? {
            $($fieldVis $field: $type,)*
            pub position: ErrorPosition
        }

        impl $(< $( $param ),* >)? WithErrorPosition for $name $(< $( $param ),* >)? {
            fn position(&self) -> &ErrorPosition {
                &self.position
            }
        }
    };
}

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

        impl $name {
            $(pub fn $variant(err: $errorType) -> Self {
                Self::$variant(err)
            })*
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
            pub message: &'static str
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
            pub code: u32
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
        let error = error!(TestError { message: "test error" });
        assert_eq!(error.position().file, file!());
        assert_eq!(error.position().line, line!() - 2);
    }

    #[test]
    fn test_error_enum() {
        let test_error = error!(TestError { message: "test error" });
        let another_error = error!(AnotherError { code: 404 });

        let enum1: TestErrorEnum = test_error.into();
        let enum2: TestErrorEnum = another_error.into();

        let enum3 = TestErrorEnum::Test(error!(TestError { message: "another test" }));
        let enum4 = TestErrorEnum::Another(error!(AnotherError { code: 500 }));
    }
}

pub use crate::error_type;
pub use crate::error;
pub use crate::error_enum;