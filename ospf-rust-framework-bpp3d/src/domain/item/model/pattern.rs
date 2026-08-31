// ============================================================================
// PatternedItem / PackageAttribute - 模式与包装规则 / Pattern and package rules
// ============================================================================

/// 货物模式键 / Item pattern key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatternedItemKey {
    /// 模式编码 / Pattern code
    pub pattern_code: String,
}

/// 模式投影方向 / Pattern projection orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternProjectionOrientation {
    /// 前向，要求长度不小于宽度 / Front, requiring length no smaller than width
    Front,
    /// 侧向，要求长度不大于宽度 / Side, requiring length no greater than width
    Side,
}

/// 模式下一点策略 / Pattern next-point policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternNextPointPolicy {
    /// 右下角追加 / Append at right-bottom
    RightBottom,
    /// 左上角追加 / Append at left-upper
    LeftUpper,
}

/// 模式步骤 / Pattern step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatternStep {
    /// 长度投影方向 / Length projection orientation
    pub length_orientation: PatternProjectionOrientation,
    /// 下一点策略 / Next-point policy
    pub next_point_policy: Option<PatternNextPointPolicy>,
}

impl PatternStep {
    /// 创建模式步骤 / Create pattern step
    pub fn new(
        length_orientation: PatternProjectionOrientation,
        next_point_policy: Option<PatternNextPointPolicy>,
    ) -> Self {
        Self {
            length_orientation,
            next_point_policy,
        }
    }
}

/// 底面尺寸范围 / Bottom dimension range
///
/// 对应 Kotlin `Pattern.bottomLengthRange` / `bottomWidthRange`，用于按货物底面尺寸
/// 过滤模式候选。
/// Corresponds to Kotlin `Pattern.bottomLengthRange` / `bottomWidthRange`,
/// filtering pattern candidates by item bottom-face dimensions.
///
/// Kotlin 中 `Bottom.length(view)` 为底面投影长度（默认朝向下的 depth），
/// `Bottom.width(view)` 为底面投影宽度（默认朝向下的 width）。
/// In Kotlin, `Bottom.length(view)` is the bottom projection length (depth under
/// default orientation), `Bottom.width(view)` is the bottom projection width.
#[derive(Debug, Clone, PartialEq)]
pub struct BottomDimensionRange {
    /// 下界（含）/ Lower bound (inclusive)
    pub min: Option<f64>,
    /// 上界（含）/ Upper bound (inclusive)
    pub max: Option<f64>,
}

impl Default for BottomDimensionRange {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
        }
    }
}

impl BottomDimensionRange {
    /// 创建无约束范围 / Create unbounded range
    pub fn unbounded() -> Self {
        Self::default()
    }

    /// 创建有界范围 / Create bounded range
    pub fn new(min: Option<f64>, max: Option<f64>) -> Self {
        Self { min, max }
    }

    /// 创建仅下界范围 / Create lower-bounded range
    pub fn at_least(min: f64) -> Self {
        Self {
            min: Some(min),
            max: None,
        }
    }

    /// 创建仅上界范围 / Create upper-bounded range
    pub fn at_most(max: f64) -> Self {
        Self {
            min: None,
            max: Some(max),
        }
    }

    /// 创建闭区间范围 / Create closed-interval range
    pub fn between(min: f64, max: f64) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
        }
    }

    /// 判断值是否在范围内 / Check whether value is within range
    pub fn contains(&self, value: f64) -> bool {
        if let Some(min) = self.min {
            if value < min {
                return false;
            }
        }
        if let Some(max) = self.max {
            if value > max {
                return false;
            }
        }
        true
    }

    /// 是否为无约束 / Whether range is unbounded
    pub fn is_unbounded(&self) -> bool {
        self.min.is_none() && self.max.is_none()
    }

    /// 诊断信息 / Diagnostics
    pub fn diagnostics(&self, label: &str) -> String {
        format!(
            "{}: [{}, {}]",
            label,
            self.min.map(|v| format!("{v}")).unwrap_or_else(|| "-inf".to_string()),
            self.max.map(|v| format!("{v}")).unwrap_or_else(|| "+inf".to_string()),
        )
    }
}

