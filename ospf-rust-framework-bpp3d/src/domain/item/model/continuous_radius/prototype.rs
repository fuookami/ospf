// ============================================================================
// ContinuousRadiusModelComponent - 连续半径模型组件 / Continuous radius model component
// ============================================================================

/// 连续半径求解器原型 / Continuous radius solver prototype
#[derive(Debug, Clone)]
pub struct ContinuousCylinderRadiusSolverPrototype {
    /// 货物标识 / Item id
    pub item_id: ItemId,
    /// 来源 / Source
    pub source: String,
    /// 对齐轴 / Alignment axis
    pub axis: Axis3,
    /// 变量名 / Variable name
    pub variable_name: String,
    /// 半径下界 / Radius lower bound
    pub radius_lower_bound: Option<f64>,
    /// 半径上界 / Radius upper bound
    pub radius_upper_bound: Option<f64>,
}

impl ContinuousCylinderRadiusSolverPrototype {
    /// 生成求解半径结果 / Build selected radius solution
    pub fn selected_solution(
        &self,
        radius: f64,
        radius_squared: Option<f64>,
        segment_index: Option<usize>,
    ) -> ContinuousCylinderRadiusSolution {
        ContinuousCylinderRadiusSolution {
            item_id: self.item_id.clone(),
            source: self.source.clone(),
            variable_name: self.variable_name.clone(),
            axis: self.axis,
            radius,
            radius_squared,
            segment_index,
        }
    }

    /// 校验半径是否落在原型边界内 / Check whether radius is within prototype bounds
    pub fn accepts_radius(&self, radius: f64) -> bool {
        if !radius.is_finite() {
            return false;
        }
        let tolerance = 1e-9;
        if let Some(lower_bound) = self.radius_lower_bound {
            if radius < lower_bound - tolerance {
                return false;
            }
        }
        if let Some(upper_bound) = self.radius_upper_bound {
            if radius > upper_bound + tolerance {
                return false;
            }
        }
        true
    }
}

