use super::*;

fn previous() -> DiscreteLoadingSolveResult<Kilogram> {
    DiscreteLoadingApplicationService::solve(|| Ok(registered(false)), Default::default(), &SolveHandle::new(), |model, _, _| {
        let mut report = optimal_report(model, vec![0.0; model.num_variables()])?;
        report.proof = None;
        report.solution_presence = ospf_rust_core::solver::SolutionPresence::Incumbent;
        Ok(report)
    }).unwrap()
}

#[test]
fn validation_only_keeps_source_identity_incumbent_and_termination_without_a_solver() {
    let mut old = previous();
    old.report.termination_reason = TerminationReason::IterationLimit;
    let mapping = old.report.model_mapping.clone();
    let fingerprint = old.report.fingerprints.model.clone();
    let owner = old.registration.as_ref().unwrap().model().linear_model_snapshot().unwrap().revision.owner;
    DiscreteLoadingCertificationService::validate(&mut old, 1e-7).unwrap();
    assert_eq!(old.report.termination_reason, TerminationReason::IterationLimit);
    assert_eq!(old.report.model_mapping, mapping); assert_eq!(old.report.fingerprints.model, fingerprint);
    assert_eq!(old.registration.as_ref().unwrap().model().linear_model_snapshot().unwrap().revision.owner, owner);
    assert!(old.solution.is_some()); assert_eq!(old.objective, Some(7.0));
    assert!(!old.report.is_optimal());
}

#[test]
fn validation_only_rejects_invalid_vectors_objectives_and_source_identities() {
    for case in 0..5 {
        let mut old = previous();
        match case {
            0 => { old.report.model_mapping = None; }
            1 => { old.report.fingerprints.model = None; }
            2 => { old.report.solution.as_mut().unwrap().values.pop(); }
            3 => { old.report.solution.as_mut().unwrap().values[0] = 0.5; }
            _ => { old.report.solution.as_mut().unwrap().objective_value = Some(100.0); }
        }
        assert!(DiscreteLoadingCertificationService::validate(&mut old, 1e-7).is_err(), "case={case}");
    }
}

#[test]
fn fresh_source_certification_warm_starts_without_old_tokens_or_proofs() {
    let old = previous();
    let owner = old.registration.as_ref().unwrap().model().linear_model_snapshot().unwrap().revision.owner;
    let mut fresh_owner = None;
    let result = DiscreteLoadingCertificationService::solve(old, || {
        let fresh = registered(false);
        fresh_owner = Some(fresh.model().linear_model_snapshot()?.revision.owner);
        Ok(fresh)
    }, Default::default(), &SolveHandle::new(), |model, _, _| {
        assert!(model.basic.variables.iter().all(|token| token.get_result() == Some(0.0)));
        optimal_report(model, vec![0.0; model.num_variables()])
    }).unwrap();
    assert_ne!(Some(owner), fresh_owner);
    assert!(result.result.report.is_optimal());
    assert_eq!(result.result.objective, Some(7.0));
    assert!(result.certification_report.unwrap().is_optimal());
}

#[test]
fn source_certification_keeps_incumbent_and_recomputes_gaps_on_empty_final() {
    let result = DiscreteLoadingCertificationService::solve(previous(), || Ok(registered(false)), Default::default(), &SolveHandle::new(), |model, _, _| {
        SolveReport::builder(ProblemStatus::Unknown, TerminationReason::TimeLimit)
            .statistics(ospf_rust_core::solver::SolveStatistics { best_bound: Some(-1.0), best_bound_value: Some(-1.0), ..Default::default() })
            .model_mapping(Some(linear_model_mapping(model)?))
            .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() }).build()
    }).unwrap();
    assert_eq!(result.result.objective, Some(7.0));
    assert!(result.result.solution.is_some());
    assert_eq!(result.result.report.termination_reason, TerminationReason::TimeLimit);
    assert_eq!(result.result.report.statistics.absolute_gap, Some(1.0));
    assert!(result.certification_report.unwrap().solution.is_none());
    result.result.report.validate().unwrap();
}

#[test]
fn source_certification_rejects_changed_source_and_does_not_call_backend() {
    assert!(DiscreteLoadingCertificationService::solve(previous(), || {
        let mut fresh = registered(false);
        fresh.model.add_linear_objective_input(ospf_rust_core::model::LinearObjectiveInput::minimize("changed").constant(1.0));
        Ok(fresh)
    }, Default::default(), &SolveHandle::new(), |_, _, _| panic!("non-equivalent model cannot solve")).is_err());
}

