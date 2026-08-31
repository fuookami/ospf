//! 资源模型 / Resource models

/// 执行资源占位 / Execution resource placeholder
#[derive(Debug, Clone, Default)]
pub struct ExecutionResource;

/// 存储资源占位 / Storage resource placeholder
#[derive(Debug, Clone, Default)]
pub struct StorageResource;

/// 连接资源占位 / Connection resource placeholder
#[derive(Debug, Clone, Default)]
pub struct ConnectionResource;

/// 资源松弛占位 / Resource slack placeholder
#[derive(Debug, Clone, Default)]
pub struct ResourceSlack;