/// 模式配置 / Pattern config
#[derive(Debug, Clone, PartialEq)]
pub struct PatternConfig {
    /// 允许堆叠层数 / Allowed piling layer count
    pub with_piling: Option<u64>,
    /// 是否允许余项模式 / Whether remainder patterns are allowed
    pub with_remainder: bool,
    /// 模式列表 / Pattern list
    pub patterns: Vec<Vec<PatternStep>>,
    /// 底面长度范围（底面较长维度）/ Bottom length range (longer bottom dimension)
    pub bottom_length_range: BottomDimensionRange,
    /// 底面宽度范围（底面较短维度）/ Bottom width range (shorter bottom dimension)
    pub bottom_width_range: BottomDimensionRange,
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            with_piling: None,
            with_remainder: false,
            patterns: PatternDefinition::default_patterns(),
            bottom_length_range: BottomDimensionRange::default(),
            bottom_width_range: BottomDimensionRange::default(),
        }
    }
}

impl PatternConfig {
    /// 创建配置 / Create config
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置堆叠层数 / Set piling layer count
    pub fn with_piling(mut self, value: u64) -> Self {
        self.with_piling = Some(value);
        self
    }

    /// 设置是否允许余项 / Set whether remainder is allowed
    pub fn with_remainder(mut self, value: bool) -> Self {
        self.with_remainder = value;
        self
    }

    /// 设置模式列表 / Set pattern list
    pub fn with_patterns(mut self, patterns: Vec<Vec<PatternStep>>) -> Self {
        self.patterns = patterns;
        self
    }

    /// 设置底面长度范围 / Set bottom length range
    pub fn with_bottom_length_range(mut self, range: BottomDimensionRange) -> Self {
        self.bottom_length_range = range;
        self
    }

    /// 设置底面宽度范围 / Set bottom width range
    pub fn with_bottom_width_range(mut self, range: BottomDimensionRange) -> Self {
        self.bottom_width_range = range;
        self
    }

    /// 判断底面尺寸是否在范围内 / Check whether bottom dimensions are within range
    ///
    /// 对应 Kotlin `Bottom.length(view)` / `Bottom.width(view)` 语义：
    /// `bottom_length = max(depth, width)`, `bottom_width = min(depth, width)`。
    /// Corresponds to Kotlin `Bottom.length(view)` / `Bottom.width(view)` semantics:
    /// `bottom_length = max(depth, width)`, `bottom_width = min(depth, width)`.
    pub fn accepts_bottom_dimensions(&self, depth: f64, width: f64) -> bool {
        let bottom_length = depth.max(width);
        let bottom_width = depth.min(width);
        self.bottom_length_range.contains(bottom_length)
            && self.bottom_width_range.contains(bottom_width)
    }

    /// 是否允许两层混合堆 / Whether two-layer mixed pile is enabled
    pub fn enables_two_sum(&self) -> bool {
        self.with_piling.map(|value| value >= 2).unwrap_or(true)
    }

    /// 是否允许三层混合堆 / Whether three-layer mixed pile is enabled
    pub fn enables_three_sum(&self) -> bool {
        self.with_piling.map(|value| value >= 3).unwrap_or(true)
    }

    /// 单物料最大堆叠层数 / Maximum single-material pile layer count
    pub fn single_pile_limit(&self) -> Option<u64> {
        self.with_piling
    }

    /// 有效模式列表 / Effective pattern list
    pub fn effective_patterns(&self) -> Vec<Vec<PatternStep>> {
        if self.patterns.is_empty() {
            PatternDefinition::default_patterns()
        } else {
            self.patterns.clone()
        }
    }
}