#[test]
fn source_certification_respects_cancelled_interrupted_and_zero_budget() {
    for state in 0..3 {
        let mut old = previous();
        let handle = SolveHandle::new();
        let mut options = DiscreteLoadingSolveOptions::default();
        let expected = match state {
            0 => { handle.cancel("certification-test"); TerminationReason::Cancelled }
            1 => { old.report.termination_reason = TerminationReason::Interrupted; TerminationReason::Interrupted }
            _ => { options.time_limit = Duration::ZERO; TerminationReason::TimeLimit }
        };
        let result = DiscreteLoadingCertificationService::solve(old, || panic!("no factory after terminal"), options, &handle,
            |_, _, _| panic!("no solve after terminal")).unwrap();
        assert!(result.result.solution.is_some());
        assert_eq!(result.result.report.termination_reason, expected);
        assert!(result.certification_report.is_none());
    }
}

#[test]
fn source_certification_backend_errors_preserve_plan_but_contract_errors_propagate() {
    for recoverable in [false, true] {
        let result = DiscreteLoadingCertificationService::solve(previous(), || Ok(registered(false)), Default::default(), &SolveHandle::new(), |_, _, _| {
            Err(if recoverable { ospf_rust_core::error::CoreError::NotImplemented("backend capability".into()) }
                else { invalid("invalid callback contract") })
        });
        if recoverable {
            let result = result.unwrap();
            assert!(result.result.solution.is_some());
            assert!(result.certification_error.is_some());
            assert_eq!(result.result.report.termination_reason, TerminationReason::BackendFailure);
        } else { assert!(result.is_err()); }
    }
}

#[test]
fn source_certification_does_not_accept_infeasibility_against_a_feasible_incumbent() {
    assert!(DiscreteLoadingCertificationService::solve(previous(), || Ok(registered(false)), Default::default(), &SolveHandle::new(), |model, _, _| {
        SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed).proof(SolveProof::infeasibility())
            .model_mapping(Some(linear_model_mapping(model)?))
            .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() }).build()
    }).is_err());
}

#[test]
fn source_certification_does_not_repeat_a_complete_source_optimal_proof() {
    let mut old = previous();
    old.report.proof = Some(SolveProof::optimality());
    old.report.solution_presence = ospf_rust_core::solver::SolutionPresence::Optimal;
    let result = DiscreteLoadingCertificationService::solve(old, || panic!("already certified"), Default::default(), &SolveHandle::new(),
        |_, _, _| panic!("already certified")).unwrap();
    assert!(result.result.report.is_optimal());
    assert!(result.certification_report.is_none());
}

#[test]
fn source_certification_keeps_a_better_plan_and_rejects_a_worse_claimed_optimum() {
    let factory = || {
        let mut registration = registered_resource();
        let resource = registration.model.tokens().iter().position(|token| token.name() == "resource").unwrap();
        registration.model.add_linear_objective_input(ospf_rust_core::model::LinearObjectiveInput::minimize("resource_cost").terms([(resource, 1.0)]));
        Ok(registration)
    };
    let values = |model: &LinearTriadModel, resource: f64| model.basic.variables.iter().map(|token| {
        if token.name() == "resource" { resource }
        else if token.name().ends_with("_occupied_0") { 1.0 } else { 0.0 }
    }).collect();
    for claimed_optimal in [false, true] {
        let old = DiscreteLoadingApplicationService::solve(factory, Default::default(), &SolveHandle::new(), |model, _, _| {
            let mut report = optimal_report(model, values(model, 1.0))?;
            report.proof = None; report.solution_presence = ospf_rust_core::solver::SolutionPresence::Incumbent;
            ospf_rust_core::solver::audit::attach_linear_solution_audit(report, model)
        }).unwrap();
        let result = DiscreteLoadingCertificationService::solve(old, factory, Default::default(), &SolveHandle::new(), |model, _, _| {
            let mut report = optimal_report(model, values(model, 2.0))?;
            if !claimed_optimal {
                report.proof = None; report.solution_presence = ospf_rust_core::solver::SolutionPresence::Incumbent;
                report.termination_reason = TerminationReason::TimeLimit;
            }
            ospf_rust_core::solver::audit::attach_linear_solution_audit(report, model)
        });
        if claimed_optimal { assert!(result.is_err()); }
        else {
            let result = result.unwrap();
            assert_eq!(result.result.objective, Some(8.0));
            let actual = result.certification_report.unwrap();
            assert_eq!(actual.solution.as_ref().unwrap().objective_value, Some(2.0));
            let compiled = result.result.registration.as_ref().unwrap().model().linear_model_snapshot().unwrap().compile_flt64().unwrap();
            let audit = ospf_rust_core::solver::audit::evaluate_linear_solution(&compiled.model, &result.result.report.solution.as_ref().unwrap().values, 1e-7).unwrap();
            assert_eq!(result.result.report.diagnostics.variable_bound_evaluations, audit.variable_bound_evaluations);
            assert_ne!(result.result.report.diagnostics.variable_bound_evaluations, actual.diagnostics.variable_bound_evaluations);
            assert!(!result.result.report.is_optimal());
        }
    }
}

