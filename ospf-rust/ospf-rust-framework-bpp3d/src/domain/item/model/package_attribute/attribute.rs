/// 包装属性 / Package attribute
#[derive(Debug, Clone, PartialEq)]
pub struct PackageAttribute {
    /// 包装类型 / Package type
    pub package_type: PackageType,
    /// 包装最大层数 / Package maximum layer count
    pub package_max_layer: Option<u64>,
    /// 最大堆叠高度 / Maximum stacking height
    pub max_height: Option<f64>,
    /// 最小深度 / Minimum depth
    pub min_depth: f64,
    /// 最大深度 / Maximum depth
    pub max_depth: Option<f64>,
    /// 可承载的上方包装类型 / Package types allowed above
    pub over_package_types: Vec<PackageType>,
    /// 是否只能在底部 / Whether bottom-only
    pub bottom_only: bool,
    /// 顶面是否平整 / Whether top surface is flat
    pub top_flat: bool,
    /// 侧放可放置的最高层 / Highest layer allowing side orientation on top
    pub side_on_top_layer: u64,
    /// 平放可放置的最高层 / Highest layer allowing lie orientation on top
    pub lie_on_top_layer: u64,
    /// 货物属性键 / Cargo attribute key
    ///
    /// 对应 Kotlin `PackageAttribute.cargoAttribute: AbstractCargoAttribute?`。
    /// 作为被动元数据标签用于身份比较，不影响堆叠/朝向/放置逻辑。
    /// Corresponds to Kotlin `PackageAttribute.cargoAttribute`. Serves as a
    /// passive metadata tag for identity comparison; does not influence
    /// stacking, orientation, or placement logic within the framework.
    pub cargo_attribute: Option<CargoAttributeKey>,
    /// 重量属性 / Weight attribute
    pub weight_attribute: WeightAttribute,
    /// 变形属性 / Deformation attribute
    pub deformation_attribute: DeformationAttribute,
    /// 悬空策略 / Hanging policy
    pub hanging_policy: HangingPolicy,
    /// 堆叠策略 / Stacking-on policy
    pub stacking_on_policy: StackingOnPolicy,
    /// 是否允许混装 / Whether mixed loading is allowed
    pub allow_mixed_loading: Option<bool>,
    /// 最大堆叠层数 / Maximum stacking layers
    pub max_stack_layers: Option<u64>,
    /// 业务标签 / Business tags
    pub tags: Vec<String>,
    /// 货物属性标签 / Cargo attribute tags
    pub cargo_tags: Vec<String>,
    /// 额外朝向规则 / Extra orientation rules
    pub extra_orientation_rules: Vec<PackageOrientationRule>,
    /// 额外两两堆叠规则 / Extra pair stacking rules
    pub extra_pair_stacking_rules: Vec<PackagePairStackingRule>,
    /// 额外放置级堆叠规则 / Extra placement stacking rules
    pub extra_placement_stacking_rules: Vec<PackagePlacementStackingRule>,
}

impl Default for PackageAttribute {
    fn default() -> Self {
        Self {
            package_type: PackageType::default(),
            package_max_layer: None,
            max_height: None,
            min_depth: 0.0,
            max_depth: None,
            over_package_types: PackageType::ALL.to_vec(),
            bottom_only: false,
            top_flat: true,
            side_on_top_layer: 0,
            lie_on_top_layer: 0,
            cargo_attribute: None,
            weight_attribute: WeightAttribute::default(),
            deformation_attribute: DeformationAttribute::default(),
            hanging_policy: HangingPolicy::default(),
            stacking_on_policy: StackingOnPolicy::default(),
            allow_mixed_loading: None,
            max_stack_layers: None,
            tags: Vec::new(),
            cargo_tags: Vec::new(),
            extra_orientation_rules: Vec::new(),
            extra_pair_stacking_rules: Vec::new(),
            extra_placement_stacking_rules: Vec::new(),
        }
    }
}

impl PackageAttribute {
    /// 包装类别 / Package category
    pub fn package_category(&self) -> PackageCategory {
        self.package_type.category()
    }

    /// 是否启用侧放上层限制 / Whether side-on-top limit is enabled
    pub fn enabled_side_on_top(&self) -> bool {
        self.side_on_top_layer != 0
    }

    /// 是否启用平放上层限制 / Whether lie-on-top limit is enabled
    pub fn enabled_lie_on_top(&self) -> bool {
        self.lie_on_top_layer != 0
    }

