//! 日志记录
//! Log record

#[derive(Debug, Clone, Default)]
pub struct LogRecord {
    pub request_id: String,
    pub level: String,
    pub message: String,
}
