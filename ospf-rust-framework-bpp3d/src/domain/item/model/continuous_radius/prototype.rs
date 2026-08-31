// ============================================================================
// ContinuousRadiusModelComponent - 连续半径模型组件 / Continuous radius model component
// ============================================================================

use ospf_rust_quantities::dimension::derived_quantity::SameDerivedDimension;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::unit::physical_unit::Unit;

fn compile_time_quantity_to_meters<U>(quantity: Quantity<f64, U>) -> Result<f64, String>
where
    U: ospf_rust_quantities::unit::physical_unit::CTUnit + Default,
    <U as ospf_rust_quantities::unit::physical_unit::CTUnit>::Dimension: SameDerivedDimension<
        <Meter as ospf_rust_quantities::unit::physical_unit::CTUnit>::Dimension,
    >,
{
    let unit = quantity.unit().symbol().to_string();
    if !quantity.value.is_finite() {
        return Err(format!(
            "continuous radius quantity '{}' is non-finite: {}",
            unit, quantity.value,
        ));
    }
    // 编译期量纲检查不能保证 f64 转换不溢出，因此在适配边界使用 checked runtime 路径。
    // Compile-time dimension checks do not guarantee finite f64 conversion, so use the checked runtime path at this adapter boundary.
    let converted = quantity
        .to_runtime()
        .to_unit(&Meter::INSTANT)
        .map_err(|error| {
            format!(
                "continuous radius quantity '{}' cannot be converted to metres: {:?}",
                unit, error,
            )
        })?;
    if !converted.value.is_finite() {
        return Err(format!(
            "continuous radius quantity '{}' overflows when converted to metres",
            unit,
        ));
    }
    Ok(converted.value)
}

