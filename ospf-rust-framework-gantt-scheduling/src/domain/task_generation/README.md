# Task Generation

:us: English | :cn: [简体中文](README_ch.md)

This directory is a reserved task-generation extension point mapped from Kotlin `gantt-scheduling-domain-task-generation-context`.

## Responsibilities

The Rust workspace currently keeps this module as a placeholder for future task-generation workflows and extension hooks.

## Modules

- `mod.rs`: reserved module entry for future task-generation context code.

## Public API

No public task-generation API is exported yet.

## Extension Points

Future task generation should add context, policy, and generator traits here while keeping generated task vocabulary compatible with `task` and compilation-ready for `task_compilation`.

## Lifecycle and Data Flow

No runtime flow is implemented yet. The intended boundary is upstream task creation before task compilation and downstream scheduling workflows.

## Verification

Use `cargo check -p ospf-rust-framework-gantt-scheduling` after adding task-generation APIs, then add focused tests for any generation policy or context.

## Related Directories

- [`../task`](../task/README.md)
- [`../task_compilation`](../task_compilation/README.md)
