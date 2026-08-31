use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ospf_rust_core::error::Result as CoreResult;
use ospf_rust_core::model::{ConstraintGroup, ConstraintRelation, MetaModel};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::CTUnit;
use ospf_rust_quantities::unit::derived::{Kilogram, KilogramPerSquareMeter, Meter};
use ospf_rust_quantities::dimension::derived_quantity::QuantityDomain;
use ospf_rust_framework::model::Pipeline;
use ospf_rust_framework_csp1d::{
    convert_solver_value, csp1d_problem, csp1d_solve_config, shadow_price_key_from_string,
    shadow_price_key_to_string, shadow_price_unit_symbol, roll_count_unit, sheet_count_unit,
    accept_partial_by_policies,
    filter_initial_plans_by_policies, is_equivalent_by_policies, select_termination_by_policies,
    select_termination_by_policies_with_default, Csp1dCandidateFilter,
    Csp1dAssignment, Csp1dColumnGeneration, Csp1dColumnGenerationRecovery,
    Csp1dConfiguration, Csp1dMilp, Csp1dMilpSolver, Csp1dDomainCalculationContext,
    Csp1dDomainPolicy, Csp1dError,
    Csp1dFinalMilpStatus, Csp1dFlowContext, Csp1dFlowPolicy, Csp1dIncrementalPipeline,
    Csp1dInitialCuttingPlanGenerator, Csp1dIterativeContext,
    Csp1dExtractionPolicy, Csp1dKpiKeys, Csp1dModelContext, Csp1dModelingContext,
    Csp1dModelingMode,
    Csp1dPricingInput, Csp1dPricingGenerator,
    Csp1dProduceContextBuilder, Csp1dRecovery, Csp1dRecoveryInput, Csp1dRecoveryStatus,
    Csp1dRecoveryOptions, Csp1dShadowPriceKey, Csp1dShadowPriceLifecycle,
    Csp1dExtensionSet, Csp1dModelingExtension, Csp1dSchedule, Csp1dSolutionStatus,
    Csp1dTerminationReason, Csp1dWarmStart, Csp1dWarmStartAdapter,
    Csp1dWarmStartAdapterInput, Csp1dWarmStartPlanPoolAdapter, Csp1dWarmStartStatus,
    Csp1dWidthFeasibilityCheck,
    Costar, CostarFiller, CuttingPlan, CuttingPlanConstraint, CuttingPlanConstraintContext,
    CuttingPlanDemandContribution, CuttingPlanGenerationInput,
    CuttingPlanGenerationBenchmarkSnapshot, CuttingPlanGenerationReport,
    CuttingPlanGenerationStatistics, CuttingPlanGenerationStopReason, CuttingPlanProduction,
    CuttingPlanSlice, DFSGenerator, DefaultQuantityArithmetic, DemandMode, FullSumGenerator, GenerationConstraints,
    GenerationReportMergeOptions, LengthAssignmentModelingConfig, LengthObjectivePipeline, Machine, Material,
    MaterialUsageShadowPriceKey, MaxKnifeCountConstraint, MaxOverProduceLengthConstraint,
    merge_generation_reports, MinKnifeCountConstraint, NSameGenerator, NSumGenerator, Product, ProductDemand,
    ProductLegacyInput, ProduceAggregation, ProduceInput, ProductDemandShadowPriceKey, Production,
    QuantityArithmetic, QuantityRange, ReducedCostPricingGenerator,
    SimpleInitialCuttingPlanGenerator, WasteMinimizationConfig, WasteObjectivePipeline,
    WidthRange, WidthUpperBoundConstraint, YieldModelingConfig, YieldObjectivePipeline,
};

fn quantity(value: f64) -> Quantity<f64, ospf_rust_quantities::unit::Unit> {
    Quantity::new(value, Meter::INSTANT.clone())
}

fn product() -> Product<f64> {
    Product {
        id: "p1".into(),
        name: "Product 1".into(),
        width: vec![quantity(30.0)],
        length: None,
        unit_weight: None,
        weight: None,
        max_over_produce_length: None,
        dynamic_length: false,
    }
}

fn material() -> Material<f64> {
    Material {
        id: "m1".into(),
        name: "Material 1".into(),
        width_range: WidthRange::new(quantity(0.0), quantity(100.0)),
        length: None,
        unit_weight: None,
        machine_id: Some("mc1".into()),
        available_batches: 3,
    }
}

fn machine() -> Machine<f64> {
    Machine {
        id: "mc1".into(),
        name: "Machine 1".into(),
        max_batch_count: Some(3),
        max_switch_count: Some(1),
        width_range: Some(WidthRange::new(quantity(0.0), quantity(100.0))),
        capacity: Some(quantity(8.0)),
    }
}

fn demand() -> ProductDemand<f64> {
    ProductDemand {
        product: product(),
        quantity: quantity(2.0),
        mode: Some(DemandMode::Roll),
    }
}

fn cutting_plan(id: &str) -> CuttingPlan<f64> {
    CuttingPlan {
        id: id.into(),
        material: material(),
        machine_id: Some("mc1".into()),
        slices: vec![CuttingPlanSlice {
            production: CuttingPlanProduction::Product(product()),
            width: quantity(30.0),
            amount: 2,
        }],
        demand_contributions: vec![CuttingPlanDemandContribution {
            product: product(),
            quantity: quantity(2.0),
        }],
        capacity_consumption: Some(quantity(1.0)),
    }
}

fn material_with_length() -> Material<f64> {
    Material {
        length: Some(quantity(10.0)),
        ..material()
    }
}

fn dynamic_product() -> Product<f64> {
    Product {
        id: "p-dyn".into(),
        name: "Dynamic Product".into(),
        width: vec![quantity(10.0)],
        length: None,
        unit_weight: None,
        weight: None,
        max_over_produce_length: Some(quantity(1.0)),
        dynamic_length: true,
    }
}

fn dynamic_demand() -> ProductDemand<f64> {
    ProductDemand {
        product: dynamic_product(),
        quantity: quantity(2.0),
        mode: Some(DemandMode::Roll),
    }
}

fn over_producing_plan(id: &str) -> CuttingPlan<f64> {
    let plan_product = product();
    CuttingPlan {
        id: id.into(),
        material: material_with_length(),
        machine_id: Some("mc1".into()),
        slices: vec![CuttingPlanSlice {
            production: CuttingPlanProduction::Product(plan_product.clone()),
            width: quantity(30.0),
            amount: 2,
        }],
        demand_contributions: vec![CuttingPlanDemandContribution {
            product: plan_product,
            quantity: quantity(2.0),
        }],
        capacity_consumption: Some(quantity(1.0)),
    }
}

#[derive(Debug, Clone)]
struct FixedPlanEnumerator {
    plans: Vec<CuttingPlan<f64>>,
}

impl Csp1dInitialCuttingPlanGenerator<f64> for FixedPlanEnumerator {
    fn generate(&self, _input: &CuttingPlanGenerationInput<f64>) -> Vec<CuttingPlan<f64>> {
        self.plans.clone()
    }

