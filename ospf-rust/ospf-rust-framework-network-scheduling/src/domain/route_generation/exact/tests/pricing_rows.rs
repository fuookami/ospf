use std::sync::Arc;
use bigdecimal::BigDecimal;
use super::*;
use crate::domain::vrp::{Route, RoutePricingArc, RoutePricingRow, RoutePricingRows, RouteRowCoefficient};

struct PreciseUsage;

impl RouteRowCoefficient<BigDecimal> for PreciseUsage {
    fn route_coefficient(&self, route: &Route<BigDecimal>, _: &VrptwInstance<BigDecimal>) -> crate::Result<Quantity<BigDecimal, ospf_rust_quantities::unit::Unit>> {
        Ok(Quantity::new(decimal("-0.5000000000000000000000000000000000000001")
            * BigDecimal::from(route.effective_arc_ids().len() as u64), Meter::INSTANT.clone()))
    }

    fn arc_coefficient(&self, _: &RoutePricingArc<'_, BigDecimal>) -> crate::Result<Option<Quantity<BigDecimal, ospf_rust_quantities::unit::Unit>>> {
        Ok(Some(Quantity::new(decimal("-0.0005000000000000000000000000000000000000001"),
            ospf_rust_quantities::unit::Kilometer::INSTANT.clone())))
    }

    fn additive_decomposition_is_exact(&self) -> bool { true }
}

fn matrix(phase: PricingPhase, price: Option<&str>) {
    let instance = precise_instance();
    let rows = Arc::new(RoutePricingRows::new(if price.is_some() { vec![RoutePricingRow {
        id: "precise".into(), unit: Meter::INSTANT.clone(),
        lower: Some(Quantity::new(decimal("-2"), Meter::INSTANT.clone())),
        upper: Some(Quantity::new(decimal("3"), Meter::INSTANT.clone())),
        coefficient: Arc::new(PreciseUsage),
    }] } else { vec![] }).unwrap());
    let mut duals = ExactBigDecimalPricingDuals::new(phase, [("c1".into(), decimal("20"))], [("v1".into(), decimal("0"))]);
    if let Some(price) = price { duals = duals.with_extension_duals([("precise".into(), decimal(price))]); }
    let builder = ExactBigDecimalRouteGraphBuilder::new(instance.clone()).with_pricing_rows(rows.clone());
    let graph = builder.build(&"v1".into(), &duals, None).unwrap();
    for arc in &graph.arcs {
        assert_eq!(arc.row_coefficients.get("precise").cloned(), price.map(|_| decimal("-0.5000000000000000000000000000000000000001")));
    }
    let result = ExactBigDecimalEspprcPricer::new(builder).price_request(
        &ExactBigDecimalPricingRequest::new(instance.clone(), duals.clone(), "v1".into())).unwrap();
    assert!(result.exact_pricing_complete);
    assert_eq!(result.routes.len(), 1);
    let coefficient = decimal("-1.0000000000000000000000000000000000000002");
    let cost = if phase == PricingPhase::PhaseOne { decimal("0") } else { decimal("0.7777777777777777777777777777") };
    let expected = cost - decimal("20") - &coefficient * price.map_or_else(|| decimal("0"), decimal);
    assert_eq!(result.min_reduced_cost, expected);
    if price.is_some() {
        assert_eq!(rows.route_coefficients(&result.routes[0], &instance).unwrap()["precise"], coefficient);
        let mut missing = duals.clone(); missing.extension.clear();
        let builder = ExactBigDecimalRouteGraphBuilder::new(instance.clone()).with_pricing_rows(rows.clone());
        assert!(builder.build(&"v1".into(), &missing, None).is_err());
        missing.extension.insert("unknown".into(), decimal("0"));
        assert!(builder.build(&"v1".into(), &missing, None).is_err());
    }
    // 只在主问题注册边界允许舍入；精确定价的以上断言保留所有位。
    // Rounding is allowed only at master registration; exact-pricing assertions above retain every digit.
    let mut context = crate::domain::route_compilation::RouteCompilationContext::new(instance).with_pricing_rows(rows).unwrap();
    let mut model = ospf_rust_core::model::MetaModel::new("precise-row-master");
    context.register(&mut model).unwrap();
    context.add_columns(0, result.routes, &mut model).unwrap();
    if phase == PricingPhase::PhaseTwo { context.switch_to_phase_two(&mut model).unwrap(); }
    context.aggregation.validate_pricing_model(&model).unwrap();
}

macro_rules! case {
    ($name:ident, $phase:ident, $price:expr) => {
        #[test]
        fn $name() { matrix(PricingPhase::$phase, $price); }
    };
}

case!(decimal_phase_one_no_extension, PhaseOne, None);
case!(decimal_phase_two_no_extension, PhaseTwo, None);
case!(decimal_phase_one_zero_extension, PhaseOne, Some("0"));
case!(decimal_phase_two_zero_extension, PhaseTwo, Some("0"));
case!(decimal_phase_one_nonzero_extension, PhaseOne, Some("2.0000000000000000000000000000000000000003"));
case!(decimal_phase_two_nonzero_extension, PhaseTwo, Some("2.0000000000000000000000000000000000000003"));

#[test]
fn exact_nonadditive_terminal_row_keeps_sub_f64_improvement_without_dominance() {
    struct TerminalUsage;
    impl RouteRowCoefficient<BigDecimal> for TerminalUsage {
        fn route_coefficient(&self, _: &Route<BigDecimal>, _: &VrptwInstance<BigDecimal>) -> crate::Result<Quantity<BigDecimal, ospf_rust_quantities::unit::Unit>> {
            Ok(Quantity::new(decimal("0.7777777777777777777777777777000000000001"), Meter::INSTANT.clone()))
        }
    }
    let instance = precise_instance();
    let rows = Arc::new(RoutePricingRows::new(vec![RoutePricingRow {
        id: "terminal".into(), unit: Meter::INSTANT.clone(), lower: None,
        upper: Some(Quantity::new(decimal("1"), Meter::INSTANT.clone())), coefficient: Arc::new(TerminalUsage),
    }]).unwrap());
    let duals = ExactBigDecimalPricingDuals::new(PricingPhase::PhaseTwo, [], [("v1".into(), decimal("0"))])
        .with_extension_duals([("terminal".into(), decimal("1"))]);
    let builder = ExactBigDecimalRouteGraphBuilder::new(instance.clone()).with_pricing_rows(rows);
    let result = ExactBigDecimalEspprcPricer::new(builder).price_request(
        &ExactBigDecimalPricingRequest::new(instance, duals, "v1".into())).unwrap();
    assert!(result.exact_pricing_complete);
    assert_eq!(result.routes.len(), 1);
    assert_eq!(result.min_reduced_cost, decimal("-0.0000000000000000000000000000000000000001"));
    assert_eq!(result.statistics.dominance_checks, 0);
}
