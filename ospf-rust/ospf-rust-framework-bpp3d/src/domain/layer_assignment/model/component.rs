// ============================================================================
// Bpp3dModelComponent - BPP3D 模型组件 trait / BPP3D model component trait
// ============================================================================

/// BPP3D 模型组件 / BPP3D model component
///
/// 领域建模组件必须通过此 trait 注册到 `MetaModel`。
/// Domain modeling components must register to `MetaModel` through this trait.
pub trait Bpp3dModelComponent: Debug + Send + Sync {
    /// 组件名称 / Component name
    fn name(&self) -> &str;

    /// 注册变量和中间表达式到模型 / Register variables and intermediate expressions to model
    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), String>;
}

