// ============================================================================
// DemandConstraint - 需求约束 / Demand constraint
// ============================================================================

/// 需求约束 / Demand constraint
///
/// 对每个需求条目添加上下界约束：
/// - RMP: `demand <= sum(x[layer] * layer_demand[layer]) + overLoad`
/// - Final MILP: `demand <= sum(x[bin, layer] * layer_demand[layer]) + overLoad`
///
/// 同时支持从对偶解提取 shadow price。
///
/// Adds lower and upper bound constraints for each demand entry:
/// - RMP: `demand <= sum(x[layer] * layer_demand[layer]) + overLoad`
/// - Final MILP: `demand <= sum(x[bin, layer] * layer_demand[layer]) + overLoad`
///
/// Also supports shadow price extraction from dual solution.
#[derive(Debug, Clone)]
pub struct DemandConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    name: String,
    group: Option<ConstraintGroup>,
    /// 需求条目列表 / Demand entries
    pub demand_entries: Vec<Bpp3dDemandEntry>,
    /// 赋值变量引用 / Assignment variable references
    pub assignment_ref: DemandAssignmentRef<V, U>,
}

/// 需求赋值变量引用 / Demand assignment variable references
///
/// 持有赋值变量的索引信息，用于约束注册和 shadow price 提取。
/// Holds assignment variable index information for constraint registration
/// and shadow price extraction.
#[derive(Debug, Clone)]
pub enum DemandAssignmentRef<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// RMP 阶段：x[layer] 连续变量 / RMP phase: x[layer] continuous variables
    Imprecise {
        /// 赋值模型 / Assignment model
        assignment: ImpreciseAssignment<V, U>,
    },
    /// Final MILP 阶段：x[bin, layer] 二值变量 / Final MILP phase: x[bin, layer] binary variables
    Precise {
        /// 赋值模型 / Assignment model
        assignment: PreciseAssignment<V, U>,
    },
}

impl<V, U> DemandConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建 RMP 阶段需求约束 / Create RMP phase demand constraint
    pub fn imprecise(
        demand_entries: Vec<Bpp3dDemandEntry>,
        assignment: ImpreciseAssignment<V, U>,
    ) -> Self {
        Self {
            name: "demand_constraint".to_string(),
            group: None,
            demand_entries,
            assignment_ref: DemandAssignmentRef::Imprecise { assignment },
        }
    }

    /// 创建 Final MILP 阶段需求约束 / Create Final MILP phase demand constraint
    pub fn precise(
        demand_entries: Vec<Bpp3dDemandEntry>,
        assignment: PreciseAssignment<V, U>,
    ) -> Self {
        Self {
            name: "demand_constraint".to_string(),
            group: None,
            demand_entries,
            assignment_ref: DemandAssignmentRef::Precise { assignment },
        }
    }

    /// 获取需求 shadow price 键 / Get demand shadow price key
    pub fn shadow_price_key(entry: &Bpp3dDemandEntry) -> DemandShadowPriceKey {
        DemandShadowPriceKey {
            mode: entry.mode,
            key: entry.key.clone(),
        }
    }
}

impl<V, U> Pipeline<MetaModel<f64>> for DemandConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        match &self.assignment_ref {
            DemandAssignmentRef::Imprecise { assignment } => {
                for (demand_idx, entry) in self.demand_entries.iter().enumerate() {
                    let terms = imprecise_layer_terms(assignment, entry);
                    if let Err(e) = model.add_eq_constraint(
                        &terms,
                        entry.demand,
                        &format!("{}_{}", self.name, demand_idx),
                    ) {
                        log::warn!("Failed to register {}_{}: {:?}", self.name, demand_idx, e);
                    }
                }
            }
            DemandAssignmentRef::Precise { assignment } => {
                for (demand_idx, entry) in self.demand_entries.iter().enumerate() {
                    let terms = precise_layer_terms(assignment, entry);
                    if let Err(e) = model.add_eq_constraint(
                        &terms,
                        entry.demand,
                        &format!("{}_{}", self.name, demand_idx),
                    ) {
                        log::warn!("Failed to register {}_{}: {:?}", self.name, demand_idx, e);
                    }
                }
            }
        }
    }

    fn invoke(&self, model: &MetaModel<f64>) -> Result<()> {
        let _ = model;
        Ok(())
    }
}

fn imprecise_layer_terms<V, U>(
    assignment: &ImpreciseAssignment<V, U>,
    entry: &Bpp3dDemandEntry,
) -> Vec<(usize, f64)>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    // Use registered load symbols when available (Phase J)
    if !assignment.load_symbols.is_empty() {
        let mut terms = Vec::new();
        for (layer_idx, symbol) in assignment.load_symbols.iter().enumerate() {
            let coefficient = assignment.layers[layer_idx]
                .demand_coverage_coefficient(entry.mode, &entry.key);
            if coefficient != 0.0 {
                // Extract the symbol's polynomial terms and scale by coefficient
                let poly = symbol.to_linear_polynomial();
                for monomial in poly.monomials() {
                    terms.push((monomial.var_index(), coefficient * *monomial.coefficient()));
                }
            }
        }
        return terms;
    }

    // Fallback: compute raw terms from variable indices
    let x = match assignment.x.as_ref() {
        Some(x) => x,
        None => return Vec::new(),
    };
    (0..assignment.layers.len())
        .filter_map(|layer_idx| {
            let model_idx = x.model_index(&layer_idx)?;
            let coefficient = assignment.layers[layer_idx]
                .demand_coverage_coefficient(entry.mode, &entry.key);
            (coefficient != 0.0).then_some((model_idx, coefficient))
        })
        .collect()
}

fn precise_layer_terms<V, U>(
    assignment: &PreciseAssignment<V, U>,
    entry: &Bpp3dDemandEntry,
) -> Vec<(usize, f64)>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    let x = match assignment.x.as_ref() {
        Some(x) => x,
        None => return Vec::new(),
    };
    assignment.bins
        .iter()
        .enumerate()
        .flat_map(|(bin_idx, _)| {
            assignment.layers.iter().enumerate().filter_map(move |(layer_idx, _)| {
                let model_idx = x.model_index(&bin_idx, &layer_idx)?;
                let coefficient = assignment.layers[layer_idx]
                    .demand_coverage_coefficient(entry.mode, &entry.key);
                (coefficient != 0.0).then_some((model_idx, coefficient))
            })
        })
        .collect()
}

fn precise_bin_marker_index<V, U>(
    assignment: &PreciseAssignment<V, U>,
    bin_idx: usize,
) -> Option<usize>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    assignment
        .v
        .as_ref()
        .and_then(|v| v.model_index(&bin_idx))
}

fn precise_assignment_index<V, U>(
    assignment: &PreciseAssignment<V, U>,
    bin_idx: usize,
    layer_idx: usize,
) -> Option<usize>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    assignment
        .x
        .as_ref()
        .and_then(|x| x.model_index(&bin_idx, &layer_idx))
}