    fn generate_with_report(
        &self,
        _input: &CuttingPlanGenerationInput<f64>,
    ) -> CuttingPlanGenerationReport<f64> {
        CuttingPlanGenerationReport {
            plans: self.plans.clone(),
            statistics: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct EmptyPricingGenerator;

impl Csp1dPricingGenerator<f64> for EmptyPricingGenerator {
    fn generate(&self, _input: &Csp1dPricingInput<f64>) -> Vec<CuttingPlan<f64>> {
        Vec::new()
    }

    fn generate_with_report(
        &self,
        _input: &Csp1dPricingInput<f64>,
    ) -> CuttingPlanGenerationReport<f64> {
        CuttingPlanGenerationReport {
            plans: Vec::new(),
            statistics: CuttingPlanGenerationStatistics::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct FixedPricingGenerator {
    plans: Vec<CuttingPlan<f64>>,
}

impl Csp1dPricingGenerator<f64> for FixedPricingGenerator {
    fn generate(&self, _input: &Csp1dPricingInput<f64>) -> Vec<CuttingPlan<f64>> {
        self.plans.clone()
    }

    fn generate_with_report(
        &self,
        _input: &Csp1dPricingInput<f64>,
    ) -> CuttingPlanGenerationReport<f64> {
        CuttingPlanGenerationReport {
            plans: self.plans.clone(),
            statistics: CuttingPlanGenerationStatistics {
                generated_candidates: self.plans.len() as i64,
                accepted_plans: self.plans.len() as i64,
                ..CuttingPlanGenerationStatistics::default()
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
struct InspectingPricingGenerator {
    observed_demand_shadow_price: Arc<AtomicBool>,
}

impl Csp1dPricingGenerator<f64> for InspectingPricingGenerator {
    fn generate(&self, input: &Csp1dPricingInput<f64>) -> Vec<CuttingPlan<f64>> {
        let key = Csp1dShadowPriceKey::ProductDemand(ProductDemandShadowPriceKey {
            product_id: "p1".into(),
            unit_symbol: "m".into(),
        });
        if input.shadow_prices.get(&key).copied() == Some(1.0) {
            self.observed_demand_shadow_price
                .store(true, Ordering::SeqCst);
        }
        Vec::new()
    }

    fn generate_with_report(
        &self,
        input: &Csp1dPricingInput<f64>,
    ) -> CuttingPlanGenerationReport<f64> {
        CuttingPlanGenerationReport {
            plans: self.generate(input),
            statistics: CuttingPlanGenerationStatistics::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct CountingIncrementalPipeline {
    group: ConstraintGroup,
}

impl CountingIncrementalPipeline {
    fn new() -> Self {
        Self {
            group: ConstraintGroup::new(90_001, "test_incremental_pipeline"),
        }
    }
}

impl Pipeline<MetaModel<f64>> for CountingIncrementalPipeline {
    fn name(&self) -> &str {
        "counting_incremental"
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        Some(&self.group)
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        let _ = model.add_linear_constraint_with_metadata(
            &[],
            ConstraintRelation::LessEqual,
            1.0,
            "incremental_initial",
            Some(Arc::new(self.group.clone())),
            false,
            0,
            None,
        );
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> CoreResult<()> {
        Ok(())
    }
}

impl Csp1dIncrementalPipeline<f64> for CountingIncrementalPipeline {
    fn add_columns(
        &self,
        _context: &dyn Csp1dModelingContext<f64>,
        iteration: u64,
        new_plans: Vec<CuttingPlan<f64>>,
        model: &mut MetaModel<f64>,
    ) -> ospf_rust_framework_csp1d::Csp1dResult<Vec<CuttingPlan<f64>>> {
        let _ = model.add_linear_constraint_with_metadata(
            &[],
            ConstraintRelation::LessEqual,
            new_plans.len() as f64,
            &format!("incremental_added_{iteration}"),
            Some(Arc::new(self.group.clone())),
            false,
            0,
            None,
        );
        Ok(new_plans)
    }
}

#[derive(Debug, Clone)]
struct NamedNoopPipeline {
    name: &'static str,
}

impl Pipeline<MetaModel<f64>> for NamedNoopPipeline {
    fn name(&self) -> &str {
        self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        None
    }

    fn register(&self, _model: &mut MetaModel<f64>) {
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> CoreResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct FixedRecoveryFallbackPolicy {
    decision: bool,
    observed_context: Arc<AtomicBool>,
}

impl Csp1dFlowPolicy<f64> for FixedRecoveryFallbackPolicy {
    fn name(&self) -> &str {
        "fixed_recovery_fallback"
    }

    fn allow_recovery_fallback(
        &self,
        context: &dyn Csp1dFlowContext<f64>,
        _default_decision: bool,
    ) -> bool {
        if context.warm_start_requires_fallback()
            && context.warm_start_plan_count() == 1
            && context.iteration() == 0
        {
            self.observed_context.store(true, Ordering::SeqCst);
        }
        self.decision
    }
}

#[derive(Debug, Clone)]
struct FilteringFlowPolicy {
    rejected_plan_id: String,
}

impl Csp1dFlowPolicy<f64> for FilteringFlowPolicy {
    fn name(&self) -> &str {
        "filtering_flow_policy"
    }

    fn filter_initial_plans(
        &self,
        _context: &dyn Csp1dFlowContext<f64>,
        plans: Vec<CuttingPlan<f64>>,
    ) -> Vec<CuttingPlan<f64>> {
        plans
            .into_iter()
            .filter(|plan| plan.id != self.rejected_plan_id)
            .collect()
    }

    fn is_equivalent(
        &self,
        _context: &dyn Csp1dFlowContext<f64>,
        existing: &CuttingPlan<f64>,
        candidate: &CuttingPlan<f64>,
    ) -> bool {
        existing.material.id == candidate.material.id
    }

    fn accept_partial(
        &self,
        _context: &dyn Csp1dFlowContext<f64>,
        _default_decision: bool,
    ) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
struct InspectingInitialFlowPolicy {
    observed: Arc<AtomicBool>,
}

impl Csp1dFlowPolicy<f64> for InspectingInitialFlowPolicy {
    fn name(&self) -> &str {
        "inspecting_initial_flow_policy"
    }

    fn filter_initial_plans(
        &self,
        context: &dyn Csp1dFlowContext<f64>,
        plans: Vec<CuttingPlan<f64>>,
    ) -> Vec<CuttingPlan<f64>> {
        if context.iteration() == 0
            && context.iteration_limit() == 7
            && context.current_plans().len() == 2
            && context.allow_partial_solution()
        {
            self.observed.store(true, Ordering::SeqCst);
        }
        plans
    }
}

#[derive(Debug, Clone)]
struct StopAfterPricingFlowPolicy;

impl Csp1dFlowPolicy<f64> for StopAfterPricingFlowPolicy {
    fn name(&self) -> &str {
        "stop_after_pricing"
    }

    fn should_stop_iteration(&self, context: &dyn Csp1dFlowContext<f64>) -> bool {
        !context.new_plans().is_empty()
    }

    fn select_termination(
        &self,
        _context: &dyn Csp1dFlowContext<f64>,
        _default_reason: String,
        _default_message: Option<String>,
    ) -> (String, Option<String>) {
        (
            "IterationLimitReached".into(),
            Some("Stopped by test flow policy".into()),
        )
    }
}

#[derive(Debug, Clone)]
struct RelaxingWidthPolicy;

impl Csp1dDomainPolicy<f64> for RelaxingWidthPolicy {
    fn name(&self) -> &str {
        "relaxing_width_policy"
    }

    fn overrides_width_feasibility(&self) -> bool {
        true
    }

    fn is_width_feasible(&self, _context: &dyn Csp1dDomainCalculationContext<f64>) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
struct ContributionThresholdPolicy {
    product_id: String,
    min_contribution: f64,
}

impl Csp1dDomainPolicy<f64> for ContributionThresholdPolicy {
    fn name(&self) -> &str {
        "contribution_threshold_policy"
    }

    fn is_feasible(&self, context: &dyn Csp1dDomainCalculationContext<f64>) -> bool {
        context.material().id == "m1"
            && context.machine_id() == Some("mc1")
            && context.slices().len() == 1
            && context.contribution_for(&self.product_id).unwrap_or(0.0)
                >= self.min_contribution
    }
}

#[derive(Debug, Clone, Default)]
struct TestFlowContext {
    plans: Vec<CuttingPlan<f64>>,
}

impl Csp1dFlowContext<f64> for TestFlowContext {
    fn iteration(&self) -> u64 {
        0
    }

    fn current_plans(&self) -> &[CuttingPlan<f64>] {
        &self.plans
    }

    fn iteration_limit(&self) -> u64 {
        1
    }

    fn allow_partial_solution(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
struct CustomExtractionPolicy {
    panic_on_enrich: bool,
}

impl Csp1dExtractionPolicy<f64> for CustomExtractionPolicy {
    fn name(&self) -> &str {
        if self.panic_on_enrich {
            "panic_extraction"
        } else {
            "custom_extraction"
        }
    }

    fn enrich_output(
        &self,
        details: &mut BTreeMap<String, String>,
        render_kpi: &mut BTreeMap<String, String>,
        _produce: &ospf_rust_framework_csp1d::Produce<f64>,
        _demands: &[ProductDemand<f64>],
        _materials: &[Material<f64>],
        _machines: &[Machine<f64>],
        _generated_plans: &[CuttingPlan<f64>],
        _iteration_count: u64,
        _termination_reason: Option<&str>,
        _final_milp_status: Option<&str>,
        _pricing_statistics: Option<&CuttingPlanGenerationStatistics>,
    ) {
        if self.panic_on_enrich {
            panic!("intentional extraction policy failure");
        }
        details.insert("custom-detail-key".into(), "detail-value".into());
        render_kpi.insert("custom-render-key".into(), "render-value".into());
    }
}

#[test]
fn builder_constructs_problem_and_solve_config() {
    let solve_config = csp1d_solve_config::<f64, _>(|builder| {
        builder
            .column_generation_limits(7, 3, 2)
            .allow_partial_solution(false)
            .top_k_plan_limit(Some(5));
    });
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand())
            .configuration(Csp1dConfiguration {
                max_initial_plans: 4,
                max_pricing_plans: 2,
                iteration_limit: 1,
            })
            .solve_config(solve_config.clone());
    });

    assert_eq!(problem.products.len(), 1);
    assert_eq!(problem.materials.len(), 1);
    assert_eq!(problem.machines.len(), 1);
    assert_eq!(problem.demands.len(), 1);
    assert_eq!(problem.configuration.max_initial_plans, 4);
    let attached = problem.solve_config.as_ref().expect("solve config should be attached");
    assert_eq!(attached.column_generation.max_initial_plans, 7);
    assert_eq!(attached.column_generation.max_pricing_plans, 3);
    assert_eq!(attached.column_generation.iteration_limit, 2);
    assert!(!attached.allow_partial_solution);
    assert_eq!(attached.top_k_plan_limit, Some(5));
}

#[test]
fn assignment_registers_plan_usage_variables_like_kotlin_helper() {
    let mut assignment = Csp1dAssignment::create(3);
    let mut model = MetaModel::<f64>::new("assignment");

    assignment
        .register(&mut model)
        .expect("assignment variables should register");

    assert_eq!(assignment.plan_count, 3);
    assert_eq!(assignment.x.len(), 3);
    assert_eq!(assignment.get(1), Some(assignment.x[1]));
    assert!(model.tokens().iter().any(|token| token.variable.name() == "x_1"));
}

#[test]
fn solve_config_all_extensions_keeps_distinct_same_mode_extensions() {
    let first_pipeline = Arc::new(NamedNoopPipeline { name: "first" });
    let second_pipeline = Arc::new(NamedNoopPipeline { name: "second" });
    let duplicated = Csp1dModelingExtension::new(first_pipeline.clone());
    let config = csp1d_solve_config::<f64, _>(|builder| {
        builder.extension(duplicated.clone());
    });
    let mut config = config;
    config.extension_set = Csp1dExtensionSet {
        modeling_extensions: vec![
            Csp1dModelingExtension::new(first_pipeline),
            Csp1dModelingExtension::new(second_pipeline),
        ],
        ..Csp1dExtensionSet::default()
    };

    let extensions = config.all_extensions();

    assert_eq!(extensions.len(), 2);
    assert_eq!(extensions[0].resolve_pipeline(None).unwrap().name(), "first");
    assert_eq!(extensions[1].resolve_pipeline(None).unwrap().name(), "second");
}

#[test]
fn cutting_plan_canonical_key_matches_kotlin_structural_dedup_shape() {
    let product = product();
    let costar = Costar {
        id: "c1".into(),
        name: "Costar 1".into(),
        width: vec![quantity(10.0)],
        length: None,
        unit_weight: None,
    };
    let mut split = cutting_plan("split");
    split.slices = vec![
        CuttingPlanSlice {
            production: CuttingPlanProduction::Product(product.clone()),
            width: quantity(30.0),
            amount: 1,
        },
        CuttingPlanSlice {
            production: CuttingPlanProduction::Costar(costar.clone()),
            width: quantity(10.0),
            amount: 1,
        },
        CuttingPlanSlice {
            production: CuttingPlanProduction::Product(product.clone()),
            width: quantity(30.0),
            amount: 1,
        },
    ];
    split.demand_contributions = vec![
        CuttingPlanDemandContribution {
            product: product.clone(),
            quantity: quantity(1.0),
        },
        CuttingPlanDemandContribution {
            product: product.clone(),
            quantity: quantity(1.0),
        },
    ];
    split.capacity_consumption = Some(quantity(2.0));

    let mut merged = cutting_plan("merged");
    merged.slices = vec![
        CuttingPlanSlice {
            production: CuttingPlanProduction::Product(product.clone()),
            width: quantity(30.0),
            amount: 2,
        },
        CuttingPlanSlice {
            production: CuttingPlanProduction::Costar(costar),
            width: quantity(10.0),
            amount: 1,
        },
    ];
    merged.demand_contributions = vec![CuttingPlanDemandContribution {
        product,
        quantity: quantity(2.0),
    }];
    merged.capacity_consumption = Some(quantity(2.0));

    assert_eq!(
        split.canonical_key(),
        merged.canonical_key(),
        "Kotlin canonical key groups duplicate slices and demand contributions"
    );

    let mut different_contribution = merged.clone();
    different_contribution.demand_contributions[0].quantity = quantity(3.0);
    assert_ne!(
        merged.canonical_key(),
        different_contribution.canonical_key(),
        "demand contribution quantity is part of the Kotlin canonical key"
    );

    let mut different_capacity = merged.clone();
    different_capacity.capacity_consumption = Some(quantity(3.0));
    assert_ne!(
        merged.canonical_key(),
        different_capacity.canonical_key(),
        "capacity consumption is part of the Kotlin canonical key"
    );
}

#[test]
fn material_width_feasibility_matches_kotlin_used_width_semantics() {
    let range = WidthRange::new(quantity(50.0), quantity(100.0));
    assert!(
        range.can_cut(&quantity(30.0)),
        "Kotlin WidthRange.canCut only requires product width not exceed upper bound"
    );
    assert!(
        !range.contains(&quantity(30.0)),
        "machine material-range checks still use closed interval containment"
    );

    let mut plan = cutting_plan("too-wide");
    plan.slices = vec![
        CuttingPlanSlice {
            production: CuttingPlanProduction::Product(product()),
            width: quantity(60.0),
            amount: 1,
        },
        CuttingPlanSlice {
            production: CuttingPlanProduction::Product(product()),
            width: quantity(50.0),
            amount: 1,
        },
    ];

    assert!(
        !material().enabled(&plan, &[]),
        "Kotlin Material.enabled checks total usedWidth against the material width range"
    );
    assert_eq!(
        plan.rest_width().expect("rest width should be computed").value,
        -10.0,
        "Kotlin restWidth keeps the arithmetic subtraction result instead of clamping"
    );
}

#[test]
fn width_override_still_checks_machine_feasibility_like_kotlin_material_helper() {
    let mut wide_material = material();
    wide_material.width_range = WidthRange::new(quantity(0.0), quantity(120.0));
    wide_material.machine_id = Some("mc-narrow".into());
    let narrow_machine = Machine {
        id: "mc-narrow".into(),
        name: "Narrow Machine".into(),
        max_batch_count: None,
        max_switch_count: None,
        width_range: Some(WidthRange::new(quantity(0.0), quantity(100.0))),
        capacity: None,
    };
    let mut plan = cutting_plan("wide-material-plan");
    plan.material = wide_material.clone();
    plan.machine_id = Some("mc-narrow".into());

    assert!(
        wide_material.enabled_without_width_check(&plan),
        "Kotlin single-argument helper skips width and machine range checks"
    );
    assert!(
        !wide_material.enabled_without_width_check_with_machines(
            &plan,
            &[narrow_machine.clone()],
        ),
        "Kotlin machine-aware helper still checks machine material width range"
    );

    let input = CuttingPlanGenerationInput {
        products: vec![product()],
        materials: vec![wide_material],
        machines: vec![narrow_machine],
        demands: vec![demand()],
        width_feasibility_check: Some(Arc::new(|_material, _product, _width| true)),
        ..CuttingPlanGenerationInput::default()
    };

    let plans = SimpleInitialCuttingPlanGenerator.generate(&input);

    assert!(
        plans.is_empty(),
        "width override should not bypass Kotlin machine feasibility"
    );
}

#[test]
fn quantity_range_arithmetic_and_production_helpers_match_kotlin() {
    let open_range = QuantityRange::with_bounds(
        quantity(1.0),
        quantity(3.0),
        false,
        true,
    )
    .expect("valid open-left range");
    assert!(!open_range.contains(&quantity(1.0)));
    assert!(open_range.contains(&quantity(2.0)));
    assert!(open_range.contains(&quantity(3.0)));
    assert!(QuantityRange::with_bounds(
        quantity(1.0),
        quantity(1.0),
        false,
        true,
    )
    .is_none());

    let width_range = WidthRange::with_step(
        QuantityRange::new(quantity(0.5), quantity(2.0)).expect("valid width range"),
        quantity(0.1),
    )
    .expect("step unit matches range units");
    assert!(width_range.width.contains(&quantity(1.0)));
    assert!(!WidthRange::with_step(
        QuantityRange::new(quantity(0.5), quantity(2.0)).expect("valid width range"),
        Quantity::new(0.1, Kilogram::INSTANT.clone()),
    )
    .is_some());

    let arithmetic = DefaultQuantityArithmetic::resolve_for(&1.0);
    assert_eq!(
        arithmetic.add(quantity(3.0), quantity(2.0)).unwrap().value,
        5.0
    );
    assert_eq!(
        arithmetic.subtract(quantity(3.0), quantity(2.0)).unwrap().value,
        1.0
    );
    assert_eq!(
        <DefaultQuantityArithmetic as QuantityArithmetic<f64>>::zero(
            &arithmetic,
            Meter::INSTANT.clone(),
        )
            .expect("zero quantity")
            .value,
        0.0
    );
    assert!(arithmetic.is_positive(&quantity(1.0)));
    assert!(arithmetic.is_non_negative(&quantity(0.0)));
    assert!(arithmetic.is_zero(&quantity(0.0)));

    let unit_weight = Quantity::new(2.0, KilogramPerSquareMeter::INSTANT.clone());
    let dynamic = Product::dynamic_length_of_with_unit_weight(
        "dyn",
        "Dynamic",
        vec![quantity(1.2)],
        Some(unit_weight.clone()),
    );
    assert!(dynamic.dynamic_length);
    assert_eq!(
        dynamic.unit_weight.as_ref().unwrap().unit.name(),
        "kilogram per square meter"
    );

    let product = Product {
        id: "weighted".into(),
        name: "Weighted".into(),
        width: vec![quantity(1.0), quantity(1.2)],
        length: Some(quantity(100.0)),
        unit_weight: Some(unit_weight.clone()),
        weight: None,
        max_over_produce_length: None,
        dynamic_length: false,
    };
    let weight = product.weight().expect("weight inferred from max width");
    assert_eq!(weight.value, 240.0);
    assert_eq!(weight.unit.symbol(), "kg");

    let costar = Costar {
        id: "c-weighted".into(),
        name: "Weighted Costar".into(),
        width: vec![quantity(0.2)],
        length: Some(quantity(20.0)),
        unit_weight: Some(unit_weight),
    };
    assert_eq!(Production::length(&costar).unwrap().value, 20.0);
    assert_eq!(
        Production::unit_weight(&costar).unwrap().unit.name(),
        "kilogram per square meter"
    );
    assert_eq!(
        CuttingPlanProduction::Costar(costar).length().unwrap().value,
        20.0
    );
}

#[test]
fn weight_demand_contribution_matches_kotlin_quantity_of_formula() {
    let product = Product {
        id: "p-weight".into(),
        name: "Weight Product".into(),
        width: vec![quantity(2.0)],
        length: Some(quantity(3.0)),
        unit_weight: Some(Quantity::new(4.0, KilogramPerSquareMeter::INSTANT.clone())),
        weight: None,
        max_over_produce_length: None,
        dynamic_length: false,
    };
    let demand = ProductDemand::weight(
        product.clone(),
        Quantity::new(20.0, KilogramPerSquareMeter::INSTANT.clone()),
    );

    let contribution = CuttingPlanDemandContribution::from_demand(
        &demand,
        &quantity(2.0),
        2,
        None,
    );

    assert_eq!(product.weight_for(&quantity(2.0), None).unwrap().value, 24.0);
    assert_eq!(product.weight().unwrap().value, 24.0);
    assert_eq!(contribution.quantity.value, 48.0);
    assert_eq!(contribution.quantity.unit.name(), "kilogram per square meter");

    let product_weight = product.weight().expect("product weight is inferred");
    assert_eq!(product_weight.unit.symbol(), "kg");

    let plans = SimpleInitialCuttingPlanGenerator.generate(&CuttingPlanGenerationInput {
        products: vec![product],
        materials: vec![material()],
        demands: vec![demand],
        ..CuttingPlanGenerationInput::default()
    });

    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].demand_contributions[0].quantity.value, 24.0);
}

#[test]
fn product_demand_legacy_helpers_and_domain_flags_match_kotlin() {
    let roll = ProductDemand::legacy_roll(product(), 3.0);
    assert_eq!(roll.mode, Some(DemandMode::Roll));
    assert_eq!(roll.quantity.unit.symbol(), "roll");
    assert_eq!(roll.quantity.unit.domain(), QuantityDomain::Discrete);
    assert!(roll.is_discrete());
    assert!(!roll.is_continuous());

    let sheet = ProductDemand::legacy_sheet(product(), 4.0);
    assert_eq!(sheet.mode, Some(DemandMode::Sheet));
    assert_eq!(sheet.quantity.unit.symbol(), "sheet");
    assert_eq!(sheet_count_unit().domain(), QuantityDomain::Discrete);

    let weight = ProductDemand::legacy_weight(product(), 5.0);
    assert_eq!(weight.mode, Some(DemandMode::Weight));
    assert_eq!(weight.quantity.unit.symbol(), "kg");
    assert!(weight.is_continuous());
    assert!(!weight.is_discrete());
    assert_eq!(roll_count_unit().symbol(), "roll");
}

#[test]
fn product_legacy_and_solver_value_conversion_match_kotlin_helpers() {
    let product = Product::legacy(ProductLegacyInput {
        id: "legacy".into(),
        name: "Legacy Product".into(),
        width: vec![1.0, 1.2],
        length: Some(100.0),
        unit_weight: Some(2.0),
        weight: Some(240.0),
        max_over_produce_length: Some(5.0),
        unit: Meter::INSTANT.clone(),
    });

    assert_eq!(product.id, "legacy");
    assert_eq!(product.width.len(), 2);
    assert_eq!(product.width[1].value, 1.2);
    assert_eq!(product.width[1].unit.symbol(), "m");
    assert_eq!(product.length.as_ref().unwrap().value, 100.0);
    assert_eq!(product.unit_weight.as_ref().unwrap().value, 2.0);
    assert_eq!(product.weight().unwrap().value, 240.0);
    assert_eq!(product.max_over_produce_length.as_ref().unwrap().value, 5.0);
    assert!(!product.dynamic_length);

    let converted: f64 = convert_solver_value(3.5).expect("convert solver boundary value");
    assert_eq!(converted, 3.5);
}

#[test]
fn domain_policy_context_exposes_kotlin_contribution_for_helper() {
    let mut short_demand = demand();
    short_demand.quantity = quantity(1.0);
    let policy = Arc::new(ContributionThresholdPolicy {
        product_id: "p1".into(),
        min_contribution: 2.0,
    });
    let generator = NSameGenerator::with_constraints(GenerationConstraints {
        max_knife_count: Some(2),
        ..GenerationConstraints::default()
    })
    .with_all_amount(true)
    .with_max_plans(4);
    let input = CuttingPlanGenerationInput {
        products: vec![product()],
        materials: vec![material()],
        machines: vec![machine()],
        demands: vec![short_demand],
        domain_policies: vec![policy],
        ..CuttingPlanGenerationInput::default()
    };

    let report = generator.generate_with_report(&input);

    assert_eq!(report.plans.len(), 1);
    assert_eq!(report.plans[0].slices[0].amount, 2);
    assert_eq!(
        report.plans[0].demand_contributions[0].quantity.value,
        2.0
    );
}

#[test]
fn generation_constraints_expose_kotlin_predicate_contract() {
    let mut long_product = product();
    long_product.length = Some(quantity(12.0));
    let slices = vec![
        CuttingPlanSlice {
            production: CuttingPlanProduction::Product(long_product.clone()),
            width: quantity(30.0),
            amount: 2,
        },
        CuttingPlanSlice {
            production: CuttingPlanProduction::Product(product()),
            width: quantity(20.0),
            amount: 1,
        },
    ];
    let context = CuttingPlanConstraintContext {
        slices,
        total_width: quantity(80.0),
        upper_bound: quantity(100.0),
        material: material(),
    };

    let max = MaxKnifeCountConstraint { value: 3 };
    let min = MinKnifeCountConstraint { value: 4 };
    let length = MaxOverProduceLengthConstraint {
        value: quantity(10.0),
    };
    let width = WidthUpperBoundConstraint;

    assert!(<MaxKnifeCountConstraint as CuttingPlanConstraint<f64>>::is_pruning(&max));
    assert!(max.is_satisfied(&context));
    assert!(!<MinKnifeCountConstraint as CuttingPlanConstraint<f64>>::is_pruning(&min));
    assert!(!min.is_satisfied(&context));
    assert!(!length.is_satisfied(&context));
    assert!(width.is_satisfied(&context));

    let constraints = GenerationConstraints {
        max_knife_count: Some(3),
        min_knife_count: Some(2),
        max_over_produce_length: Some(quantity(10.0)),
        ..GenerationConstraints::default()
    }
    .to_constraints();

    assert_eq!(constraints.len(), 4);
    assert_eq!(constraints[0].name(), "MaxKnifeCountConstraint");
    assert_eq!(constraints[1].name(), "MinKnifeCountConstraint");
    assert_eq!(constraints[2].name(), "MaxOverProduceLengthConstraint");
    assert_eq!(constraints[3].name(), "WidthUpperBoundConstraint");
    assert!(!constraints[1].is_pruning());
}

#[test]
fn produce_aggregation_deduplicates_and_registers_plan_variables() {
    let mut aggregation = ProduceAggregation::new(
        vec![cutting_plan("plan-1"), cutting_plan("plan-duplicate-id")],
        vec![demand()],
        vec![material()],
        vec![machine()],
        Vec::new(),
    );
    assert_eq!(aggregation.plan_count(), 1);

    let added = aggregation.add_columns(1, vec![cutting_plan("plan-2")]);
    assert!(added.is_empty(), "canonical-equivalent plan should be deduplicated");

    let mut different = cutting_plan("plan-3");
    different.slices[0].width = quantity(20.0);
    let added = aggregation.add_columns(2, vec![different]);
    assert_eq!(added.len(), 1);
    assert_eq!(aggregation.plan_count(), 2);

    let mut model = MetaModel::<f64>::new("csp1d_aggregation");
    aggregation.register(&mut model, false).unwrap();
    assert_eq!(aggregation.plan_variable_indices().len(), 2);
    assert_eq!(model.tokens().len(), 2);
}

#[test]
fn flow_policy_helpers_apply_filter_equivalence_and_partial_decisions() {
    let policy = Arc::new(FilteringFlowPolicy {
        rejected_plan_id: "drop-me".into(),
    });
    let plans = vec![cutting_plan("keep-me"), cutting_plan("drop-me")];
    let filtered = filter_initial_plans_by_policies(plans, &[policy.clone()]);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "keep-me");
    assert!(is_equivalent_by_policies(
        &cutting_plan("left"),
        &cutting_plan("right"),
        &[policy.clone()]
    ));
    assert!(!accept_partial_by_policies(
        &TestFlowContext::default(),
        &[policy]
    ));
    let termination_policy = Arc::new(StopAfterPricingFlowPolicy);
    assert_eq!(
        select_termination_by_policies(
            &TestFlowContext::default(),
            &[termination_policy.clone()],
        ),
        (
            "IterationLimitReached".to_string(),
            Some("Stopped by test flow policy".to_string())
        )
    );
    assert_eq!(
        select_termination_by_policies_with_default(
            &TestFlowContext::default(),
            &[termination_policy],
            "PricingConverged".into(),
            None,
        ),
        (
            "IterationLimitReached".to_string(),
            Some("Stopped by test flow policy".to_string())
        )
    );
}

#[test]
fn column_generation_applies_initial_plan_flow_policy_filter() {
    let solve_config = csp1d_solve_config::<f64, _>(|builder| {
        builder.flow_policy(Arc::new(FilteringFlowPolicy {
            rejected_plan_id: "drop-me".into(),
        }));
    });
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand())
            .solve_config(solve_config);
    });
    let mut kept = cutting_plan("keep-me");
    kept.slices[0].width = quantity(30.0);
    let mut dropped = cutting_plan("drop-me");
    dropped.slices[0].width = quantity(20.0);
    let service = Csp1dColumnGeneration::with_generators(
        Box::new(FixedPlanEnumerator {
            plans: vec![kept.clone(), dropped],
        }),
        Box::new(EmptyPricingGenerator),
    );

    let result = service.solve_with_trace(problem, None);

    assert!(result.solution.generated_plans.iter().any(|plan| plan.id == kept.id));
    assert!(!result
        .solution
        .generated_plans
        .iter()
        .any(|plan| plan.id == "drop-me"));
}

#[test]
fn column_generation_initial_flow_policy_receives_kotlin_like_context() {
    let observed = Arc::new(AtomicBool::new(false));
    let solve_config = csp1d_solve_config::<f64, _>(|builder| {
        builder
            .column_generation_limits(4, 1, 7)
            .allow_partial_solution(false)
            .flow_policy(Arc::new(InspectingInitialFlowPolicy {
                observed: observed.clone(),
            }));
    });
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand())
            .solve_config(solve_config);
    });
    let mut second = cutting_plan("initial-context-2");
    second.slices[0].width = quantity(20.0);
    let service = Csp1dColumnGeneration::with_generators(
        Box::new(FixedPlanEnumerator {
            plans: vec![cutting_plan("initial-context-1"), second],
        }),
        Box::new(EmptyPricingGenerator),
    );

    let result = service.solve_with_trace(problem, None);

    assert!(observed.load(Ordering::SeqCst));
    assert_eq!(result.trace.initial_plan_count, 2);
}

#[test]
fn column_generation_uses_flow_policy_equivalence_for_pricing_duplicates() {
    let solve_config = csp1d_solve_config::<f64, _>(|builder| {
        builder
            .column_generation_limits(4, 4, 1)
            .flow_policy(Arc::new(FilteringFlowPolicy {
                rejected_plan_id: "unused".into(),
            }));
    });
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand())
            .solve_config(solve_config);
    });
    let initial = cutting_plan("initial");
    let mut priced_equivalent = cutting_plan("priced-equivalent");
    priced_equivalent.slices[0].width = quantity(20.0);
    let service = Csp1dColumnGeneration::with_generators(
        Box::new(FixedPlanEnumerator {
            plans: vec![initial],
        }),
        Box::new(FixedPricingGenerator {
            plans: vec![priced_equivalent],
        }),
    );

    let result = service.solve_with_trace(problem, None);

    assert_eq!(result.trace.termination_reason, Csp1dTerminationReason::AllDuplicates);
    assert_eq!(result.trace.priced_plan_count, vec![0]);
    assert_eq!(result.solution.generated_plans.len(), 1);
}

#[test]
fn column_generation_honors_flow_policy_early_stop_and_termination_selection() {
    let solve_config = csp1d_solve_config::<f64, _>(|builder| {
        builder
            .column_generation_limits(4, 4, 3)
            .flow_policy(Arc::new(StopAfterPricingFlowPolicy));
    });
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand())
            .solve_config(solve_config);
    });
    let initial = cutting_plan("initial-stop");
    let mut priced = cutting_plan("priced-stop");
    priced.slices[0].width = quantity(20.0);
    priced.demand_contributions[0].quantity = quantity(1.0);
    let service = Csp1dColumnGeneration::with_generators(
        Box::new(FixedPlanEnumerator {
            plans: vec![initial],
        }),
        Box::new(FixedPricingGenerator { plans: vec![priced] }),
    );

    let result = service.solve_with_trace(problem, None);

    assert_eq!(
        result.trace.termination_reason,
        Csp1dTerminationReason::IterationLimitReached
    );
    assert_eq!(result.trace.iterations.len(), 1);
    assert_eq!(result.trace.priced_plan_count, vec![1]);
    assert_eq!(result.trace.initial_plan_count, 1);
    assert_eq!(result.trace.final_plan_count, 2);
    assert_eq!(
        result
            .solution
            .kpi
            .details
            .get(Csp1dKpiKeys::ColumnGenerationTerminationReason),
        Some(&"IterationLimitReached".to_string())
    );
}

#[test]
fn produce_context_registers_variables_constraints_and_objective() {
    let input = ProduceInput {
        cutting_plans: vec![cutting_plan("plan-1")],
        demands: vec![demand()],
        materials: vec![material()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    };
    let mut context = Csp1dProduceContextBuilder::new(input)
        .mode(Csp1dModelingMode::MILP)
        .is_final_milp(true)
        .build()
        .unwrap();
    let mut model = MetaModel::<f64>::new("csp1d_context_register");
    context.register(&mut model).unwrap();

    assert_eq!(context.produce.plan_variable_indices().len(), 1);
    assert_eq!(model.tokens().len(), 1);
    assert_eq!(model.constraints().len(), 4);
    assert!(
        model
            .constraints()
            .iter()
            .any(|constraint| constraint.args.as_deref().unwrap_or_default().starts_with("product-demand:"))
    );
    assert!(
        model
            .constraints()
            .iter()
            .any(|constraint| constraint.args.as_deref().unwrap_or_default().starts_with("material-usage:"))
    );
    assert!(!model.objective().sub_objectives.is_empty());
    assert!(
        !model.objective().sub_objectives[0]
            .polynomial
            .monomials()
            .is_empty()
    );
}

#[test]
fn shadow_price_key_roundtrips_and_lifecycle_extracts_duals() {
    let key = Csp1dShadowPriceKey::MaterialUsage(MaterialUsageShadowPriceKey {
        material_id: "m1".into(),
    });
    let serialized = shadow_price_key_to_string(&key);
    assert_eq!(serialized, "material-usage:m1");
    assert_eq!(shadow_price_key_from_string(&serialized), Some(key.clone()));
    assert_eq!(
        shadow_price_key_from_string("materialUsage|m1"),
        Some(key.clone()),
        "legacy Rust pipe format remains readable while emitting Kotlin names"
    );

    let input = ProduceInput {
        cutting_plans: vec![cutting_plan("plan-1")],
        demands: vec![demand()],
        materials: vec![material()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    };
    let mut context = Csp1dProduceContextBuilder::new(input).build().unwrap();
    let mut model = MetaModel::<f64>::new("csp1d_shadow_price");
    context.register(&mut model).unwrap();
    let duals = vec![1.5; model.constraints().len()];
    let mut lifecycle = Csp1dShadowPriceLifecycle::new(0.0);
    let shadow_prices = lifecycle.extract_from_dual_solution(&model, &duals);

    assert!(shadow_prices.contains_key(&key));
    assert!(shadow_prices.contains_key(&Csp1dShadowPriceKey::ProductDemand(
        ProductDemandShadowPriceKey {
            product_id: "p1".into(),
            unit_symbol: "m".into(),
        }
    )));
}

#[test]
fn iterative_context_extracts_shadow_price_map_via_cg_pipelines() {
    let input = ProduceInput {
        cutting_plans: vec![cutting_plan("plan-1")],
        demands: vec![demand()],
        materials: vec![material()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    };
    let mut context = Csp1dProduceContextBuilder::new(input)
        .mode(Csp1dModelingMode::LP)
        .build()
        .unwrap();
    let mut model = MetaModel::<f64>::new("csp1d_iterative_shadow_price");
    context.register(&mut model).unwrap();
    let duals = model
        .constraints()
        .iter()
        .map(|constraint| match constraint.args.as_deref() {
            Some(value) if value.starts_with("product-demand:") => 2.0,
            Some(value) if value.starts_with("material-usage:") => 0.25,
            Some(value) if value.starts_with("machine-batch:") => 0.5,
            Some(value) if value.starts_with("machine-capacity:") => 0.75,
            _ => 0.0,
        })
        .collect::<Vec<_>>();

    let shadow_prices = context.extract_shadow_price(&model, &duals).unwrap();

    assert_eq!(
        shadow_prices
            .get(&Csp1dShadowPriceKey::ProductDemand(
                ProductDemandShadowPriceKey {
                    product_id: "p1".into(),
                    unit_symbol: "m".into(),
                }
            ))
            .copied(),
        Some(2.0)
    );
    assert_eq!(
        shadow_prices
            .get(&Csp1dShadowPriceKey::MaterialUsage(
                MaterialUsageShadowPriceKey {
                    material_id: "m1".into(),
                }
            ))
            .copied(),
        Some(0.25)
    );
}

#[test]
fn milp_solver_surface_registers_context_and_extracts_results() {
    let input = ProduceInput {
        cutting_plans: vec![cutting_plan("solver-plan")],
        demands: vec![demand()],
        materials: vec![material()],
        machines: vec![machine()],
        warm_start_plan_usages: vec![ospf_rust_framework_csp1d::CuttingPlanUsage {
            plan: cutting_plan("solver-plan"),
            amount: 1,
        }],
    };

    let result = Csp1dMilpSolver::new()
        .solve(input, None, None, None, Vec::new(), Vec::new(), false)
        .expect("MILP solver surface should return heuristic result");

    assert_eq!(result.produce.cutting_plans.len(), 1);
    assert!(!result.model.tokens().is_empty());
    assert!(!result.solution_by_id.is_empty());
}

#[test]
fn milp_solver_derives_default_length_bounds_for_dynamic_product_ids() {
    let mut product = dynamic_product();
    product.id = "p-default-derive".into();
    product.max_over_produce_length = Some(quantity(5.0));
    let demand = ProductDemand {
        product: product.clone(),
        quantity: quantity(3.0),
        mode: Some(DemandMode::Roll),
    };
    let plan = CuttingPlan {
        id: "plan-default-derive".into(),
        material: material(),
        machine_id: Some("mc1".into()),
        slices: vec![CuttingPlanSlice {
            production: CuttingPlanProduction::Product(product.clone()),
            width: quantity(10.0),
            amount: 1,
        }],
        demand_contributions: vec![CuttingPlanDemandContribution {
            product: product.clone(),
            quantity: quantity(1.0),
        }],
        capacity_consumption: Some(quantity(1.0)),
    };
    let mut length_config = LengthAssignmentModelingConfig::default();
    length_config
        .dynamic_product_ids
        .insert(product.id.clone());
    let input = ProduceInput {
        cutting_plans: vec![plan],
        demands: vec![demand],
        materials: vec![material()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    };

    let result = Csp1dMilpSolver::new()
        .solve(input, None, None, Some(length_config), Vec::new(), Vec::new(), false)
        .expect("MILP solver should derive default assigned-length bounds");

    assert!(
        result
            .model
            .tokens()
            .iter()
            .any(|token| token.name().contains("assigned_length_0"))
    );
    let length_result = result
        .length_result
        .expect("length result should be extracted");
    assert_eq!(length_result.assignments.len(), 1);
    assert_eq!(length_result.assignments[0].product.id, product.id);
}

#[test]
fn milp_solver_surface_solves_lp_shadow_price_shape() {
    let input = ProduceInput {
        cutting_plans: vec![cutting_plan("lp-solver-plan")],
        demands: vec![demand()],
        materials: vec![material()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    };

    let result = Csp1dMilpSolver::new()
        .solve_lp(input, Vec::new())
        .expect("LP solver surface should return heuristic shadow prices");

    assert!(!result.model.constraints().is_empty());
    assert_eq!(result.dual_solution.len(), result.model.constraints().len());
    assert!(!result.shadow_prices.is_empty());
    assert!(!result.framework_shadow_price_map.prices.is_empty());
}

#[test]
fn cg_pipeline_extractors_compute_plan_shadow_price_contribution() {
    let input = ProduceInput {
        cutting_plans: vec![cutting_plan("plan-1")],
        demands: vec![demand()],
        materials: vec![material()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    };
    let mut context = Csp1dProduceContextBuilder::new(input)
        .mode(Csp1dModelingMode::LP)
        .build()
        .unwrap();
    let mut model = MetaModel::<f64>::new("csp1d_cg_pipeline_extractors");
    context.register(&mut model).unwrap();
    let duals = model
        .constraints()
        .iter()
        .map(|constraint| match constraint.args.as_deref() {
            Some(value) if value.starts_with("product-demand:") => 2.0,
            Some(value) if value.starts_with("material-usage:") => 0.25,
            Some(value) if value.starts_with("machine-batch:") => 0.5,
            Some(value) if value.starts_with("machine-capacity:") => 0.75,
            _ => 0.0,
        })
        .collect::<Vec<_>>();
    let mut lifecycle = Csp1dShadowPriceLifecycle::with_pipelines(
        0.0,
        context.cg_pipelines.clone(),
    );
    lifecycle
        .try_extract_from_dual_solution(&model, &duals)
        .unwrap();

    let benefit = lifecycle
        .plan_shadow_price(&context.produce.cutting_plans()[0])
        .expect("plan shadow price should be computable");

    assert_eq!(context.cg_pipelines.len(), 3);
    assert!((benefit - 5.5).abs() < 1e-9);
    assert!(
        context
            .cg_pipelines
            .iter()
            .all(|pipeline| pipeline.shadow_price_extractor().is_some())
    );
}

#[test]
fn column_generation_pricing_receives_context_extracted_shadow_prices() {
    let observed = Arc::new(AtomicBool::new(false));
    let service = Csp1dColumnGeneration::with_generators(
        Box::new(FixedPlanEnumerator {
            plans: vec![cutting_plan("initial")],
        }),
        Box::new(InspectingPricingGenerator {
            observed_demand_shadow_price: observed.clone(),
        }),
    );
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand())
            .configuration(Csp1dConfiguration {
                max_initial_plans: 1,
                max_pricing_plans: 1,
                iteration_limit: 1,
            });
    });

    let result = service.solve_with_trace(problem, None);

    assert_eq!(result.trace.priced_plan_count, vec![0]);
    assert!(observed.load(Ordering::SeqCst));
}

#[test]
fn produce_context_registers_yield_waste_length_pipelines() {
    let mut short_demand = demand();
    short_demand.quantity = quantity(1.0);
    let demand_key = ProductDemandShadowPriceKey {
        product_id: short_demand.product.id.clone(),
        unit_symbol: shadow_price_unit_symbol(&short_demand.quantity.unit),
    };
    let mut yield_config = YieldModelingConfig::default();
    yield_config.under_production_penalty.insert(demand_key.clone(), 7.0);
    yield_config.over_production_penalty.insert(demand_key.clone(), 5.0);
    yield_config.over_production_upper_bound.insert(demand_key, 2.0);
    let mut material_cost_penalty = BTreeMap::new();
    material_cost_penalty.insert("m1".to_string(), 3.0);
    let mut length_config = LengthAssignmentModelingConfig::default();
    length_config.dynamic_product_ids.insert("p-dyn".to_string());
    length_config
        .assigned_length_lower_bound
        .insert("p-dyn".to_string(), 1.0);
    length_config
        .assigned_length_upper_bound
        .insert("p-dyn".to_string(), 5.0);
    length_config
        .over_length_upper_bound
        .insert("p-dyn".to_string(), 4.0);
    length_config
        .over_length_penalty
        .insert("p-dyn".to_string(), 11.0);
    length_config.total_length_penalty = Some(13.0);
    length_config.batch_min_penalty = Some(0.5);
    let waste_config = WasteMinimizationConfig {
        trim_width_penalty: Some(0.25),
        material_cost_penalty,
        over_production_area_penalty: Some(0.75),
        rest_material_penalty: Some(0.5),
        ..WasteMinimizationConfig::default()
    };
    let input = ProduceInput {
        cutting_plans: vec![over_producing_plan("plan-pipelines")],
        demands: vec![short_demand, dynamic_demand()],
        materials: vec![material_with_length()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    };
    let mut context = Csp1dProduceContextBuilder::new(input)
        .mode(Csp1dModelingMode::MILP)
        .is_final_milp(true)
        .yield_config(yield_config)
        .waste_config(waste_config.clone())
        .length_config(length_config)
        .build()
        .unwrap();
    let mut model = MetaModel::<f64>::new("csp1d_domain_pipelines");
    context.register(&mut model).unwrap();

    assert!(model.tokens().len() >= 5);
    assert!(
        model
            .tokens()
            .iter()
            .any(|token| token.name().contains("under_production_0"))
    );
    assert!(
        model
            .tokens()
            .iter()
            .any(|token| token.name().contains("over_production_0"))
    );
    assert!(
        model
            .tokens()
            .iter()
            .any(|token| token.name().contains("assigned_length_1"))
    );
    assert!(
        model
            .tokens()
            .iter()
            .any(|token| token.name().contains("over_length_1"))
    );
    assert!(
        model
            .constraints()
            .iter()
            .any(|constraint| constraint.name.contains("over_production_bound_0")
                && constraint.args.as_deref().unwrap_or_default().starts_with("yield-over-production-bound:"))
    );
    assert!(
        model
            .constraints()
            .iter()
            .any(|constraint| constraint.name.contains("assigned_over_length_link_1"))
    );
    let objective_terms = model.objective().sub_objectives[0]
        .polynomial
        .monomials()
        .len();
    assert!(objective_terms >= 7);
    let yield_terms = YieldObjectivePipeline::new(
        context.r#yield.as_ref().expect("yield slack").clone(),
    )
    .objective_terms();
    assert_eq!(yield_terms.len(), 2);
    let waste_terms = WasteObjectivePipeline::new(
        context.produce.clone(),
        waste_config,
        context.r#yield.clone(),
    )
    .objective_terms();
    assert!(!waste_terms.is_empty());
    let length_pipeline = LengthObjectivePipeline::new(
        context.length.as_ref().expect("length slack").clone(),
    );
    assert_eq!(length_pipeline.batch_coefficient(), 1.5);
    assert_eq!(length_pipeline.objective_terms().len(), 2);
}

#[test]
fn yield_and_length_results_prefer_solver_slack_values() {
    let mut short_demand = demand();
    short_demand.quantity = quantity(1.0);
    let demand_key = ProductDemandShadowPriceKey {
        product_id: short_demand.product.id.clone(),
        unit_symbol: shadow_price_unit_symbol(&short_demand.quantity.unit),
    };
    let mut yield_config = YieldModelingConfig::default();
    yield_config.under_production_penalty.insert(demand_key.clone(), 7.0);
    yield_config.over_production_penalty.insert(demand_key, 5.0);
    let mut length_config = LengthAssignmentModelingConfig::default();
    length_config.dynamic_product_ids.insert("p-dyn".to_string());
    length_config.total_length_penalty = Some(1.0);
    length_config
        .over_length_penalty
        .insert("p-dyn".to_string(), 2.0);
    let input = ProduceInput {
        cutting_plans: vec![over_producing_plan("plan-slack")],
        demands: vec![short_demand, dynamic_demand()],
        materials: vec![material_with_length()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    };
    let mut context = Csp1dProduceContextBuilder::new(input)
        .mode(Csp1dModelingMode::MILP)
        .yield_config(yield_config)
        .length_config(length_config)
        .build()
        .unwrap();
    let mut model = MetaModel::<f64>::new("csp1d_slack_extract");
    context.register(&mut model).unwrap();
    let mut solution = HashMap::new();
    let plan_variable = context
        .produce
        .plan_variable_index(0)
        .expect("plan variable should exist");
    solution.insert(model.tokens()[plan_variable].id(), 1.0);
    let under_variable = context
        .r#yield
        .as_ref()
        .expect("yield aggregation")
        .under_production[0]
        .expect("under slack");
    let over_variable = context
        .r#yield
        .as_ref()
        .expect("yield aggregation")
        .over_production[0]
        .expect("over slack");
    solution.insert(model.tokens()[under_variable].id(), 4.0);
    solution.insert(model.tokens()[over_variable].id(), 6.0);
    let assigned_variable = context
        .length
        .as_ref()
        .expect("length aggregation")
        .assigned_length[1]
        .expect("assigned length");
    let over_length_variable = context
        .length
        .as_ref()
        .expect("length aggregation")
        .over_length[1]
        .expect("over length");
    solution.insert(model.tokens()[assigned_variable].id(), 8.0);
    solution.insert(model.tokens()[over_length_variable].id(), 3.0);
    model.set_solution_by_id(&solution);

    let yield_result = context
        .extract_yield_result(&model)
        .expect("yield result should be extracted");
    assert_eq!(yield_result.analysis.under_productions.len(), 1);
    assert_eq!(yield_result.analysis.under_productions[0].shortfall.value, 4.0);
    assert_eq!(yield_result.analysis.over_productions.len(), 1);
    assert_eq!(yield_result.analysis.over_productions[0].surplus.value, 6.0);
    let length_result = context
        .extract_length_result(&model)
        .expect("length result should be extracted");
    assert_eq!(length_result.assignments.len(), 1);
    assert_eq!(length_result.assignments[0].assigned_length.value, 8.0);
    assert_eq!(length_result.over_length_records.len(), 1);
    assert_eq!(length_result.over_length_records[0].over_length.value, 3.0);
}

#[test]
fn add_columns_refreshes_builtin_constraints_and_full_objective() {
    let mut material_cost_penalty = BTreeMap::new();
    material_cost_penalty.insert("m1".to_string(), 3.0);
    let waste_config = WasteMinimizationConfig {
        trim_width_penalty: Some(0.25),
        material_cost_penalty,
        ..WasteMinimizationConfig::default()
    };
    let input = ProduceInput {
        cutting_plans: vec![cutting_plan("plan-1")],
        demands: vec![demand()],
        materials: vec![material()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    };
    let mut context = Csp1dProduceContextBuilder::new(input)
        .mode(Csp1dModelingMode::LP)
        .waste_config(waste_config)
        .build()
        .unwrap();
    let mut model = MetaModel::<f64>::new("csp1d_add_columns_refresh");
    context.register(&mut model).unwrap();

    let initial_constraint_count = model.constraints().len();
    let initial_objective_count = model.objective().sub_objectives.len();
    let initial_objective_terms = model.objective().sub_objectives[0]
        .polynomial
        .monomials()
        .len();

    let mut new_plan = cutting_plan("plan-2");
    new_plan.slices[0].width = quantity(20.0);
    new_plan.demand_contributions[0].quantity = quantity(1.0);
    let added = context.add_columns(1, vec![new_plan], &mut model).unwrap();

    assert_eq!(added.len(), 1);
    assert_eq!(context.produce.plan_count(), 2);
    assert_eq!(context.produce.plan_variable_indices().len(), 2);
    assert_eq!(model.constraints().len(), initial_constraint_count);
    assert_eq!(model.objective().sub_objectives.len(), initial_objective_count);
    assert_eq!(
        model.objective().sub_objectives[0]
            .polynomial
            .monomials()
            .len(),
        initial_objective_terms + 2
    );

    let new_variable = context
        .produce
        .plan_variable_index(1)
        .expect("new plan variable should be registered");
    let demand_constraint = model
        .constraints()
        .iter()
        .find(|constraint| constraint.name == "demand_0")
        .expect("demand constraint should be rebuilt");
    assert_eq!(demand_constraint.inequality.relation, ConstraintRelation::GreaterEqual);
    assert!(
        demand_constraint
            .inequality
            .polynomial
            .monomials()
            .iter()
            .any(|monomial| monomial.var_index() == new_variable
                && (*monomial.coefficient() - 1.0).abs() < 1e-9)
    );
    let objective = &model.objective().sub_objectives[0].polynomial;
    assert!(
        objective
            .monomials()
            .iter()
            .any(|monomial| monomial.var_index() == new_variable
                && (*monomial.coefficient() - 1.0).abs() < 1e-9)
    );
    assert!(
        objective
            .monomials()
            .iter()
            .any(|monomial| monomial.var_index() == new_variable
                && (*monomial.coefficient() - 18.0).abs() < 1e-9)
    );
}

#[test]
fn add_columns_invokes_incremental_extension_pipelines() {
    let input = ProduceInput {
        cutting_plans: vec![cutting_plan("plan-1")],
        demands: vec![demand()],
        materials: vec![material()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    };
    let mut builder = Csp1dProduceContextBuilder::new(input);
    builder
        .mode(Csp1dModelingMode::LP)
        .incremental_pipeline(Arc::new(CountingIncrementalPipeline::new()));
    let mut context = builder.build().unwrap();
    let mut model = MetaModel::<f64>::new("csp1d_incremental_pipeline");
    context.register(&mut model).unwrap();

    assert!(
        model
            .constraints()
            .iter()
            .any(|constraint| constraint.name == "incremental_initial")
    );
    let mut new_plan = cutting_plan("plan-incremental");
    new_plan.slices[0].width = quantity(20.0);
    let added = context.add_columns(1, vec![new_plan], &mut model).unwrap();

    assert_eq!(added.len(), 1);
    assert!(
        model
            .constraints()
            .iter()
            .any(|constraint| constraint.name == "incremental_added_1"
                && constraint.group.as_ref().map(|group| group.id) == Some(90_001))
    );
}

#[test]
fn remove_columns_retires_plan_variables_and_refreshes_builtin_model() {
    let mut second_plan = cutting_plan("plan-remove");
    second_plan.slices[0].width = quantity(20.0);
    second_plan.demand_contributions[0].quantity = quantity(1.0);
    let input = ProduceInput {
        cutting_plans: vec![cutting_plan("plan-keep"), second_plan],
        demands: vec![demand()],
        materials: vec![material()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    };
    let mut context = Csp1dProduceContextBuilder::new(input)
        .mode(Csp1dModelingMode::LP)
        .build()
        .unwrap();
    let mut model = MetaModel::<f64>::new("csp1d_remove_columns");
    context.register(&mut model).unwrap();
    let removed_variable = context
        .produce
        .plan_variable_index(1)
        .expect("second plan variable should exist");

    let removed = context.remove_columns(&[1], &mut model).unwrap();

    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].id, "plan-remove");
    assert!(context.produce.retired_plan_indices().contains(&1));
    assert_eq!(
        model.variable_range_by_index(removed_variable),
        Some(ospf_rust_core::variable::VariableRange::fixed(0.0))
    );
    let demand_constraint = model
        .constraints()
        .iter()
        .find(|constraint| constraint.name == "demand_0")
        .expect("demand constraint should be rebuilt");
    assert!(
        !demand_constraint
            .inequality
            .polynomial
            .monomials()
            .iter()
            .any(|monomial| monomial.var_index() == removed_variable)
    );
    assert!(
        !model.objective().sub_objectives[0]
            .polynomial
            .monomials()
            .iter()
            .any(|monomial| monomial.var_index() == removed_variable)
    );

    let mut solution = HashMap::new();
    solution.insert(model.tokens()[removed_variable].id(), 1.0);
    model.set_solution_by_id(&solution);
    let produce = context.extract_solution(&model).unwrap();
    assert!(produce.cutting_plans.is_empty());
}

#[test]
fn simple_initial_generator_applies_width_check_and_candidate_filters() {
    let width_check: Csp1dWidthFeasibilityCheck<f64> = Arc::new(|_material, _product, width| {
        width.value == 30.0
    });
    let filter: Csp1dCandidateFilter<f64> = Arc::new(|plan, _existing| {
        plan.slices.iter().any(|slice| slice.amount == 1)
    });
    let input = CuttingPlanGenerationInput {
        products: vec![product()],
        materials: vec![material()],
        machines: vec![machine()],
        costars: Vec::new(),
        demands: vec![demand()],
        existing_plans: Vec::new(),
        domain_policies: Vec::new(),
        generation_strategies: Vec::new(),
        candidate_filters: vec![filter],
        width_feasibility_check: Some(width_check),
        canonical_key_overrides: Vec::new(),
        dominance_accept_overrides: Vec::new(),
    };

    let plans = SimpleInitialCuttingPlanGenerator.generate(&input);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].slices[0].width.value, 30.0);
}

#[test]
fn nsame_generator_honors_amount_and_knife_constraints() {
    let mut narrow_product = product();
    narrow_product.width = vec![quantity(20.0)];
    let demand = ProductDemand {
        product: narrow_product.clone(),
        quantity: quantity(1.0),
        mode: Some(DemandMode::Roll),
    };
    let input = CuttingPlanGenerationInput {
        products: vec![narrow_product],
        materials: vec![material()],
        machines: vec![machine()],
        costars: Vec::new(),
        demands: vec![demand],
        existing_plans: Vec::new(),
        domain_policies: Vec::new(),
        generation_strategies: Vec::new(),
        candidate_filters: vec![Arc::new(|_plan: &CuttingPlan<f64>, _existing| true)],
        width_feasibility_check: None,
        canonical_key_overrides: Vec::new(),
        dominance_accept_overrides: Vec::new(),
    };
    let generator = NSameGenerator::with_constraints(GenerationConstraints {
        max_knife_count: Some(3),
        min_knife_count: Some(2),
        ..GenerationConstraints::default()
    })
    .with_all_amount(true);

    let report = generator.generate_with_report(&input);

    assert_eq!(report.plans.len(), 2);
    let amounts = report
        .plans
        .iter()
        .map(|plan| plan.slices[0].amount)
        .collect::<Vec<_>>();
    assert_eq!(amounts, vec![2, 3]);
    assert_eq!(report.statistics.accepted_plans, 2);
    assert!(report.statistics.knife_bound_pruned_nodes >= 1);
}

#[test]
fn dfs_nsum_and_fullsum_generators_emit_multi_product_plans() {
    let mut second_product = product();
    second_product.id = "p2".into();
    second_product.name = "Product 2".into();
    second_product.width = vec![quantity(20.0)];
    let second_demand = ProductDemand {
        product: second_product.clone(),
        quantity: quantity(1.0),
        mode: Some(DemandMode::Roll),
    };
    let input = CuttingPlanGenerationInput {
        products: vec![product(), second_product],
        materials: vec![material()],
        machines: vec![machine()],
        costars: Vec::new(),
        demands: vec![demand(), second_demand],
        existing_plans: Vec::new(),
        domain_policies: Vec::new(),
        generation_strategies: Vec::new(),
        candidate_filters: Vec::new(),
        width_feasibility_check: None,
        canonical_key_overrides: Vec::new(),
        dominance_accept_overrides: Vec::new(),
    };

    let generators: Vec<Box<dyn Csp1dInitialCuttingPlanGenerator<f64>>> = vec![
        Box::new(DFSGenerator::default().with_max_plans(8)),
        Box::new(NSumGenerator::default().with_max_depth(2).with_max_plans(8)),
        Box::new(FullSumGenerator::default().with_max_plans(8)),
    ];

    for generator in generators {
        let report = generator.generate_with_report(&input);
        assert!(report.statistics.visited_nodes >= 1);
        assert!(report.plans.iter().any(|plan| plan.slices.len() >= 2));
    }
}

#[test]
fn combination_generators_report_width_index_and_quantity_cache_hits() {
    let mut second_material = material();
    second_material.id = "m2".into();
    second_material.name = "Material 2".into();
    let mut cache_product = product();
    cache_product.width = vec![quantity(20.0), quantity(30.0)];
    let cache_demand = ProductDemand {
        product: cache_product.clone(),
        quantity: quantity(1.0),
        mode: Some(DemandMode::Roll),
    };
    let input = CuttingPlanGenerationInput {
        products: vec![cache_product],
        materials: vec![material(), second_material],
        machines: vec![machine()],
        costars: Vec::new(),
        demands: vec![cache_demand],
        existing_plans: Vec::new(),
        domain_policies: Vec::new(),
        generation_strategies: Vec::new(),
        candidate_filters: vec![Arc::new(|_plan: &CuttingPlan<f64>, _existing| true)],
        width_feasibility_check: None,
        canonical_key_overrides: Vec::new(),
        dominance_accept_overrides: Vec::new(),
    };
    let generator = DFSGenerator::with_constraints(GenerationConstraints {
        max_knife_count: Some(1),
        ..GenerationConstraints::default()
    })
    .with_max_plans(8);

    let report = generator.generate_with_report(&input);

    assert_eq!(report.statistics.material_width_index_cache_hits, 1);
    assert!(report.statistics.quantity_cache_hits >= 1);
    assert!(report.statistics.quantity_cache_misses >= 1);
}

#[test]
fn combination_generators_reuse_slice_templates_for_equivalent_material_widths() {
    let mut second_material = material();
    second_material.id = "m2".into();
    second_material.name = "Material 2".into();
    let input = CuttingPlanGenerationInput {
        products: vec![product()],
        materials: vec![material(), second_material],
        machines: vec![machine()],
        costars: Vec::new(),
        demands: vec![demand()],
        existing_plans: Vec::new(),
        domain_policies: Vec::new(),
        generation_strategies: Vec::new(),
        candidate_filters: Vec::new(),
        width_feasibility_check: None,
        canonical_key_overrides: Vec::new(),
        dominance_accept_overrides: Vec::new(),
    };
    let generator = DFSGenerator::with_constraints(GenerationConstraints {
        max_knife_count: Some(1),
        ..GenerationConstraints::default()
    })
    .with_max_plans(8);

    let report = generator.generate_with_report(&input);

    assert_eq!(report.statistics.material_slice_template_cache_misses, 1);
    assert_eq!(report.statistics.material_slice_template_cache_hits, 1);
    assert!(report.plans.iter().any(|plan| plan.material.id == "m2"));
    assert!(report
        .plans
        .iter()
        .filter(|plan| plan.material.id == "m2")
        .all(|plan| plan.demand_contributions[0].quantity.value == 1.0));
}

#[test]
fn generator_dominance_override_receives_accepted_plans_like_kotlin_collector() {
    let mut narrow_product = product();
    narrow_product.width = vec![quantity(20.0)];
    let demand = ProductDemand {
        product: narrow_product.clone(),
        quantity: quantity(1.0),
        mode: Some(DemandMode::Roll),
    };
    let input = CuttingPlanGenerationInput {
        products: vec![narrow_product],
        materials: vec![material()],
        machines: vec![machine()],
        costars: Vec::new(),
        demands: vec![demand],
        existing_plans: Vec::new(),
        domain_policies: Vec::new(),
        generation_strategies: Vec::new(),
        candidate_filters: Vec::new(),
        width_feasibility_check: None,
        canonical_key_overrides: Vec::new(),
        dominance_accept_overrides: vec![Arc::new(
            |_plan: &CuttingPlan<f64>, accepted: &[CuttingPlan<f64>]| accepted.is_empty(),
        )],
    };
    let generator = NSameGenerator::with_constraints(GenerationConstraints {
        max_knife_count: Some(2),
        ..GenerationConstraints::default()
    })
    .with_all_amount(true)
    .with_max_plans(8);

    let report = generator.generate_with_report(&input);

    assert_eq!(report.plans.len(), 1);
    assert_eq!(report.plans[0].slices[0].amount, 1);
    assert_eq!(report.statistics.dominated_candidates, 1);
}

#[test]
fn generator_dominance_pruning_replaces_same_contribution_worse_rest_width() {
    let mut narrow_product = product();
    narrow_product.width = vec![quantity(20.0), quantity(30.0)];
    let demand = ProductDemand {
        product: narrow_product.clone(),
        quantity: quantity(1.0),
        mode: Some(DemandMode::Roll),
    };
    let input = CuttingPlanGenerationInput {
        products: vec![narrow_product],
        materials: vec![material()],
        machines: vec![machine()],
        costars: Vec::new(),
        demands: vec![demand],
        existing_plans: Vec::new(),
        domain_policies: Vec::new(),
        generation_strategies: Vec::new(),
        candidate_filters: Vec::new(),
        width_feasibility_check: None,
        canonical_key_overrides: vec![Arc::new(|plan: &CuttingPlan<f64>| Some(plan.id.clone()))],
        dominance_accept_overrides: Vec::new(),
    };
    let generator = DFSGenerator::with_constraints(GenerationConstraints {
        max_knife_count: Some(1),
        enable_dominance_pruning: true,
        ..GenerationConstraints::default()
    })
    .with_max_plans(8);

    let report = generator.generate_with_report(&input);

    assert_eq!(report.plans.len(), 1);
    assert_eq!(report.plans[0].slices[0].width.value, 30.0);
    assert_eq!(report.statistics.dominated_candidates, 1);
}

#[test]
fn generator_cross_contribution_dominance_counts_relaxed_rejections() {
    let mut narrow_product = product();
    narrow_product.width = vec![quantity(20.0)];
    let demand = ProductDemand {
        product: narrow_product.clone(),
        quantity: quantity(1.0),
        mode: Some(DemandMode::Roll),
    };
    let input = CuttingPlanGenerationInput {
        products: vec![narrow_product],
        materials: vec![material()],
        machines: vec![machine()],
        costars: Vec::new(),
        demands: vec![demand],
        existing_plans: Vec::new(),
        domain_policies: Vec::new(),
        generation_strategies: Vec::new(),
        candidate_filters: Vec::new(),
        width_feasibility_check: None,
        canonical_key_overrides: Vec::new(),
        dominance_accept_overrides: Vec::new(),
    };
    let generator = NSameGenerator::with_constraints(GenerationConstraints {
        max_knife_count: Some(3),
        enable_dominance_pruning: true,
        dominance_strategy: ospf_rust_framework_csp1d::DominanceStrategy::CrossContribution,
        ..GenerationConstraints::default()
    })
    .with_all_amount(true)
    .with_max_plans(8);

    let report = generator.generate_with_report(&input);

    assert_eq!(report.plans.len(), 1);
    assert_eq!(report.plans[0].slices[0].amount, 1);
    assert_eq!(report.statistics.cross_contribution_dominated, 2);
    assert_eq!(report.statistics.dominated_candidates, 2);
}

#[test]
fn costar_filler_adds_costar_slices_without_changing_contributions() {
    let plan = cutting_plan("costar-base");
    let costar = Costar {
        id: "c1".into(),
        name: "Costar 1".into(),
        width: vec![quantity(20.0)],
        length: None,
        unit_weight: None,
    };

    let results = CostarFiller::new().fill(&plan, &[costar.clone()]);

    assert!(results
        .iter()
        .any(|candidate| candidate.slices.iter().any(|slice| {
            matches!(&slice.production, CuttingPlanProduction::Costar(candidate_costar) if candidate_costar.id == costar.id)
        })));
    assert!(results
        .iter()
        .all(|candidate| candidate.demand_contributions.len() == plan.demand_contributions.len()));
    assert!(results.iter().all(|candidate| candidate
        .demand_contributions
        .iter()
        .zip(plan.demand_contributions.iter())
        .all(|(left, right)| left.product.id == right.product.id
            && left.quantity.value == right.quantity.value)));
}

#[test]
fn generation_benchmark_snapshot_uses_kotlin_stable_stop_reason_names() {
    let statistics = CuttingPlanGenerationStatistics {
        visited_nodes: 3,
        generated_candidates: 4,
        accepted_plans: 2,
        duplicate_candidates: 1,
        stop_reason: CuttingPlanGenerationStopReason::MaxPlans,
        ..CuttingPlanGenerationStatistics::default()
    };

    let snapshot = CuttingPlanGenerationBenchmarkSnapshot::from_statistics(
        "NSameGenerator",
        &statistics,
    );
    let stable_line = snapshot.to_stable_line();

    assert!(stable_line.contains("generator=NSameGenerator"));
    assert!(stable_line.contains("visitedNodes=3"));
    assert!(stable_line.contains("duplicateCandidates=1"));
    assert!(stable_line.ends_with("stopReason=MaxPlans"));
}

#[test]
fn merge_generation_reports_matches_kotlin_parallel_merge_semantics() {
    let plan_a = cutting_plan("plan-a");
    let mut plan_b = cutting_plan("plan-b");
    plan_b.slices[0].width = quantity(20.0);
    let mut plan_duplicate = plan_b.clone();
    plan_duplicate.id = "plan-duplicate".into();

    let report_a = CuttingPlanGenerationReport {
        plans: vec![plan_a.clone(), plan_b.clone()],
        statistics: CuttingPlanGenerationStatistics {
            visited_nodes: 3,
            generated_candidates: 4,
            duplicate_candidates: 1,
            dominated_candidates: 2,
            width_bound_pruned_nodes: 5,
            material_width_index_cache_hits: 7,
            quantity_cache_hits: 11,
            quantity_cache_misses: 13,
            stop_reason: CuttingPlanGenerationStopReason::Exhausted,
            ..CuttingPlanGenerationStatistics::default()
        },
    };
    let report_b = CuttingPlanGenerationReport {
        plans: vec![plan_duplicate],
        statistics: CuttingPlanGenerationStatistics {
            visited_nodes: 17,
            generated_candidates: 19,
            duplicate_candidates: 23,
            material_slice_template_cache_misses: 29,
            cross_contribution_dominated: 31,
            stop_reason: CuttingPlanGenerationStopReason::Exhausted,
            ..CuttingPlanGenerationStatistics::default()
        },
    };

    let merged = merge_generation_reports(
        vec![report_a, report_b],
        GenerationReportMergeOptions::new(8)
            .with_started_at(Instant::now() - Duration::from_millis(3))
            .with_canonical_key_override(Arc::new(|plan: &CuttingPlan<f64>| {
                Some(if plan.slices[0].width.value == 20.0 {
                    "same-width".to_string()
                } else {
                    plan.id.clone()
                })
            })),
    );

    assert_eq!(merged.plans.len(), 2);
    assert_eq!(merged.statistics.accepted_plans, 2);
    assert_eq!(merged.statistics.visited_nodes, 20);
    assert_eq!(merged.statistics.generated_candidates, 23);
    assert_eq!(merged.statistics.duplicate_candidates, 25);
    assert_eq!(merged.statistics.cross_worker_duplicate_candidates, 1);
    assert_eq!(merged.statistics.dominated_candidates, 2);
    assert_eq!(merged.statistics.width_bound_pruned_nodes, 5);
    assert_eq!(merged.statistics.material_width_index_cache_hits, 7);
    assert_eq!(merged.statistics.quantity_cache_hits, 11);
    assert_eq!(merged.statistics.quantity_cache_misses, 13);
    assert_eq!(merged.statistics.material_slice_template_cache_misses, 29);
    assert_eq!(merged.statistics.cross_contribution_dominated, 31);
    assert!(merged.statistics.elapsed_milliseconds >= 1);
    assert_eq!(
        merged.statistics.stop_reason,
        CuttingPlanGenerationStopReason::Exhausted
    );

    let limited = merge_generation_reports(
        vec![CuttingPlanGenerationReport {
            plans: vec![plan_a, plan_b],
            statistics: CuttingPlanGenerationStatistics::default(),
        }],
        GenerationReportMergeOptions::new(1),
    );
    assert_eq!(limited.plans.len(), 1);
    assert_eq!(
        limited.statistics.stop_reason,
        CuttingPlanGenerationStopReason::MaxPlans
    );

    let timed_out = merge_generation_reports(
        vec![CuttingPlanGenerationReport::<f64> {
            plans: Vec::new(),
            statistics: CuttingPlanGenerationStatistics::default(),
        }],
        GenerationReportMergeOptions::new(8)
            .with_deadline(Some(Instant::now() - Duration::from_millis(1))),
    );
    assert_eq!(
        timed_out.statistics.stop_reason,
        CuttingPlanGenerationStopReason::Timeout
    );
}

#[test]
fn reduced_cost_pricing_filters_existing_and_non_improving_candidates() {
    let mut high_value = cutting_plan("high-value");
    high_value.demand_contributions[0].quantity = quantity(3.0);
    let mut low_value = cutting_plan("low-value");
    low_value.slices[0].width = quantity(20.0);
    let duplicate_existing = cutting_plan("duplicate-existing");
    let pricing = ReducedCostPricingGenerator::new(FixedPlanEnumerator {
        plans: vec![low_value.clone(), high_value.clone(), duplicate_existing.clone()],
    });
    let demand_key = Csp1dShadowPriceKey::ProductDemand(ProductDemandShadowPriceKey {
        product_id: "p1".into(),
        unit_symbol: "m".into(),
    });
    let mut input = Csp1dPricingInput::default();
    input.generation_input = CuttingPlanGenerationInput {
        products: vec![product()],
        materials: vec![material()],
        machines: vec![machine()],
        costars: Vec::new(),
        demands: vec![demand()],
        existing_plans: vec![duplicate_existing],
        domain_policies: Vec::new(),
        generation_strategies: Vec::new(),
        candidate_filters: Vec::new(),
        width_feasibility_check: None,
        canonical_key_overrides: vec![Arc::new(|plan: &CuttingPlan<f64>| {
            plan.id
                .contains("duplicate")
                .then(|| "duplicate-group".to_string())
        })],
        dominance_accept_overrides: Vec::new(),
    };
    input.shadow_prices.insert(demand_key, 0.5);
    input.max_generated_plans = 1;

    let report = pricing.generate_with_report(&input);
    assert_eq!(report.plans.len(), 1);
    assert_eq!(report.plans[0].id, "high-value");
    assert_eq!(report.statistics.accepted_plans, 1);
}

#[test]
fn warm_start_plan_pool_adapter_extracts_previous_solution_usages() {
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand());
    });
    let previous = Csp1dColumnGeneration::default()
        .solve_with_trace(problem.clone(), None)
        .solution;
    assert!(!previous.produce.cutting_plans.is_empty());
    let warm_start = Csp1dWarmStart {
        cutting_plans: Vec::new(),
        previous_solution: Some(previous.clone()),
    };
    let adapter = Csp1dWarmStartPlanPoolAdapter {
        append_fallback_plans: false,
    };
    let result = adapter.apply(Csp1dWarmStartAdapterInput {
        problem: Some(problem),
        solve_config: None,
        warm_start: Some(warm_start),
        cutting_plans: previous.generated_plans.clone(),
    });

    assert!(result.initial_generator.is_some());
    assert_eq!(result.applied_plan_count, previous.generated_plans.len() as i64);
    assert_eq!(result.applied_usage_count, previous.produce.cutting_plans.len() as i64);
    assert_eq!(result.initial_plan_usages.len(), previous.produce.cutting_plans.len());
}

#[test]
fn recovery_applies_previous_solution_warm_start_trace() {
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand());
    });
    let previous = Csp1dColumnGeneration::default()
        .solve_with_trace(problem.clone(), None)
        .solution;
    let recovery = Csp1dRecovery::with_warm_start_adapter(
        Csp1dColumnGeneration::default(),
        Box::new(Csp1dWarmStartPlanPoolAdapter {
            append_fallback_plans: false,
        }),
    );
    let result = recovery
        .solve_with_trace(Csp1dRecoveryInput {
            problem: Some(problem),
            solve_config: None,
            warm_start: Some(Csp1dWarmStart {
                cutting_plans: Vec::new(),
                previous_solution: Some(previous.clone()),
            }),
            options: Default::default(),
        })
        .unwrap();

