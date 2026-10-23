# Gantt Network Bridge

[简体中文](README_ch.md)

This optional crate connects Network resource paths to Gantt business bunches. It
does not register a second master model, invoke a solver, or own global scheduling.
Gantt itself does not depend on this crate or on Network.

`NetworkBunchGenerationContext::generate` accepts a `PathProblem`, computational
budget, current `ColumnGenerationScope`, a whole-path bunch extractor and an
independent no-improvement predicate. The result retains original states, arc-ID
traces and diagnostics alongside `ColumnGenerationBatch`.

An empty output-limited list is not a no-improvement certificate. Budget exhaustion,
interruption, uncertified dominance, validation rejection or discarded extraction
cannot certify exhaustion. Gantt must separately validate its current model-bound
LP report and the exact pricing scope before promoting a bound.

Run offline tests with a compatible nightly toolchain:

```powershell
cargo +nightly-2026-08-01 test -p ospf-rust-framework-gantt-network --no-default-features
```

This bridge does not imply that the full Network domain migration or Demo4
end-to-end migration is complete; see the Network migration notes.