#[test]
fn source_certification_preserves_a_true_unbounded_terminal_over_a_finite_plan() {
    let factory = || {
        let mut fresh = registered(false);
        let free = fresh.model.register_auto_variable::<ospf_rust_core::variable::UContinuous>("unbounded_t")?;
        fresh.model.add_linear_objective_input(ospf_rust_core::model::LinearObjectiveInput::minimize("unbounded_cost").terms([(free, -1.0)]));
        Ok(fresh)
    };
    let old = DiscreteLoadingApplicationService::solve(factory, Default::default(), &SolveHandle::new(), |model, _, _| {
        let mut report = optimal_report(model, vec![0.0; model.num_variables()])?;
        report.proof = None; report.solution_presence = ospf_rust_core::solver::SolutionPresence::Incumbent;
        Ok(report)
    }).unwrap();
    let result = DiscreteLoadingCertificationService::solve(old, factory, Default::default(), &SolveHandle::new(), |model, _, _| {
        SolveReport::builder(ProblemStatus::Unbounded, TerminationReason::Completed).proof(SolveProof::unboundedness())
            .model_mapping(Some(linear_model_mapping(model)?))
            .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() }).build()
    }).unwrap();
    assert_eq!(result.result.report.problem_status, ProblemStatus::Unbounded);
    assert!(result.result.report.proof.is_some());
    assert!(result.result.report.solution.is_none());
}

#[test]
fn completed_source_terminal_proofs_survive_zero_budget_cancellation_and_backend_failures() {
    for status in [ProblemStatus::Infeasible, ProblemStatus::Unbounded, ProblemStatus::InfeasibleOrUnbounded] {
        for cancel in [false, true] {
            let old = DiscreteLoadingApplicationService::solve(|| Ok(registered(false)), Default::default(), &SolveHandle::new(), |model, _, _| {
                let proof = match status {
                    ProblemStatus::Infeasible => SolveProof::infeasibility(),
                    ProblemStatus::Unbounded => SolveProof::unboundedness(),
                    _ => SolveProof::infeasible_or_unbounded(),
                };
                SolveReport::builder(status, TerminationReason::Completed).proof(proof)
                    .model_mapping(Some(linear_model_mapping(model)?))
                    .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() }).build()
            }).unwrap();
            let handle = SolveHandle::new();
            if cancel { handle.cancel("source-already-proven"); }
            let result = DiscreteLoadingCertificationService::solve(old, || panic!("terminal source proof needs no new factory"),
                DiscreteLoadingSolveOptions { time_limit: if cancel { Duration::from_secs(10) } else { Duration::ZERO }, ..Default::default() }, &handle,
                |_, _, _| Err(ospf_rust_core::error::CoreError::NotImplemented("must never try a backend after a source terminal proof".into()))).unwrap();
            assert_eq!(result.result.report.problem_status, status);
            assert_eq!(result.result.report.termination_reason, TerminationReason::Completed);
            assert!(result.result.report.proof.is_some());
            assert!(result.certification_report.is_none());
            result.result.report.validate().unwrap();
        }
    }
}

#[test]
fn source_terminal_proofs_require_both_original_matching_identity_fields() {
    for missing in 0..4 {
        let mut old = previous();
        old.report.solution = None; old.report.solution_presence = ospf_rust_core::solver::SolutionPresence::None;
        old.report.problem_status = ProblemStatus::Infeasible;
        old.report.proof = Some(SolveProof::infeasibility());
        old.solution = None; old.objective = None;
        match missing {
            0 => old.report.model_mapping = None,
            1 => old.report.fingerprints.model = None,
            2 => {
                let foreign = registered(false).model().linear_model_snapshot().unwrap().compile_flt64().unwrap();
                old.report.model_mapping = Some(linear_model_mapping(&foreign.model).unwrap());
            }
            _ => {
                let mut foreign = old.registration.as_ref().unwrap().model().linear_model_snapshot().unwrap().compile_flt64().unwrap();
                foreign.model.c[0] += 1.0;
                old.report.fingerprints.model = Some(linear_model_fingerprint(&foreign.model).unwrap());
            }
        }
        assert!(DiscreteLoadingCertificationService::solve(old, || panic!("unidentified proof cannot create a factory"), Default::default(), &SolveHandle::new(),
            |_, _, _| panic!("unidentified proof cannot be relabeled or solved")).is_err(), "case={missing}");
    }
}