    assert_eq!(result.trace.status, Csp1dRecoveryStatus::Solved);
    assert_eq!(result.trace.warm_start_status, Csp1dWarmStartStatus::Applied);
    assert_eq!(
        result.trace.applied_warm_start_usage_count,
        previous.produce.cutting_plans.len() as i64
    );
    assert_eq!(result.solution.status, Csp1dSolutionStatus::Feasible);
    assert!(!result.solution.produce.cutting_plans.is_empty());
}

#[test]
fn default_recovery_reports_unsupported_adapter_when_fallback_is_disabled() {
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand());
    });
    let error = Csp1dRecovery::default()
        .solve_with_trace(Csp1dRecoveryInput {
            problem: Some(problem),
            solve_config: None,
            warm_start: Some(Csp1dWarmStart {
                cutting_plans: vec![cutting_plan("warm-start")],
                previous_solution: None,
            }),
            options: Csp1dRecoveryOptions {
                retry_without_warm_start: false,
            },
        })
        .unwrap_err();

    let Csp1dError::RecoveryFallbackDisabled { trace, .. } = error else {
        panic!("expected fallback-disabled recovery error");
    };
    assert_eq!(trace.status, Csp1dRecoveryStatus::FallbackDisabled);
    assert_eq!(
        trace.warm_start_status,
        Csp1dWarmStartStatus::AdapterUnsupported
    );
    assert_eq!(trace.attempt_count, 0);
    assert_eq!(trace.warm_start_plan_count, 1);
    assert_eq!(trace.applied_warm_start_plan_count, 0);
    assert_eq!(trace.applied_warm_start_usage_count, 0);
    assert_eq!(
        trace.message.as_deref(),
        Some("Warm start is compatible but current adapter does not support applying it; fallback is disabled")
    );
}

