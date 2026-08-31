// DepthBoundaryLayerOrientationPolicy - 深度边界策略 / Depth boundary policy
// ============================================================================

/// 深度边界校验阶段 / Depth-boundary validation stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepthBoundaryValidationStage {
    /// 层生成阶段 / Layer generation stage
    Generation,
    /// 最终已知坐标阶段 / Final known-coordinate stage
    FinalKnownCoordinate,
}

/// 深度边界层朝向策略 / Depth boundary layer orientation policy
///
/// 策略只在最终已知坐标阶段生效，生成阶段不提前过滤候选。
/// The policy is enforced only at final known-coordinate validation; generation
/// candidates are not filtered early.
#[derive(Debug, Clone)]
pub struct DepthBoundaryLayerOrientationPolicy {
    /// 是否允许深度边界上的旋转朝向 / Whether rotated orientation is allowed on depth boundary
    pub allow_rotated_on_depth_boundary: bool,
}

impl Default for DepthBoundaryLayerOrientationPolicy {
    fn default() -> Self {
        Self {
            allow_rotated_on_depth_boundary: false,
        }
    }
}

impl DepthBoundaryLayerOrientationPolicy {
    /// 创建默认策略 / Create default policy
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置是否允许旋转 / Set whether rotated orientation is allowed
    pub fn with_allow_rotated_on_depth_boundary(mut self, value: bool) -> Self {
        self.allow_rotated_on_depth_boundary = value;
        self
    }

    /// 校验朝向 / Validate orientation
    pub fn validate(
        &self,
        stage: DepthBoundaryValidationStage,
        orientation: Orientation,
        is_depth_boundary: bool,
    ) -> Result<(), String> {
        if stage == DepthBoundaryValidationStage::Generation {
            return Ok(());
        }
        if !is_depth_boundary || self.allow_rotated_on_depth_boundary || !orientation.is_rotated() {
            return Ok(());
        }

        Err(format!(
            "Rotated orientation {:?} is not allowed on depth boundary. / 深度边界不允许旋转朝向 {:?}。",
            orientation, orientation
        ))
    }
}

// ============================================================================