    /// 最大层数 / Maximum layer count
    pub fn max_layer(&self) -> Option<u64> {
        match (self.package_max_layer, self.weight_attribute.max_layer) {
            (Some(package), Some(weight)) => Some(package.min(weight)),
            (Some(package), None) => Some(package),
            (None, Some(weight)) => Some(weight),
            (None, None) => self.max_stack_layers,
        }
    }

    /// 朝向相关最大层数 / Orientation-aware maximum layer count
    pub fn max_layer_for_orientation(
        &self,
        orientation: Orientation,
        orientation_enabled: bool,
    ) -> Option<u64> {
        if self.oriented_top_flat(orientation, orientation_enabled) {
            return self.max_layer();
        }
        match orientation.category() {
            OrientationCategory::Side if self.enabled_side_on_top() => Some(self.side_on_top_layer),
            OrientationCategory::Lie if self.enabled_lie_on_top() => Some(self.lie_on_top_layer),
            OrientationCategory::Upright | OrientationCategory::Side | OrientationCategory::Lie => Some(1),
        }
    }

    /// 判断深度是否满足包装边界 / Check whether depth is within package bounds
    pub fn enabled_depth(&self, depth: f64) -> bool {
        if depth < self.min_depth {
            return false;
        }
        self.max_depth.map(|max_depth| depth <= max_depth).unwrap_or(true)
    }

    fn oriented_top_flat(
        &self,
        orientation: Orientation,
        orientation_enabled: bool,
    ) -> bool {
        if orientation.category() == OrientationCategory::Upright {
            self.top_flat
        } else {
            orientation_enabled
        }
    }

    /// 判断朝向层限制 / Check orientation layer limit
    pub fn enabled_orientation_on_layer(
        &self,
        orientation: Orientation,
        layer: u64,
        orientation_enabled: bool,
        orientation_enabled_at_space: bool,
        space_width: f64,
        space_height: f64,
        space_depth: f64,
    ) -> bool {
        if !orientation_enabled_at_space {
            return false;
        }
        let input = PackageOrientationRuleInput {
            orientation,
            space_width,
            space_height,
            space_depth,
        };
        if !self.enabled_orientation_by_rule(&input) {
            return false;
        }
        if orientation_enabled {
            return true;
        }
        match orientation.category() {
            OrientationCategory::Side => layer < self.side_on_top_layer,
            OrientationCategory::Lie => layer < self.lie_on_top_layer,
            OrientationCategory::Upright => true,
        }
    }

    /// 判断朝向是否满足额外规则 / Check whether orientation satisfies extra rules
    pub fn enabled_orientation_by_rule(&self, input: &PackageOrientationRuleInput) -> bool {
        self.extra_orientation_rules
            .iter()
            .all(|rule| rule.allows(input))
    }

    /// 判断底部支撑是否允许堆叠 / Check whether bottom support allows stacking
    pub fn enabled_stacking_on_support(
        &self,
        item_weight: f64,
        footprint_area: f64,
        footprint_min_span: f64,
        bottom_support_area: f64,
        bottom_support_weight: f64,
    ) -> bool {
        self.hanging_policy.enabled_stacking_on_support(
            item_weight,
            footprint_area,
            footprint_min_span,
            bottom_support_area,
            bottom_support_weight,
        )
    }

    /// 判断包装是否可堆叠到底部包装上 / Check whether package can stack on bottom package
    pub fn enabled_stacking_on(&self, input: &PackageStackingInput<'_>) -> bool {
        if let Some(bottom_item) = input.bottom_item {
            if self.bottom_only && !bottom_item.bottom_only {
                return false;
            }
            if (!bottom_item.oriented_top_flat(
                input.bottom_orientation,
                input.bottom_orientation_enabled,
            ) || !input.bottom_orientation_enabled)
                && self.package_category() != PackageCategory::Filler
            {
                return false;
            }
        }
        if !self.enabled_orientation_on_layer(
            input.item_orientation,
            input.layer,
            input.item_orientation_enabled,
            input.item_orientation_enabled_at_space,
            input.space_width,
            input.space_height,
            input.space_depth,
        ) {
            return false;
        }
        if !self
            .extra_pair_stacking_rules
            .iter()
            .all(|rule| rule.allows(input))
        {
            return false;
        }
        let Some(bottom_item) = input.bottom_item else {
            return true;
        };
        match self.stacking_on_policy {
            StackingOnPolicy::Box {
                max_difference,
                max_over_weight,
            } => self.enabled_box_stacking_on(input, bottom_item, max_difference, max_over_weight),
            StackingOnPolicy::CartonContainer {
                max_difference,
                max_over_weight,
            } => self.enabled_carton_container_stacking_on(
                input,
                bottom_item,
                max_difference,
                max_over_weight,
            ),
            StackingOnPolicy::Filter { max_over_weight } => {
                self.enabled_filter_stacking_on(input, bottom_item, max_over_weight)
            }
        }
    }