#[test]
fn recovery_invalid_warm_start_reports_fallback_disabled_trace() {
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand());
    });
    let mut invalid_plan = cutting_plan("invalid-warm-start");
    invalid_plan.material.id = "missing-material".into();

    let error = Csp1dRecovery::default()
        .solve_with_trace(Csp1dRecoveryInput {
            problem: Some(problem),
            solve_config: None,
            warm_start: Some(Csp1dWarmStart {
                cutting_plans: vec![invalid_plan],
                previous_solution: None,
            }),
            options: Csp1dRecoveryOptions {
                retry_without_warm_start: false,
            },
        })
        .unwrap_err();

    let Csp1dError::RecoveryFallbackDisabled { trace, .. } = error else {
        panic!("expected fallback-disabled recovery error");
    };
    assert_eq!(trace.status, Csp1dRecoveryStatus::FallbackDisabled);
    assert_eq!(trace.warm_start_status, Csp1dWarmStartStatus::Invalid);
    assert_eq!(trace.attempt_count, 0);
    assert_eq!(trace.warm_start_plan_count, 1);
    assert_eq!(
        trace.message.as_deref(),
        Some("Warm start is incompatible with current problem; fallback is disabled")
    );
}

#[test]
fn recovery_flow_policy_can_disable_default_fallback() {
    let observed_context = Arc::new(AtomicBool::new(false));
    let solve_config = csp1d_solve_config::<f64, _>(|builder| {
        builder.flow_policy(Arc::new(FixedRecoveryFallbackPolicy {
            decision: false,
            observed_context: observed_context.clone(),
        }));
    });
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand())
            .solve_config(solve_config);
    });

    let error = Csp1dRecovery::default()
        .solve_with_trace(Csp1dRecoveryInput {
            problem: Some(problem),
            solve_config: None,
            warm_start: Some(Csp1dWarmStart {
                cutting_plans: vec![cutting_plan("warm-start")],
                previous_solution: None,
            }),
            options: Csp1dRecoveryOptions {
                retry_without_warm_start: true,
            },
        })
        .unwrap_err();

    assert!(matches!(
        error,
        Csp1dError::RecoveryFallbackDisabled { .. }
    ));
    assert!(observed_context.load(Ordering::SeqCst));
}

