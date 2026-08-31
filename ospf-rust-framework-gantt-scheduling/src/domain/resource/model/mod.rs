//! 资源模型 / Resource models
//!
//! 包含资源容量、资源 trait、资源使用量组件和松弛配置。
//! Contains resource capacity, resource traits, resource usage components, and slack configuration.

pub mod capacity;
pub mod connection_usage;
pub mod resource_trait;
pub mod slack;
pub mod storage_usage;
pub mod usage;

pub use capacity::ResourceCapacity;
pub use connection_usage::ConnectionResourceUsage;
pub use resource_trait::{
    BasicConnectionResource, BasicExecutionResource, BasicStorageResource, ConnectionResourceTrait,
    ExecutionResourceTrait, ResourceTrait, StorageResourceTrait,
};
pub use slack::ResourceSlack;
pub use storage_usage::StorageResourceUsage;
pub use usage::ResourceUsage;
