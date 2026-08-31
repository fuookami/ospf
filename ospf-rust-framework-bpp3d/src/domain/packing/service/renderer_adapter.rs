// ============================================================================
// PackingRendererAdapter - 装箱渲染适配器 / Packing renderer adapter
// ============================================================================

/// 装箱渲染适配器 / Packing renderer adapter
///
/// 将装箱结果转换为渲染 DTO，输出 actualVolume。
/// Converts packing results to render DTOs, outputting actualVolume.
#[derive(Debug, Clone, Default)]
pub struct PackingRendererAdapter;

impl PackingRendererAdapter {
    /// 创建渲染适配器 / Create renderer adapter
    pub fn new() -> Self {
        Self
    }

    /// 将装箱结果转换为渲染 DTO / Convert packing result to render DTO
    ///
    /// 所有几何量在 f64 标量域转换，使用 actualVolume。
    /// All geometric quantities are converted in the f64 scalar domain,
    /// using actualVolume for true volume (not bounding cuboid volume).
    pub fn to_render_dto<V, U>(&self, result: &PackingResult<V, U>) -> Vec<RenderLoadingPlanDto>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone,
    {
        result.packed_bins.iter().map(|bin| {
            let items: Vec<crate::infrastructure::renderer::RenderLoadingPlanItemDto> = bin.items.iter().map(|item| {
                let (render_shape_type, render_algo_shape_type, radius, axis) = match item.packing_shape.shape_type {
                    PackingShapeType::Cuboid => (
                        crate::infrastructure::renderer::RenderShapeType::Cuboid,
                        crate::infrastructure::renderer::RenderAlgorithmShapeType::Cuboid,
                        None,
                        None,
                    ),
                    PackingShapeType::Cylinder => {
                        let axis = item.packing_shape.axis.unwrap_or(Axis3::Y);
                        let render_axis = crate::infrastructure::renderer::RenderAxis3::from(axis);
                        // 从 infrastructure::PackingAlgorithmShapeType 获取渲染类型
                        let infra_algo = crate::infrastructure::PackingAlgorithmShapeType::from_cylinder_axis(axis);
                        let algo_type = match infra_algo {
                            crate::infrastructure::PackingAlgorithmShapeType::Cuboid =>
                                crate::infrastructure::renderer::RenderAlgorithmShapeType::Cuboid,
                            crate::infrastructure::PackingAlgorithmShapeType::VerticalCylinder =>
                                crate::infrastructure::renderer::RenderAlgorithmShapeType::VerticalCylinder,
                            crate::infrastructure::PackingAlgorithmShapeType::HorizontalCylinderX =>
                                crate::infrastructure::renderer::RenderAlgorithmShapeType::HorizontalCylinderX,
                            crate::infrastructure::PackingAlgorithmShapeType::HorizontalCylinderZ =>
                                crate::infrastructure::renderer::RenderAlgorithmShapeType::HorizontalCylinderZ,
                        };
                        let r = item.packing_shape.radius.as_ref()
                            .map(|q| -> f64 { q.value.clone().into() });
                        (
                            crate::infrastructure::renderer::RenderShapeType::Cylinder,
                            algo_type,
                            r,
                            Some(render_axis),
                        )
                    }
                };

                crate::infrastructure::renderer::RenderLoadingPlanItemDto {
                    name: item.item.id.clone(),
                    package_type: "default".to_string(),
                    width: item.packing_shape.bounding_width.value.clone().into(),
                    height: item.packing_shape.bounding_height.value.clone().into(),
                    depth: item.packing_shape.bounding_depth.value.clone().into(),
                    x: item.position.x.value.clone().into(),
                    y: item.position.y.value.clone().into(),
                    z: item.position.z.value.clone().into(),
                    weight: item.item.weight.value.clone().into(),
                    loading_order: item.loading_order,
                    shape_type: render_shape_type,
                    algorithm_shape_type: render_algo_shape_type,
                    radius,
                    axis,
                    bounding_width: item.packing_shape.bounding_width.value.clone().into(),
                    bounding_height: item.packing_shape.bounding_height.value.clone().into(),
                    bounding_depth: item.packing_shape.bounding_depth.value.clone().into(),
                    // 使用 actualVolume，不只使用 bounding cuboid volume
                    actual_volume: item.actual_volume().value.clone().into(),
                    info: None,
                }
            }).collect();

            let total_volume: f64 = bin.total_actual_volume().value.clone().into();
            let bin_volume: f64 = bin.bin_type.width.value.clone().into()
                * bin.bin_type.height.value.clone().into()
                * bin.bin_type.depth.value.clone().into();

            crate::infrastructure::renderer::RenderLoadingPlanDto {
                group: "default".to_string(),
                name: bin.name.clone(),
                type_code: bin.bin_type.type_code.clone(),
                width: bin.bin_type.width.value.clone().into(),
                height: bin.bin_type.height.value.clone().into(),
                depth: bin.bin_type.depth.value.clone().into(),
                loading_rate: if bin_volume > 0.0 { total_volume / bin_volume } else { 0.0 },
                weight: bin.total_weight().value.clone().into(),
                volume: total_volume,
                items,
                info: None,
            }
        }).collect()
    }
}