#[test]
fn column_generation_recovery_applies_plan_pool_warm_start_trace() {
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand());
    });
    let previous = Csp1dColumnGeneration::default()
        .solve_with_trace(problem.clone(), None)
        .solution;
    let recovery = Csp1dColumnGenerationRecovery::with_warm_start_adapter(
        Csp1dColumnGeneration::default(),
        Box::new(Csp1dWarmStartPlanPoolAdapter {
            append_fallback_plans: false,
        }),
    );

    let result = recovery
        .solve_with_trace(Csp1dRecoveryInput {
            problem: Some(problem),
            solve_config: None,
            warm_start: Some(Csp1dWarmStart {
                cutting_plans: Vec::new(),
                previous_solution: Some(previous.clone()),
            }),
            options: Default::default(),
        })
        .unwrap();

    assert_eq!(result.trace.status, Csp1dRecoveryStatus::Solved);
    assert_eq!(result.trace.warm_start_status, Csp1dWarmStartStatus::Applied);
    assert_eq!(
        result.trace.applied_warm_start_plan_count,
        previous.generated_plans.len() as i64
    );
    assert_eq!(
        result.trace.applied_warm_start_usage_count,
        previous.produce.cutting_plans.len() as i64
    );
    assert_eq!(result.solution.status, Csp1dSolutionStatus::Feasible);
}