    fn enabled_box_stacking_on(
        &self,
        input: &PackageStackingInput<'_>,
        bottom_item: &PackageAttribute,
        max_difference: f64,
        max_over_weight: f64,
    ) -> bool {
        if !bottom_item.over_package_types.contains(&self.package_type) {
            return false;
        }
        if bottom_item.package_category() == PackageCategory::Pallet
            && self.package_category() != PackageCategory::Filler
            && !bottom_item.top_flat
        {
            return false;
        }
        if bottom_item.package_category() != PackageCategory::Filler
            && self.package_category() != PackageCategory::Filler
        {
            let difference =
                (input.item_width - input.bottom_width).abs()
                    + (input.item_depth - input.bottom_depth).abs();
            if difference > max_difference {
                return false;
            }
        }
        self.enabled_common_stacking_limits(input, max_over_weight)
    }

    fn enabled_carton_container_stacking_on(
        &self,
        input: &PackageStackingInput<'_>,
        bottom_item: &PackageAttribute,
        max_difference: f64,
        max_over_weight: f64,
    ) -> bool {
        if !bottom_item.over_package_types.contains(&self.package_type) {
            return false;
        }
        if bottom_item.package_category() == PackageCategory::SoftBox
            && self.package_category() == PackageCategory::SoftBox
        {
            let difference =
                input.item_width + input.item_depth - input.bottom_width - input.bottom_depth;
            if difference > max_difference {
                return false;
            }
        }
        self.enabled_common_stacking_limits(input, max_over_weight)
    }

    fn enabled_filter_stacking_on(
        &self,
        input: &PackageStackingInput<'_>,
        bottom_item: &PackageAttribute,
        max_over_weight: f64,
    ) -> bool {
        if !bottom_item.over_package_types.contains(&self.package_type) {
            return false;
        }
        self.enabled_common_stacking_limits(input, max_over_weight)
    }

    fn enabled_common_stacking_limits(
        &self,
        input: &PackageStackingInput<'_>,
        max_over_weight: f64,
    ) -> bool {
        if input.item_weight - input.bottom_weight > max_over_weight {
            return false;
        }
        if let Some(max_layer) = self.max_layer() {
            if input.layer >= max_layer {
                return false;
            }
        }
        if let Some(max_height) = self.max_height {
            if input.height + input.item_height > max_height {
                return false;
            }
        }
        true
    }

    /// 判断放置级堆叠规则 / Check placement-level stacking rules
    pub fn enabled_placement_stacking(
        &self,
        input: &PackagePlacementStackingInput<'_>,
    ) -> bool {
        if self.bottom_only
            && input
                .direct_bottom_items
                .iter()
                .chain(input.indirect_bottom_items.iter())
                .any(|item| !item.bottom_only)
        {
            return false;
        }
        if !input.item_orientation_enabled_at_space {
            return false;
        }
        if self.package_category() != PackageCategory::Filler {
            let direct_bottom_contexts = if input.direct_bottom_contexts.is_empty() {
                input
                    .direct_bottom_items
                    .iter()
                    .copied()
                    .map(PackagePlacementBottomContext::from_attribute)
                    .collect::<Vec<_>>()
            } else {
                input.direct_bottom_contexts.clone()
            };
            for bottom_item in direct_bottom_contexts {
                if !bottom_item.oriented_top_flat() {
                    if !bottom_item.orientation_enabled()
                        && !input.item_orientation_enabled
                    {
                        match input.item_orientation.category() {
                            OrientationCategory::Side if input.layer >= self.side_on_top_layer => {
                                return false;
                            }
                            OrientationCategory::Lie if input.layer >= self.lie_on_top_layer => {
                                return false;
                            }
                            _ => {}
                        }
                    } else {
                        return false;
                    }
                }
            }
        }
        self.extra_placement_stacking_rules
            .iter()
            .all(|rule| rule.allows(input))
    }