/// 模式定义 / Pattern definition
#[derive(Debug, Clone, PartialEq)]
pub struct PatternDefinition {
    /// 模式列表 / Pattern list
    pub patterns: Vec<Vec<PatternStep>>,
    /// 配置 / Config
    pub config: PatternConfig,
}

impl PatternDefinition {
    /// 默认模式列表 / Default pattern list
    pub fn default_patterns() -> Vec<Vec<PatternStep>> {
        vec![vec![
            PatternStep::new(PatternProjectionOrientation::Front, None),
            PatternStep::new(
                PatternProjectionOrientation::Front,
                Some(PatternNextPointPolicy::RightBottom),
            ),
            PatternStep::new(
                PatternProjectionOrientation::Front,
                Some(PatternNextPointPolicy::LeftUpper),
            ),
        ]]
    }

    /// 创建模式定义 / Create pattern definition
    pub fn new(patterns: Vec<Vec<PatternStep>>, config: PatternConfig) -> Self {
        Self { patterns, config }
    }

    /// 是否为有效模式 / Whether definition is valid
    pub fn is_valid(&self) -> bool {
        !self.patterns.is_empty() && self.patterns.iter().all(|pattern| !pattern.is_empty())
    }

    /// 诊断信息 / Diagnostics
    pub fn diagnostics(&self) -> Vec<String> {
        vec![format!(
            "pattern definition: patterns={}, with_piling={:?}, with_remainder={}, two_sum={}, three_sum={}, {}, {}",
            self.patterns.len(),
            self.config.with_piling,
            self.config.with_remainder,
            self.config.enables_two_sum(),
            self.config.enables_three_sum(),
            self.config.bottom_length_range.diagnostics("bottom_length"),
            self.config.bottom_width_range.diagnostics("bottom_width"),
        )]
    }
}