#[test]
fn column_generation_extracts_yield_waste_length_results_and_kpi_details() {
    let mut material_cost_penalty = BTreeMap::new();
    material_cost_penalty.insert("m1".to_string(), 3.0);
    let solve_config = csp1d_solve_config::<f64, _>(|builder| {
        builder
            .column_generation_limits(1, 0, 0)
            .yield_config(Some(YieldModelingConfig::default()))
            .waste_config(Some(WasteMinimizationConfig {
                material_cost_penalty,
                ..WasteMinimizationConfig::default()
            }))
            .length_config(Some(LengthAssignmentModelingConfig::default()));
    });
    let mut short_demand = demand();
    short_demand.quantity = quantity(1.0);
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .product(dynamic_product())
            .material(material_with_length())
            .machine(machine())
            .demand(short_demand)
            .demand(dynamic_demand())
            .solve_config(solve_config);
    });
    let service = Csp1dColumnGeneration::with_generators(
        Box::new(FixedPlanEnumerator {
            plans: vec![over_producing_plan("over-producing")],
        }),
        Box::new(EmptyPricingGenerator),
    );

    let result = service.solve_with_trace(problem, None);
    let solution = result.solution;

    let yield_result = solution
        .yield_result
        .as_ref()
        .expect("yield result should be extracted");
    assert_eq!(yield_result.analysis.outputs.len(), 1);
    assert_eq!(yield_result.analysis.over_productions.len(), 1);
    assert_eq!(yield_result.analysis.over_productions[0].surplus.value, 1.0);
    assert_eq!(yield_result.analysis.under_productions.len(), 1);
    assert_eq!(
        yield_result.analysis.under_productions[0].demand.product.id,
        "p-dyn"
    );

    let waste_result = solution
        .waste_result
        .as_ref()
        .expect("waste result should be extracted");
    assert_eq!(waste_result.total_trim_width, Some(40.0));
    assert_eq!(waste_result.total_rest_material, Some(400.0));
    assert_eq!(waste_result.over_production_area, Some(30.0));
    assert_eq!(waste_result.material_costs.len(), 1);
    assert_eq!(waste_result.material_costs[0].material_id, "m1");
    assert_eq!(waste_result.material_costs[0].cost, 3.0);

    let length_result = solution
        .length_result
        .as_ref()
        .expect("length result should be extracted");
    assert_eq!(length_result.assignments.len(), 1);
    assert_eq!(length_result.assignments[0].product.id, "p-dyn");
    assert_eq!(length_result.assignments[0].assigned_length.value, 2.0);
    assert_eq!(length_result.over_length_records.len(), 1);
    assert_eq!(length_result.over_length_records[0].over_length.value, 1.0);

    assert!(solution.kpi.details.contains_key(Csp1dKpiKeys::TotalTrimWidth));
    assert!(
        solution
            .kpi
            .details
            .contains_key(&Csp1dKpiKeys::materialCost("m1"))
    );
    assert!(
        solution
            .kpi
            .details
            .contains_key(&Csp1dKpiKeys::assignedLength("p-dyn"))
    );
    assert!(
        solution
            .kpi
            .details
            .contains_key(&Csp1dKpiKeys::overLength("p-dyn"))
    );
    assert!(
        solution
            .kpi
            .details
            .contains_key(&Csp1dKpiKeys::overProduction("p1", "m"))
    );
    assert!(
        solution
            .kpi
            .details
            .contains_key(&Csp1dKpiKeys::underProduction("p-dyn", "m"))
    );
}