    /// 验证业务规则 / Validate business rules
    pub fn validate(&self) -> Vec<String> {
        let mut diagnostics = Vec::new();
        if self.allow_mixed_loading == Some(false) {
            diagnostics.push("package attribute disallows mixed loading".to_string());
        }
        if self.max_stack_layers == Some(0) {
            diagnostics.push("package attribute max_stack_layers must be positive".to_string());
        }
        if self.package_max_layer == Some(0) {
            diagnostics.push("package attribute package_max_layer must be positive".to_string());
        }
        if self.weight_attribute.max_layer == Some(0) {
            diagnostics.push("package attribute weight max_layer must be positive".to_string());
        }
        if self.min_depth < 0.0 {
            diagnostics.push("package attribute min_depth must be non-negative".to_string());
        }
        if let Some(max_depth) = self.max_depth {
            if max_depth < self.min_depth {
                diagnostics.push("package attribute max_depth must be >= min_depth".to_string());
            }
        }
        if let Some(max_height) = self.max_height {
            if max_height <= 0.0 {
                diagnostics.push("package attribute max_height must be positive".to_string());
            }
        }
        if self.over_package_types.is_empty() {
            diagnostics.push("package attribute over_package_types must not be empty".to_string());
        }
        if self.cargo_tags.iter().any(|tag| tag.trim().is_empty()) {
            diagnostics.push("package attribute cargo_tags must not contain empty tag".to_string());
        }
        if self.tags.iter().any(|tag| tag.trim().is_empty()) {
            diagnostics.push("package attribute tags must not contain empty tag".to_string());
        }
        if let HangingPolicy::Relative {
            hanging_percentage, ..
        } = self.hanging_policy
        {
            if !(0.0..=1.0).contains(&hanging_percentage) {
                diagnostics.push(
                    "package attribute relative hanging percentage must be in [0, 1]".to_string(),
                );
            }
        }
        diagnostics.extend(self.diagnostics());
        diagnostics
    }

    /// 诊断信息 / Diagnostics
    pub fn diagnostics(&self) -> Vec<String> {
        vec![format!(
            "package attribute rule: type={:?}, category={:?}, mixed_loading={:?}, max_layer={:?}, max_stack_layers={:?}, tags={}",
            self.package_type,
            self.package_category(),
            self.allow_mixed_loading,
            self.max_layer(),
            self.max_stack_layers,
            self.tags.len(),
        )]
    }

    /// 装箱诊断信息 / Packing diagnostics
    pub fn packing_diagnostics(&self, layer_count: u64) -> Vec<String> {
        let mut diagnostics = self.diagnostics();
        if let Some(max_stack_layers) = self.max_stack_layers {
            if layer_count > max_stack_layers {
                diagnostics.push(format!(
                    "package attribute max_stack_layers exceeded: {} > {}",
                    layer_count,
                    max_stack_layers,
                ));
            }
        }
        if self.allow_mixed_loading == Some(false) {
            diagnostics.push("package attribute disallows mixed loading".to_string());
        }
        if let Some(max_layer) = self.max_layer() {
            if layer_count > max_layer {
                diagnostics.push(format!(
                    "package attribute max_layer exceeded: {} > {}",
                    layer_count,
                    max_layer,
                ));
            }
        }
        diagnostics
    }
}

impl<V: Clone + Field + num_traits::FloatConst, U: CTUnit + Default + Clone> ActualItem<V, U> {
    /// 物料数量 / Material amounts
    pub fn material_amounts(&self) -> Vec<(MaterialKey, u64)> {
        let mut material_amounts = HashMap::<MaterialKey, u64>::new();
        if let Some(pack) = &self.pack {
            for (material_key, amount) in &pack.materials {
                let entry = material_amounts.entry(material_key.clone()).or_insert(0);
                *entry = entry.saturating_add(*amount);
            }
        }
        let mut material_amounts = material_amounts.into_iter().collect::<Vec<_>>();
        material_amounts.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
        material_amounts
    }

    /// 获取包装形状 / Get package shape
    pub fn package_shape(&self) -> PackageShape<V, U> {
        if let Some(spec) = &self.shape_spec_override {
            PackageShape {
                width: self.width.clone(),
                height: self.height.clone(),
                depth: self.depth.clone(),
                weight: self.weight.clone(),
                spec: spec.clone(),
            }
        } else {
            // 默认为长方体
            PackageShape {
                width: self.width.clone(),
                height: self.height.clone(),
                depth: self.depth.clone(),
                weight: self.weight.clone(),
                spec: PackageShapeSpec::Cuboid,
            }
        }
    }

    /// 获取包装形状3D / Get 3D packing shape
    pub fn packing_shape(&self) -> PackingShape3<V, U> {
        self.package_shape().to_packing_shape()
    }
}