fn runtime_quantity_to_meters(quantity: Quantity<f64, Unit>) -> Result<f64, String> {
    let unit = quantity.unit().symbol().to_string();
    if !quantity.value.is_finite() {
        return Err(format!(
            "continuous radius quantity '{}' is non-finite: {}",
            unit, quantity.value,
        ));
    }
    let converted = quantity.to_unit(&Meter::INSTANT).map_err(|error| {
        format!(
            "continuous radius quantity '{}' is incompatible with metres: {:?}",
            unit, error,
        )
    })?;
    if !converted.value.is_finite() {
        return Err(format!(
            "continuous radius quantity '{}' overflows when converted to metres",
            unit,
        ));
    }
    Ok(converted.value)
}

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
    /// 使用带单位的半径边界创建求解器适配副本 / Build a solver adapter from unit-bearing bounds
    ///
    /// 现有 `Option<f64>` 字段仍表示米制求解器边界；该方法把编译时或运行时长度单位
    /// 集中转换到米，并在写入原型前校验有限性、正值和上下界关系。
    /// Existing `Option<f64>` fields remain metre-based solver bounds; this method
    /// centralizes compile-time or runtime unit conversion and validates finite,
    /// positive, ordered bounds before storing them.
    pub fn with_quantity_bounds<Lower, Upper>(
        mut self,
        lower_bound: Quantity<f64, Lower>,
        upper_bound: Quantity<f64, Upper>,
    ) -> Result<Self, String>
    where
        Lower: ospf_rust_quantities::unit::physical_unit::CTUnit + Default,
        <Lower as ospf_rust_quantities::unit::physical_unit::CTUnit>::Dimension:
            SameDerivedDimension<
                <Meter as ospf_rust_quantities::unit::physical_unit::CTUnit>::Dimension,
            >,
        Upper: ospf_rust_quantities::unit::physical_unit::CTUnit + Default,
        <Upper as ospf_rust_quantities::unit::physical_unit::CTUnit>::Dimension:
            SameDerivedDimension<
                <Meter as ospf_rust_quantities::unit::physical_unit::CTUnit>::Dimension,
            >,
    {
        self.radius_lower_bound = Some(compile_time_quantity_to_meters(lower_bound)?);
        self.radius_upper_bound = Some(compile_time_quantity_to_meters(upper_bound)?);
        self.validate_bounds()?;
        Ok(self)
    }

    /// 使用运行时单位半径边界创建求解器适配副本 / Build a solver adapter from runtime-unit bounds
    pub fn with_runtime_quantity_bounds(
        mut self,
        lower_bound: Quantity<f64, Unit>,
        upper_bound: Quantity<f64, Unit>,
    ) -> Result<Self, String> {
        self.radius_lower_bound = Some(runtime_quantity_to_meters(lower_bound)?);
        self.radius_upper_bound = Some(runtime_quantity_to_meters(upper_bound)?);
        self.validate_bounds()?;
        Ok(self)
    }

    /// 使用带单位的半径边界创建求解器适配副本 / Build a solver adapter from unit-bearing bounds
    ///
    /// 这是编译时单位入口的兼容别名；现有 `Option<f64>` 字段仍表示米制边界。
    /// Compatibility alias for the compile-time unit entry point; existing
    /// `Option<f64>` fields remain metre-based bounds.
    pub fn with_radius_bounds<Lower, Upper>(
        self,
        lower_bound: Quantity<f64, Lower>,
        upper_bound: Quantity<f64, Upper>,
    ) -> Result<Self, String>
    where
        Lower: ospf_rust_quantities::unit::physical_unit::CTUnit + Default,
        <Lower as ospf_rust_quantities::unit::physical_unit::CTUnit>::Dimension:
            SameDerivedDimension<
                <Meter as ospf_rust_quantities::unit::physical_unit::CTUnit>::Dimension,
            >,
        Upper: ospf_rust_quantities::unit::physical_unit::CTUnit + Default,
        <Upper as ospf_rust_quantities::unit::physical_unit::CTUnit>::Dimension:
            SameDerivedDimension<
                <Meter as ospf_rust_quantities::unit::physical_unit::CTUnit>::Dimension,
            >,
    {
        self.with_quantity_bounds(lower_bound, upper_bound)
    }

    /// 校验选中的半径平方 / Validate a selected radius-squared value
    pub fn validate_radius_squared(&self, radius_squared: f64) -> Result<f64, String> {
        if !radius_squared.is_finite() {
            return Err(format!(
                "continuous radius variable '{}' has non-finite radius squared {}",
                self.variable_name, radius_squared,
            ));
        }
        if radius_squared < 0.0 {
            return Err(format!(
                "continuous radius variable '{}' has invalid radius squared {}",
                self.variable_name, radius_squared,
            ));
        }
        Ok(radius_squared)
    }

    /// 校验重量函数系数 / Validate radius-squared weight coefficients
    pub fn validate_weight_function(
        &self,
        function: &ContinuousRadiusWeightFunction,
    ) -> Result<(), String> {
        if !function.intercept.is_finite()
            || !function.radius_squared_coefficient.is_finite()
            || !function.objective_weight.is_finite()
        {
            return Err(format!(
                "continuous radius weight function '{}' has non-finite intercept, radius-squared coefficient, or objective weight",
                function.key,
            ));
        }
        Ok(())
    }

    /// 校验求解器半径边界 / Validate solver-facing radius bounds
    ///
    /// 原型当前以求解器边界 `f64` 表达，调用方负责在进入该边界前完成单位归一化。
    /// The prototype currently stores solver-facing bounds as `f64`; callers are
    /// responsible for unit normalization before crossing this boundary.
    pub fn validate_bounds(&self) -> Result<(f64, f64), String> {
        let lower_bound = self.radius_lower_bound.ok_or_else(|| {
            format!(
                "continuous radius variable '{}' requires a finite lower bound",
                self.variable_name,
            )
        })?;
        let upper_bound = self.radius_upper_bound.ok_or_else(|| {
            format!(
                "continuous radius variable '{}' requires a finite upper bound",
                self.variable_name,
            )
        })?;
        if !lower_bound.is_finite()
            || !upper_bound.is_finite()
            || lower_bound <= 0.0
            || upper_bound < lower_bound
            || !lower_bound.powi(2).is_finite()
            || !upper_bound.powi(2).is_finite()
        {
            return Err(format!(
                "continuous radius variable '{}' has invalid bounds {}..{}",
                self.variable_name, lower_bound, upper_bound,
            ));
        }
        Ok((lower_bound, upper_bound))
    }

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
