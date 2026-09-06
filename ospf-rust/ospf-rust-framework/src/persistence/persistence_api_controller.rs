//! 持久化 API 控制器占位
//! Persistence API controller placeholder

use crate::persistence::request::PersistenceRequest;

/// 持久化 API 控制器接口。
/// Persistence API controller interface.
pub trait PersistenceApiController: Send + Sync {
    /// 持久化请求。
    /// Persist a request.
    fn persist(&self, _request: &PersistenceRequest) -> Result<(), String> {
        Ok(())
    }
}