impl<V, U> ActualItem<V, U>
where
    V: Clone + Field + num_traits::Float + num_traits::FloatConst,
    U: CTUnit + Default + Clone,
{
    /// 获取指定朝向尺寸 / Get dimensions for a specific orientation
    pub fn oriented_dimensions(&self, orientation: Orientation) -> (V, V, V) {
        let perm = orientation.to_axis_permutation();
        let cuboid = ospf_rust_math::geometry::Cuboid3::new(
            self.width.value,
            self.height.value,
            self.depth.value,
        );
        let permuted = perm.apply_cuboid(&cuboid);
        (permuted.width, permuted.height, permuted.depth)
    }

    /// 获取指定朝向包装形状 / Get packing shape for a specific orientation
    pub fn oriented_packing_shape(&self, orientation: Orientation) -> PackingShape3<V, U> {
        let mut shape = self.packing_shape();
        let perm = orientation.to_axis_permutation();
        let cuboid = ospf_rust_math::geometry::Cuboid3::new(
            shape.bounding_width.value,
            shape.bounding_height.value,
            shape.bounding_depth.value,
        );
        let permuted = perm.apply_cuboid(&cuboid);
        shape.bounding_width = Quantity::new_ct(permuted.width);
        shape.bounding_height = Quantity::new_ct(permuted.height);
        shape.bounding_depth = Quantity::new_ct(permuted.depth);
        if let Some(axis) = shape.axis {
            let mapped_axis = perm.map_axis(axis).unwrap_or(axis);
            shape.axis = Some(mapped_axis);
            shape.algorithm_shape_type =
                crate::infrastructure::PackingAlgorithmShapeType::from_cylinder_axis(mapped_axis);
        }
        shape
    }
}

impl<V, U> ActualItem<V, U>
where
    V: Clone + Field + num_traits::Float + num_traits::FloatConst + ToPrimitive + PartialOrd,
    U: CTUnit + Default + Clone,
{
    /// 获取当前箱型空间内可用朝向 / Get orientations enabled in the current bin space
    pub fn enabled_orientations_at_bin(
        &self,
        package_attribute: Option<&PackageAttribute>,
        bin: &BinType<V, U>,
    ) -> Vec<Orientation> {
        let base_orientations = if self.enabled_orientations.is_empty() {
            vec![Orientation::Upright]
        } else {
            self.enabled_orientations.clone()
        };
        let mut orientations = base_orientations.clone();
        if let Some(attribute) = package_attribute {
            let item_height = self.height.value.to_f64().unwrap_or(f64::INFINITY);
            let space_height = bin.height.value.to_f64().unwrap_or(f64::INFINITY);
            if space_height <= item_height {
                if attribute.enabled_side_on_top() {
                    for orientation in Orientation::ALL
                        .iter()
                        .copied()
                        .filter(|orientation| orientation.category() == OrientationCategory::Side)
                    {
                        if !orientations.contains(&orientation) {
                            orientations.push(orientation);
                        }
                    }
                }
                if attribute.enabled_lie_on_top() {
                    for orientation in Orientation::ALL
                        .iter()
                        .copied()
                        .filter(|orientation| orientation.category() == OrientationCategory::Lie)
                    {
                        if !orientations.contains(&orientation) {
                            orientations.push(orientation);
                        }
                    }
                }
            }
        }
        orientations
            .into_iter()
            .filter(|orientation| {
                let (width, height, depth) = self.oriented_dimensions(*orientation);
                if width > bin.width.value || height > bin.height.value || depth > bin.depth.value {
                    return false;
                }
                let Some(attribute) = package_attribute else {
                    return true;
                };
                let input = PackageOrientationRuleInput {
                    orientation: *orientation,
                    space_width: bin.width.value.to_f64().unwrap_or(f64::INFINITY),
                    space_height: bin.height.value.to_f64().unwrap_or(f64::INFINITY),
                    space_depth: bin.depth.value.to_f64().unwrap_or(f64::INFINITY),
                };
                attribute.enabled_orientation_by_rule(&input)
            })
            .collect()
    }
}

impl<V, U> ActualItem<V, U>
where
    V: Clone + Field + num_traits::Float + num_traits::FloatConst + ToPrimitive,
    U: CTUnit + Default + Clone,
{
    /// 物料重量 / Material weights
    pub fn material_weights(&self, material_catalog: &[Material<V, U>]) -> Vec<(MaterialKey, Quantity<V, U>)> {
        let catalog = material_catalog
            .iter()
            .map(|material| (material.key(), material.weight.clone()))
            .collect::<HashMap<_, _>>();
        let mut material_weights = self
            .material_amounts()
            .into_iter()
            .filter_map(|(material_key, amount)| {
                let unit_weight = catalog.get(&material_key)?;
                let multiplier = V::from(amount).unwrap_or_else(V::zero);
                Some((material_key, Quantity::new_ct(unit_weight.value.clone() * multiplier)))
            })
            .collect::<Vec<_>>();
        material_weights.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
        material_weights
    }
}

