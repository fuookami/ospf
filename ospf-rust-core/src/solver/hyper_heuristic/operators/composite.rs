//! 有界嵌套工厂与显式编码能力检查 / Bounded nested factories and explicit encoding capability checks

use super::*;

fn encoding_allowed(
    capabilities: Option<&OperatorCapabilities>,
    encoding: SearchEncoding,
    nested: bool,
) -> Result<()> {
    match capabilities {
        Some(capabilities) if capabilities.encodings.contains(&encoding) && (!nested || capabilities.stateless) => Ok(()),
        None if encoding == SearchEncoding::Continuous && !nested => Ok(()),
        _ => Err(invalid("custom nested operators require explicit stateless compatible encoding capabilities")),
    }
}

fn builtin_encoding(kind: &str, encoding: SearchEncoding, compatible: &[&str]) -> Result<()> {
    if encoding == SearchEncoding::Continuous || compatible.contains(&kind) { Ok(()) }
    else { Err(invalid("operator does not support this solution encoding")) }
}

pub(super) fn build_mutation(
    spec: &ComponentSpec,
    registry: &OperatorRegistry<f64>,
    encoding: SearchEncoding,
    nested: bool,
) -> Result<Box<dyn MutationComponent<f64>>> {
    if spec.version != 1 { return Err(invalid("unsupported nested mutation version")); }
    if let Some(factory) = registry.mutations.get(&spec.r#type) {
        encoding_allowed(registry.mutation_capabilities.get(&spec.r#type), encoding, nested)?;
        let component = factory(spec)?;
        if nested && !component.stateless() { return Err(invalid("nested mutation instance is stateful")); }
        return Ok(component);
    }
    builtin_encoding(&spec.r#type, encoding, &["none", "swap", "inversion"])?;
    Ok(Box::new(Stateless(mutation(spec)?)))
}

pub(super) fn build_neighborhood(
    spec: &ComponentSpec,
    registry: &OperatorRegistry<f64>,
    encoding: SearchEncoding,
    nested: bool,
) -> Result<Box<dyn NeighborhoodComponent<f64>>> {
    if spec.version != 1 { return Err(invalid("unsupported nested neighborhood version")); }
    if let Some(factory) = registry.neighborhoods.get(&spec.r#type) {
        encoding_allowed(registry.neighborhood_capabilities.get(&spec.r#type), encoding, nested)?;
        let component = factory(spec)?;
        if nested && !component.stateless() { return Err(invalid("nested neighborhood instance is stateful")); }
        return Ok(component);
    }
    builtin_encoding(&spec.r#type, encoding, &[])?;
    Ok(Box::new(Stateless(neighborhood(spec)?)))
}

pub(super) fn build_update(
    spec: &ComponentSpec,
    registry: &OperatorRegistry<f64>,
    encoding: SearchEncoding,
) -> Result<Box<dyn UpdateComponent<f64>>> {
    nested_update(spec, registry, encoding, 0, &mut 0)
}

fn nested_update(
    spec: &ComponentSpec,
    registry: &OperatorRegistry<f64>,
    encoding: SearchEncoding,
    depth: usize,
    nodes: &mut usize,
) -> Result<Box<dyn UpdateComponent<f64>>> {
    *nodes += 1;
    if depth > 16 || *nodes > 1024 || spec.version != 1 { return Err(invalid("composite depth, node count or version exceeded")); }
    if let Some(factory) = registry.updates.get(&spec.r#type) {
        encoding_allowed(registry.update_capabilities.get(&spec.r#type), encoding, depth > 0)?;
        let component = factory(spec)?;
        if depth > 0 && !component.stateless() { return Err(invalid("nested update instance is stateful")); }
        return Ok(component);
    }
    let result: Box<dyn SolutionUpdate<f64>> = match spec.r#type.as_str() {
        "mutation-stage" | "neighborhood-stage" => {
            spec.validate_parameters(if spec.r#type == "mutation-stage" { &["operator", "rate"] } else { &["operator"] })?;
            let child: ComponentSpec = serde_json::from_value(spec.parameters.get("operator").cloned().ok_or_else(|| invalid("missing composite operator"))?)
                .map_err(|_| invalid("invalid composite operator"))?;
            *nodes += 1;
            if *nodes > 1024 || depth >= 16 { return Err(invalid("nested operator depth or node count exceeded")); }
            if spec.r#type == "mutation-stage" {
                let rate = spec.number("rate", 1.0)?;
                if !(0.0..=1.0).contains(&rate) { return Err(invalid("invalid mutation-stage rate")); }
                Box::new(MutationUpdate::new(build_mutation(&child, registry, encoding, true)?, rate))
            } else {
                Box::new(NeighborhoodUpdate::new(build_neighborhood(&child, registry, encoding, true)?))
            }
        }
        "pipeline" | "vnd" | "elite-local" => {
            spec.validate_parameters(match spec.r#type.as_str() {
                "vnd" => &["stages", "maxTrials"], "elite-local" => &["stages", "amount"], _ => &["stages"],
            })?;
            let children: Vec<ComponentSpec> = serde_json::from_value(spec.parameters.get("stages").cloned().ok_or_else(|| invalid("missing composite stages"))?)
                .map_err(|_| invalid("invalid composite stages"))?;
            if children.is_empty() || children.len() > 64 { return Err(invalid("composite requires 1..64 stages")); }
            let stages = children.iter().map(|child| {
                nested_update(child, registry, encoding, depth + 1, nodes).map(|component| component as Box<dyn SolutionUpdate<f64>>)
            }).collect::<Result<Vec<_>>>()?;
            match spec.r#type.as_str() {
                "vnd" => {
                    let trials = usize::try_from(spec.integer("maxTrials", 50)?).map_err(|_| invalid("maxTrials overflow"))?;
                    if trials == 0 { return Err(invalid("maxTrials must be positive")); }
                    Box::new(VNDCompositeUpdate::new(stages, trials))
                }
                "elite-local" => {
                    let amount = usize::try_from(spec.integer("amount", 1)?).map_err(|_| invalid("elite amount overflow"))?;
                    if amount == 0 { return Err(invalid("elite amount must be positive")); }
                    Box::new(EliteLocalSearchWrapper::new(Box::new(PipelineCompositeUpdate::new(stages)), amount))
                }
                _ => Box::new(PipelineCompositeUpdate::new(stages)),
            }
        }
        _ => {
            builtin_encoding(&spec.r#type, encoding, &["order", "cycle"])?;
            update(spec)?
        }
    };
    // 子节点都已验证无状态，组合才可声明无状态 / Only composites whose children were verified stateless may declare statelessness.
    Ok(Box::new(Stateless(result)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stages(kind: &str, children: Vec<ComponentSpec>) -> ComponentSpec {
        ComponentSpec::new(kind).parameter("stages", serde_json::to_value(children).unwrap())
    }

    struct CustomUpdate(bool);

    impl RecoverableComponent for CustomUpdate { fn stateless(&self) -> bool { self.0 } }

    impl SolutionUpdate<f64> for CustomUpdate {
        fn update(
            &self,
            _model: &dyn VariableModel<f64>,
            _iteration: &Iteration,
            parents: &[Solution<f64>],
            _entropy: &mut dyn Entropy,
        ) -> Vec<Solution<f64>> { parents.to_vec() }
    }

    #[test]
    fn nested_factory_checks_versions_parameters_bounds_and_compatible_encodings() {
        let registry = OperatorRegistry::new();
        let permutation = stages("elite-local", vec![stages("vnd", vec![ComponentSpec::new("mutation-stage")
            .parameter("operator", serde_json::to_value(ComponentSpec::new("swap")).unwrap())])]);
        assert!(build_update(&permutation, &registry, SearchEncoding::Permutation).unwrap().stateless());
        let continuous = stages("pipeline", vec![ComponentSpec::new("opposition"), ComponentSpec::new("pattern-search")]);
        assert!(build_update(&continuous, &registry, SearchEncoding::Continuous).is_ok());
        assert!(build_update(&continuous, &registry, SearchEncoding::Permutation).is_err());
        let mut version = ComponentSpec::new("order"); version.version = 2;
        for spec in [stages("pipeline", vec![version]), stages("pipeline", vec![]), stages("pipeline", vec![ComponentSpec::new("order"); 65]),
            stages("pipeline", vec![ComponentSpec::new("order")]).parameter("unknown", 1),
            stages("vnd", vec![ComponentSpec::new("order")]).parameter("maxTrials", 0),
            stages("elite-local", vec![ComponentSpec::new("order")]).parameter("amount", -1),
            ComponentSpec::new("pattern-search").parameter("contraction", 1.0),
            ComponentSpec::new("mutation-stage").parameter("operator", serde_json::to_value(ComponentSpec::new("swap")).unwrap()).parameter("rate", 1.1),
            ComponentSpec::new("neighborhood-stage").parameter("operator", serde_json::json!({"type": "random", "extra": true})),
            ComponentSpec::new("pipeline").parameter("stages", serde_json::json!({"type": "order"}))] {
            assert!(build_update(&spec, &registry, SearchEncoding::Continuous).is_err(), "{spec:?}");
        }
        let mut deep = ComponentSpec::new("order");
        for _ in 0..18 { deep = stages("pipeline", vec![deep]); }
        assert!(build_update(&deep, &registry, SearchEncoding::Continuous).is_err());
        let wide = stages("pipeline", vec![stages("pipeline", vec![ComponentSpec::new("order"); 32]); 32]);
        assert!(build_update(&wide, &registry, SearchEncoding::Continuous).is_err());
    }

    #[test]
    fn custom_nesting_requires_explicit_capabilities_and_actual_stateless_instance() {
        let mut registry = OperatorRegistry::new();
        registry.register_update("legacy", |_| Ok(Box::new(CustomUpdate(true)))).unwrap();
        let compatible = OperatorCapabilities { encodings: vec![SearchEncoding::Continuous, SearchEncoding::Permutation], stateless: true };
        registry.register_update_with_capabilities("safe", compatible.clone(), |_| Ok(Box::new(CustomUpdate(true)))).unwrap();
        registry.register_update_with_capabilities("liar", compatible, |_| Ok(Box::new(CustomUpdate(false)))).unwrap();
        registry.register_update_with_capabilities("stateful", OperatorCapabilities { encodings: vec![SearchEncoding::Continuous], stateless: false }, |_| Ok(Box::new(CustomUpdate(false)))).unwrap();
        assert!(build_update(&ComponentSpec::new("legacy"), &registry, SearchEncoding::Continuous).is_ok());
        assert!(build_update(&ComponentSpec::new("legacy"), &registry, SearchEncoding::Permutation).is_err());
        for kind in ["legacy", "liar", "stateful"] {
            assert!(build_update(&stages("pipeline", vec![ComponentSpec::new(kind)]), &registry, SearchEncoding::Continuous).is_err(), "{kind}");
        }
        assert!(build_update(&stages("pipeline", vec![ComponentSpec::new("safe")]), &registry, SearchEncoding::Permutation).is_ok());
        assert!(registry.register_update_with_capabilities("bad", OperatorCapabilities { encodings: vec![], stateless: true }, |_| Ok(Box::new(CustomUpdate(true)))).is_err());
        assert!(!registry.updates.contains_key("bad"));
        assert!(registry.register_update_with_capabilities("bad", OperatorCapabilities { encodings: vec![SearchEncoding::Continuous; 2], stateless: true }, |_| Ok(Box::new(CustomUpdate(true)))).is_err());
    }
}
