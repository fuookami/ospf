use concat_idents::concat_idents;

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

    #[strum(serialize = "other")]
    Other = u8::MAX - 1,
    #[strum(serialize = "unknown")]
    Unknown = u8::MAX,
}

impl From<u8> for ErrorCode {
    fn from(value: u8) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl Into<u8> for ErrorCode {
    fn into(self) -> u8 {
        self as u8
    }
}

trait Error {
    fn what(&self) -> &str;
}

trait ExError<T: Sized>: Error {
    fn arg(&self) -> &Option<T>;
}

trait LogicError: Error {}
trait RuntimeError: Error {
    fn code(&self) -> ErrorCode;
}

trait ExLogicError<T: Sized>: LogicError + ExError<T> {}
trait ExRuntimeError<T: Sized>: RuntimeError + ExError<T> {}

macro_rules! logic_error_template {
    ($($type:ident)*) => ($(
        pub struct $type {
            msg: String
        }

        impl Error for $type {
            fn what(&self) -> &str { &self.msg }
        }

        impl LogicError for $type {}

        concat_idents!(ex_error_name = Ex, $type {
            pub struct ex_error_name<T: Sized> {
                msg: String,
                arg: Option<T>
            }

            impl<T: Sized> Error for ex_error_name<T> {
                fn what(&self) -> &str { &self.msg }
            }

            impl<T: Sized> ExError<T> for ex_error_name<T> {
                fn arg(&self) -> &Option<T> { &self.arg }
            }

            impl<T: Sized> LogicError for ex_error_name<T> {}
            impl<T: Sized> ExLogicError<T> for ex_error_name<T> {}
        });
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
            fn what(&self) -> &str { &self.msg }
        }

        impl RuntimeError for $type {
            fn code(&self) -> ErrorCode { self.code }
        }

        concat_idents!(ex_error_name = Ex, $type {
            pub struct ex_error_name<T: Sized> {
                code: ErrorCode,
                msg: String,
                arg: Option<T>
            }

            impl<T: Sized> Error for ex_error_name<T> {
                fn what(&self) -> &str { &self.msg }
            }

            impl<T: Sized> ExError<T> for ex_error_name<T> {
                fn arg(&self) -> &Option<T> { &self.arg }
            }

            impl<T: Sized> RuntimeError for ex_error_name<T> {
                fn code(&self) -> ErrorCode { self.code }
            }

            impl<T: Sized> ExRuntimeError<T> for ex_error_name<T> {}
        });
    )*)
}
runtime_error_template! { ApplicationError RangeError OverflowError UnderflowError SystemError FilesystemError }
