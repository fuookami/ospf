//! Framework 异步工具入口（Kotlin 对齐）
//! Framework async helpers (Kotlin-aligned)

#[cfg(feature = "async")]
pub type FrameworkJoinHandle<T> = tokio::task::JoinHandle<T>;

#[cfg(feature = "async")]
pub fn spawn_framework_task<F, T>(future: F) -> FrameworkJoinHandle<T>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    tokio::spawn(future)
}
