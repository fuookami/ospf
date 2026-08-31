//! 持久化 API 控制器占位
//! Persistence API controller placeholder

use crate::persistence::request::PersistenceRequest;

pub trait PersistenceApiController: Send + Sync {
    fn persist(&self, _request: &PersistenceRequest) -> Result<(), String> {
        Ok(())
    }
}
