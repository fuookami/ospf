//! 日志记录
//! Log record

/// 日志记录 / Log record
#[derive(Debug, Clone, Default)]
pub struct LogRecord {
    /// 请求 ID / Request ID
    pub request_id: String,
    /// 日志级别 / Log level
    pub level: String,
    /// 日志消息 / Log message
    pub message: String,
}