#[test]
fn default_column_generation_returns_trace_kpi_and_render() {
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand())
            .configuration(Csp1dConfiguration {
                max_initial_plans: 4,
                max_pricing_plans: 2,
                iteration_limit: 1,
            });
    });
    let result = Csp1dColumnGeneration::default().solve_with_trace(problem, None);

    assert_eq!(result.trace.final_milp_status, Csp1dFinalMilpStatus::Solved);
    assert!(result.trace.initial_plan_count >= 1);
    assert!(result.trace.final_plan_count >= 1);
    assert!(result.solution.kpi.generated_plan_count >= 1);
    assert!(
        result
            .solution
            .kpi
            .details
            .contains_key(Csp1dKpiKeys::SelectedPlanCount)
    );
    assert!(!result.solution.render.cutting_plans.is_empty());
    let render_plan = &result.solution.render.cutting_plans[0];
    assert_eq!(render_plan.group, vec!["Material 1".to_string(), "mc1".to_string()]);
    assert_eq!(
        render_plan.width,
        result.solution.produce.cutting_plans[0]
            .plan
            .used_width()
            .map(|width| format!("{:?}", width.value))
            .unwrap_or_else(|| "0".to_string())
    );
    assert_eq!(render_plan.standard_width, "100.0");
    assert_eq!(
        render_plan.info.get("planId"),
        Some(&render_plan.id)
    );
    assert_eq!(render_plan.productions[0].name, "Product 1");
    assert_eq!(render_plan.productions[0].x, "0");
    assert_eq!(
        render_plan.productions[0].info.get("amount"),
        Some(&render_plan.productions[0].amount.to_string())
    );
}

