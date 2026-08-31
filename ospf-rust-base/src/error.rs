use paste::paste;
use strum::{Display, EnumString};

#[repr(u8)]
#[derive(EnumString, Clone, Copy, Display, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorCode {
    #[strum(serialize = "none")]
    None = 0u8,

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
    #[strum(serialize = "OR model no solution")]
    ORModelNoSolution = 0x29u8,
    #[strum(serialize = "OR model unbounded")]
    ORModelUnbounded = 0x2au8,
    #[strum(serialize = "OR solution invalid")]
    ORSolutionInvalid = 0x2bu8,

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

pub trait Error {
    fn msg(&self) -> &str;
}

pub trait ExError<T: Sized>: Error {
    fn arg(&self) -> &Option<T>;
}

pub trait LogicError: Error {}

pub trait RuntimeError: Error {
    fn code(&self) -> ErrorCode;
}

pub trait ExLogicError<T: Sized>: LogicError + ExError<T> {}

pub trait ExRuntimeError<T: Sized>: RuntimeError + ExError<T> {}

macro_rules! logic_error_template {
    ($($type:ident)*) => ($(
        pub struct $type {
            msg: String
        }

        impl Error for $type {
            fn msg(&self) -> &str { &self.msg }
        }

        impl LogicError for $type {}

        paste! {
            pub struct [<Ex $type>]<T: Sized> {
                msg: String,
                arg: Option<T>
            }

            impl<T: Sized> Error for [<Ex $type>]<T> {
                fn msg(&self) -> &str { &self.msg }
            }

            impl<T: Sized> ExError<T> for [<Ex $type>]<T> {
                fn arg(&self) -> &Option<T> { &self.arg }
            }

            impl<T: Sized> LogicError for [<Ex $type>]<T> {}
            impl<T: Sized> ExLogicError<T> for [<Ex $type>]<T> {}
        }
    )*)
}
logic_error_template! { InvalidArgument DomainError LengthError OutOfRange }

macro_rules! runtime_error_template {
    ($($type:ident)*) => ($(
        pub struct $type {
            code: ErrorCode,
            msg: String
        }

        impl Error for $type {
            fn msg(&self) -> &str { &self.msg }
        }

        impl RuntimeError for $type {
            fn code(&self) -> ErrorCode { self.code }
        }

        paste! {
            pub struct [<Ex $type>]<T: Sized> {
                code: ErrorCode,
                msg: String,
                arg: Option<T>
            }

            impl<T: Sized> Error for [<Ex $type>]<T> {
                fn msg(&self) -> &str { &self.msg }
            }

            impl<T: Sized> ExError<T> for [<Ex $type>]<T> {
                fn arg(&self) -> &Option<T> { &self.arg }
            }

            impl<T: Sized> RuntimeError for [<Ex $type>]<T> {
                fn code(&self) -> ErrorCode { self.code }
            }

            impl<T: Sized> ExRuntimeError<T> for [<Ex $type>]<T> {}
        }
    )*)
}
runtime_error_template! { ApplicationError RangeError OverflowError UnderflowError SystemError FilesystemError }

pub struct Ok {}
pub const OK: Ok = Ok {};

impl<E> From<Ok> for Result<(), E> {
    fn from(_: Ok) -> Self {
        Ok(())
    }
}

pub type RuntimeResult<T> = Result<T, Box<dyn RuntimeError>>;
pub type Try = Result<(), Box<dyn RuntimeError>>;