/// 模式货物 / Patterned item
#[derive(Debug, Clone)]
pub struct PatternedItem<V, U: UnitTrait> {
    /// 模式键 / Pattern key
    pub key: PatternedItemKey,
    /// 货物 / Item
    pub item: ActualItem<V, U>,
    /// 数量 / Amount
    pub amount: u64,
    /// 数量区间 / Amount range
    pub amount_range: IntervalValue<u64>,
    /// 物料数量 / Material amounts
    pub material_amounts: Vec<(MaterialKey, u64)>,
    /// 物料重量 / Material weights
    pub material_weights: Vec<(MaterialKey, Quantity<V, U>)>,
    /// 实际货物聚合项 / Aggregated actual items
    pub actual_items: Vec<(ActualItem<V, U>, u64, IntervalValue<u64>)>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

impl<V, U> PatternedItem<V, U>
where
    V: Clone + Field + num_traits::Float + num_traits::FloatConst,
    U: UnitTrait,
{
    /// 创建模式货物 / Create patterned item
    pub fn new(
        pattern_code: impl Into<String>,
        item: ActualItem<V, U>,
    ) -> Self {
        let pattern_code = pattern_code.into();
        let mut material_amounts = item
            .pack
            .as_ref()
            .map(|pack| pack.materials.clone())
            .unwrap_or_default();
        material_amounts.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
        Self {
            key: PatternedItemKey {
                pattern_code: pattern_code.clone(),
            },
            item,
            amount: 1,
            amount_range: IntervalValue::new(1, 1),
            material_amounts,
            material_weights: Vec::new(),
            actual_items: Vec::new(),
            diagnostics: vec![format!(
                "patterned item '{}' uses conservative item-level demand coverage",
                pattern_code,
            )],
        }
    }

    /// 创建兼容入口 / Create compatibility entry
    pub fn unsupported(
        pattern_code: impl Into<String>,
        item: ActualItem<V, U>,
    ) -> Self {
        Self::new(pattern_code, item)
    }

    /// 创建需求覆盖 / Create demand coverage
    pub fn demand_coverage(&self, coefficient: f64) -> Bpp3dLayerDemandCoverage {
        Bpp3dLayerDemandCoverage::new(
            Bpp3dDemandMode::Item,
            Bpp3dDemandKey::Item {
                id: self.item.id.clone(),
            },
            coefficient,
        )
    }

    /// 按聚合索引查询实际货物 / Query actual item by aggregated index
    pub fn actual_item_at(&self, index: usize) -> Option<&ActualItem<V, U>> {
        let mut remaining = index;
        for (item, amount, _) in &self.actual_items {
            let amount = (*amount).try_into().ok()?;
            if remaining < amount {
                return Some(item);
            }
            remaining -= amount;
        }
        None
    }

    /// 从实际货物聚合创建模式货物 / Create patterned item from actual items
    pub fn from_actual_items(
        pattern_code: impl Into<String>,
        pattern_shape: &PackageShape<V, U>,
        package_attribute: &PackageAttribute,
        actual_items: &[(ActualItem<V, U>, u64)],
    ) -> Option<(Self, u64)>
    where
        U: CTUnit + Default + Clone,
        V: ToPrimitive,
    {
        let actual_items_with_ranges = actual_items
            .iter()
            .map(|(item, amount)| (item.clone(), *amount, IntervalValue::new(*amount, *amount)))
            .collect::<Vec<_>>();
        Self::from_actual_items_with_ranges(
            pattern_code,
            pattern_shape,
            package_attribute,
            &actual_items_with_ranges,
        )
        .map(|(patterned, amount, _)| (patterned, amount))
    }

    /// 从带数量区间的实际货物聚合创建模式货物 / Create patterned item from actual items with amount ranges
    pub fn from_actual_items_with_ranges(
        pattern_code: impl Into<String>,
        pattern_shape: &PackageShape<V, U>,
        package_attribute: &PackageAttribute,
        actual_items: &[(ActualItem<V, U>, u64, IntervalValue<u64>)],
    ) -> Option<(Self, u64, IntervalValue<u64>)>
    where
        U: CTUnit + Default + Clone,
        V: ToPrimitive,
    {
        Self::from_actual_items_with_ranges_and_material_catalog(
            pattern_code,
            pattern_shape,
            package_attribute,
            actual_items,
            &[],
        )
    }

    /// 从带数量区间和物料目录的实际货物聚合创建模式货物 / Create patterned item from actual items with amount ranges and material catalog
    pub fn from_actual_items_with_ranges_and_material_catalog(
        pattern_code: impl Into<String>,
        pattern_shape: &PackageShape<V, U>,
        package_attribute: &PackageAttribute,
        actual_items: &[(ActualItem<V, U>, u64, IntervalValue<u64>)],
        material_catalog: &[Material<V, U>],
    ) -> Option<(Self, u64, IntervalValue<u64>)>
    where
        U: CTUnit + Default + Clone,
        V: ToPrimitive,
    {
        Self::from_actual_items_with_ranges_catalog_and_orientations(
            pattern_code,
            pattern_shape,
            package_attribute,
            actual_items,
            material_catalog,
            &[],
        )
    }

    /// 从带数量区间、物料目录和模式朝向的实际货物聚合创建模式货物 / Create patterned item from actual items with amount ranges, material catalog, and pattern orientations
    pub fn from_actual_items_with_ranges_catalog_and_orientations(
        pattern_code: impl Into<String>,
        pattern_shape: &PackageShape<V, U>,
        package_attribute: &PackageAttribute,
        actual_items: &[(ActualItem<V, U>, u64, IntervalValue<u64>)],
        material_catalog: &[Material<V, U>],
        pattern_orientations: &[Orientation],
    ) -> Option<(Self, u64, IntervalValue<u64>)>
    where
        U: CTUnit + Default + Clone,
        V: ToPrimitive,
    {
        let pattern_code = pattern_code.into();
        let Some((first_item, _, _)) = actual_items.first() else {
            return None;
        };
        let amount = actual_items
            .iter()
            .fold(0_u64, |acc, (_, amount, _)| acc.saturating_add(*amount));
        if amount == 0 {
            return None;
        }
        let mut range_lower = 0_u64;
        let mut range_upper = 0_u64;
        for (_, _, amount_range) in actual_items {
            range_lower = range_lower.saturating_add(*amount_range.lower_bound().value().unwrap()?);
            range_upper = range_upper.saturating_add(*amount_range.upper_bound().value().unwrap()?);
        }
        let amount_range = IntervalValue::new(range_lower, range_upper);
        let mut material_amounts = HashMap::<MaterialKey, u64>::new();
        let mut material_weights = HashMap::<MaterialKey, V>::new();
        for (item, item_amount, _) in actual_items {
            if let Some(pack) = &item.pack {
                for (material_key, material_amount) in &pack.materials {
                    let entry = material_amounts.entry(material_key.clone()).or_insert(0);
                    *entry = entry.saturating_add(material_amount.saturating_mul(*item_amount));
                }
            }
            for (material_key, weight) in item.material_weights(material_catalog) {
                let entry = material_weights.entry(material_key).or_insert_with(V::zero);
                *entry = *entry
                    + weight.value * V::from(*item_amount).unwrap_or_else(V::zero);
            }
        }
        let mut material_amounts = material_amounts.into_iter().collect::<Vec<_>>();
        material_amounts.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
        let mut material_weights = material_weights
            .into_iter()
            .map(|(material_key, weight)| (material_key, Quantity::new_ct(weight)))
            .collect::<Vec<_>>();
        material_weights.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
        let total_weight = actual_items.iter().fold(V::zero(), |acc, (item, amount, _)| {
            acc + item.weight.value * V::from(*amount).unwrap_or_else(V::zero)
        });
        let total_volume = actual_items.iter().fold(V::zero(), |acc, (item, amount, _)| {
            acc + item.width.value * item.height.value * item.depth.value
                * V::from(*amount).unwrap_or_else(V::zero)
        });
        let average_volume = total_volume / V::from(amount).unwrap_or_else(V::one);
        let deformation = package_attribute
            .deformation_attribute
            .deformation_quantity(average_volume.to_f64().unwrap_or(0.0));
        let pack = first_item.pack.clone().map(|mut pack| {
            pack.shape = pattern_shape.clone();
            pack.materials = material_amounts.clone();
            pack.amount = amount;
            pack
        });
        let item = ActualItem {
            id: first_item.id.clone(),
            name: first_item.name.clone(),
            package_code: first_item.package_code.clone(),
            pack,
            width: Quantity::new_ct(
                pattern_shape.width.value + V::from(deformation[0]).unwrap_or_else(V::zero),
            ),
            height: Quantity::new_ct(
                pattern_shape.height.value + V::from(deformation[1]).unwrap_or_else(V::zero),
            ),
            depth: Quantity::new_ct(
                pattern_shape.depth.value + V::from(deformation[2]).unwrap_or_else(V::zero),
            ),
            weight: Quantity::new_ct(total_weight / V::from(amount).unwrap_or_else(V::one)),
            enabled_orientations: merge_orientations(
                &first_item.enabled_orientations,
                pattern_orientations,
            ),
            shape_spec_override: Some(pattern_shape.spec.clone()),
        };
        let mut patterned = Self::new(pattern_code, item);
        patterned.amount = amount;
        patterned.amount_range = amount_range.clone();
        patterned.material_amounts = material_amounts;
        patterned.material_weights = material_weights;
        patterned.actual_items = actual_items.to_vec();
        patterned.diagnostics.push(format!(
            "patterned item '{}' aggregates {} actual items with deformation",
            patterned.key.pattern_code,
            amount,
        ));
        Some((patterned, amount, amount_range))
    }
}

fn merge_orientations(base: &[Orientation], extra: &[Orientation]) -> Vec<Orientation> {
    let mut orientations = if base.is_empty() {
        vec![Orientation::Upright]
    } else {
        base.to_vec()
    };
    for orientation in extra {
        if !orientations.contains(orientation) {
            orientations.push(*orientation);
        }
    }
    orientations
}

