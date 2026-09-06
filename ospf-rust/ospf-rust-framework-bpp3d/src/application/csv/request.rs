/// CSV application request 草稿 / CSV application request draft
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvApplicationRequestDraft {
    /// 货物数量 / Item count
    pub item_count: usize,
    /// 箱数量 / Bin count
    pub bin_count: usize,
    /// 层数量 / Layer count
    pub layer_count: usize,
    /// 深度边界策略 / Depth boundary policy
    pub depth_boundary_policy: Option<CsvDepthBoundaryPolicy>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

/// CSV 物化 application request / CSV materialized application request
#[derive(Debug, Clone)]
pub struct CsvMaterializedApplicationRequest {
    /// 货物列表 / Items
    pub items: Vec<ActualItem<f64, Meter>>,
    /// 货物数量 / Item amounts
    pub item_amounts: Vec<(String, u64)>,
    /// 箱型列表 / Bin types
    pub bins: Vec<BinType<f64, Meter>>,
    /// 初始层 / Initial layers
    pub initial_layers: Vec<BinLayer<f64, Meter>>,
    /// 深度边界策略 / Depth boundary policy
    pub depth_boundary_policy: Option<CsvDepthBoundaryPolicy>,
    /// 模式货物键 / Patterned item keys
    pub patterned_items: Vec<(String, PatternedItemKey)>,
    /// 包装属性 / Package attributes
    pub package_attributes: Vec<(String, PackageAttribute)>,
    /// 连续半径模型组件 / Continuous radius model component
    pub continuous_radius_component: Option<ContinuousRadiusModelComponent>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

impl CsvMaterializedApplicationRequest {
    /// 校验物化业务规则 / Validate materialized business rules
    pub fn validate_business_rules(&self) -> Vec<String> {
        let mut diagnostics = Vec::new();
        let item_ids = self
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect::<std::collections::HashSet<_>>();
        let mut pattern_counts = std::collections::HashMap::<String, usize>::new();
        for (item_id, pattern) in &self.patterned_items {
            if item_ids.contains(item_id.as_str()) {
                diagnostics.push(format!(
                    "patterned item '{}' mapped to pattern '{}'",
                    item_id,
                    pattern.pattern_code,
                ));
                *pattern_counts.entry(pattern.pattern_code.clone()).or_default() += 1;
            } else {
                diagnostics.push(format!(
                    "patterned item '{}' references unknown item",
                    item_id,
                ));
            }
        }
        let mut mixed_loading_disabled = 0usize;
        let mut max_stack_layers = Vec::new();
        let mut tags = std::collections::BTreeSet::new();
        for (item_id, attribute) in &self.package_attributes {
            if !item_ids.contains(item_id.as_str()) {
                diagnostics.push(format!(
                    "package attribute '{}' references unknown item",
                    item_id,
                ));
            }
            if attribute.allow_mixed_loading == Some(false) {
                mixed_loading_disabled += 1;
            }
            if let Some(layer_count) = attribute.max_stack_layers {
                max_stack_layers.push(layer_count);
            }
            tags.extend(attribute.tags.iter().cloned());
            diagnostics.extend(
                attribute
                    .validate()
                    .into_iter()
                    .map(|diagnostic| format!("package attribute '{}': {}", item_id, diagnostic)),
            );
        }
        if !pattern_counts.is_empty() {
            let mut summary = pattern_counts
                .into_iter()
                .collect::<Vec<_>>();
            summary.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
            diagnostics.push(format!(
                "patterned item groups: {}",
                summary
                    .iter()
                    .map(|(pattern, count)| format!("{}={}", pattern, count))
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }
        if !self.package_attributes.is_empty() {
            diagnostics.push(format!(
                "package attribute summary: total={}, mixed_loading_disabled={}, min_stack_layers={}, tags={}",
                self.package_attributes.len(),
                mixed_loading_disabled,
                max_stack_layers.into_iter().min().unwrap_or(0),
                tags.into_iter().collect::<Vec<_>>().join("|"),
            ));
        }
        if self
            .initial_layers
            .iter()
            .all(|layer| layer.demand_coverage.is_empty())
            && !self.items.is_empty()
        {
            diagnostics.push(
                "initial layers do not carry explicit demand coverage; application defaults will be applied"
                    .to_string(),
            );
        }
        if let Some(component) = &self.continuous_radius_component {
            for (key, value) in component.info() {
                diagnostics.push(format!("continuous radius {}={}", key, value));
            }
            for prototype in &component.prototypes {
                diagnostics.push(format!(
                    "continuous radius prototype '{}' axis={:?} bounds={:?}..{:?}",
                    prototype.variable_name,
                    prototype.axis,
                    prototype.radius_lower_bound,
                    prototype.radius_upper_bound,
                ));
            }
        }
        diagnostics
    }
}