#[test]
fn schedule_entry_delegates_to_column_generation_by_default() {
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand());
    });

    let solution = Csp1dSchedule::default().solve(problem, None);

    assert_eq!(solution.status, Csp1dSolutionStatus::Feasible);
    assert!(solution.kpi.generated_plan_count >= 1);
    assert!(!solution.render.cutting_plans.is_empty());
}

#[cfg(feature = "serde")]
#[test]
fn render_schema_serializes_with_kotlin_camel_case_fields() {
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand());
    });
    let result = Csp1dColumnGeneration::with_generators(
        Box::new(FixedPlanEnumerator {
            plans: vec![cutting_plan("render-serde-plan")],
        }),
        Box::new(EmptyPricingGenerator),
    )
    .solve_with_trace(problem, None);
    let json = serde_json::to_string(&result.solution.render)
        .expect("render schema should serialize");

    assert!(json.contains("\"cuttingPlans\""));
    assert!(json.contains("\"unitLength\""));
    assert!(json.contains("\"productionType\""));
    assert!(json.contains("\"standardWidth\""));
    assert!(!json.contains("\"cutting_plans\""));
    assert!(!json.contains("\"unit_length\""));
    assert!(!json.contains("\"standard_width\""));
}

#[test]
fn plain_milp_generates_initial_plans_without_pricing_loop() {
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand())
            .configuration(Csp1dConfiguration {
                max_initial_plans: 4,
                max_pricing_plans: 4,
                iteration_limit: 3,
            });
    });
    let initial = cutting_plan("plain-milp-initial");
    let service = Csp1dMilp::with_initial_generator(Box::new(FixedPlanEnumerator {
        plans: vec![initial.clone()],
    }));

    let result = service.solve_with_trace(problem, None);

    assert_eq!(result.solution.status, Csp1dSolutionStatus::Feasible);
    assert_eq!(result.trace.initial_plan_count, 1);
    assert_eq!(result.trace.final_plan_count, 1);
    assert!(result.trace.iterations.is_empty());
    assert!(result.trace.priced_plan_count.is_empty());
    assert!(result.trace.pricing_generation_statistics.is_none());
    assert_eq!(result.trace.final_milp_status, Csp1dFinalMilpStatus::Solved);
    assert!(result
        .solution
        .generated_plans
        .iter()
        .any(|plan| plan.id == initial.id));
    assert_eq!(
        result
            .solution
            .kpi
            .details
            .get(Csp1dKpiKeys::FinalMilpStatus),
        Some(&"Solved".to_string())
    );
    assert!(!result
        .solution
        .kpi
        .details
        .contains_key(Csp1dKpiKeys::PricingGeneratedCandidates));
}

#[test]
fn plain_milp_applies_domain_policy_width_feasibility_override() {
    let solve_config = csp1d_solve_config::<f64, _>(|builder| {
        builder.domain_policy(Arc::new(RelaxingWidthPolicy));
    });
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(Product {
                width: vec![quantity(130.0)],
                ..product()
            })
            .material(material())
            .machine(machine())
            .demand(ProductDemand {
                product: Product {
                    width: vec![quantity(130.0)],
                    ..product()
                },
                quantity: quantity(1.0),
                mode: Some(DemandMode::Roll),
            })
            .solve_config(solve_config);
    });

    let result = Csp1dMilp::default().solve_with_trace(problem, None);

    assert_eq!(result.solution.status, Csp1dSolutionStatus::Feasible);
    assert_eq!(result.trace.initial_plan_count, 1);
    assert_eq!(result.solution.generated_plans.len(), 1);
    assert_eq!(result.solution.generated_plans[0].slices[0].width.value, 130.0);
}

#[test]
fn solution_enrichment_exports_full_generation_statistics_keys() {
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand());
    });
    let generator = NSameGenerator::with_constraints(GenerationConstraints {
        min_knife_count: Some(2),
        ..GenerationConstraints::default()
    })
    .with_all_amount(true);
    let service = Csp1dColumnGeneration::with_generators(
        Box::new(generator),
        Box::new(EmptyPricingGenerator),
    );

    let result = service.solve_with_trace(problem, None);

    assert_eq!(
        result
            .solution
            .kpi
            .details
            .get(Csp1dKpiKeys::InitialGenerationKnifeBoundPrunedNodes),
        Some(&"1".to_string())
    );
    assert_eq!(
        result
            .solution
            .kpi
            .details
            .get(Csp1dKpiKeys::InitialGenerationStopReason),
        Some(&"Exhausted".to_string())
    );
    assert_eq!(
        result
            .solution
            .render
            .kpi
            .get(Csp1dKpiKeys::InitialKnifeBoundPrunedNodes),
        Some(&"1".to_string())
    );
    assert_eq!(
        result
            .solution
            .render
            .kpi
            .get(Csp1dKpiKeys::InitialGenerationStopReasonRender),
        Some(&"Exhausted".to_string())
    );
}

#[test]
fn top_k_plans_are_sorted_by_used_width_like_kotlin() {
    let solve_config = csp1d_solve_config::<f64, _>(|builder| {
        builder.top_k_plan_limit(Some(1));
    });
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand())
            .solve_config(solve_config);
    });
    let mut narrow = cutting_plan("top-narrow");
    narrow.slices[0].width = quantity(20.0);
    let mut wide = cutting_plan("top-wide");
    wide.slices[0].width = quantity(30.0);
    let service = Csp1dColumnGeneration::with_generators(
        Box::new(FixedPlanEnumerator {
            plans: vec![narrow, wide.clone()],
        }),
        Box::new(EmptyPricingGenerator),
    );

    let result = service.solve_with_trace(problem, None);

    assert_eq!(result.solution.top_plans.len(), 1);
    assert_eq!(result.solution.top_plans[0].id, wide.id);
    assert_eq!(result.solution.kpi.top_plan_count, 1);
    assert_eq!(
        result
            .solution
            .render
            .kpi
            .get(Csp1dKpiKeys::TopPlanCount),
        Some(&"1".to_string())
    );
}

#[test]
fn extraction_policy_panic_does_not_escape_enrichment() {
    let solve_config = csp1d_solve_config::<f64, _>(|builder| {
        builder
            .extraction_policy(Arc::new(CustomExtractionPolicy {
                panic_on_enrich: true,
            }))
            .extraction_policy(Arc::new(CustomExtractionPolicy {
                panic_on_enrich: false,
            }));
    });
    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand())
            .solve_config(solve_config);
    });

    let result = Csp1dColumnGeneration::default().solve_with_trace(problem, None);

    assert_eq!(result.solution.status, Csp1dSolutionStatus::Feasible);
    assert_eq!(
        result.solution.kpi.details.get("custom-detail-key"),
        Some(&"detail-value".to_string())
    );
    assert_eq!(
        result.solution.render.kpi.get("custom-render-key"),
        Some(&"render-value".to_string())
    );
}
