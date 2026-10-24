# Renderer Contract

English | [简体中文](README_ch.md)

Shared import helpers used by the BPP3D, CSP1D and Gantt Tauri renderers. They normalize the current Kotlin and Rust **renderer DTOs** to camelCase JSON for Vue. Domain solver results must first pass through their framework's renderer adapter.

| Input difference | Import behavior |
| --- | --- |
| Kotlin camelCase / Rust snake_case | Both accepted; output uses camelCase. |
| Numeric strings / JSON numbers | Converted to finite numbers; invalid values fail with an import error. |
| BPP3D string group / list group | Converted to a list; shape and bounding-box metadata are retained. |
| Text, object or null `info`; JSON-valued KPI | Converted to string-valued records; text uses the `info` key. Nested KPI values remain JSON text. |
| CSP1D `id`, `materialId`, `unitLength`, per-production `amount`, `restWidth` | Retained when supplied; absent optional fields stay null. |
| Gantt lists / records | Lists become index-keyed records, preserving repeated values and order. |
| Gantt `actual_time` / `scheduled_time` ranges | Converted to flat start/end fields; scheduled-only tasks use the scheduled range for display. |
| Gantt date strings / `time::OffsetDateTime` serde tuples | Both accepted, retaining nanosecond precision at the DTO boundary. |

Timestamps with explicit offsets are normalized to UTC. Kotlin `DateTimeSerializer` produces offset-free local wall times, which remain unchanged; it is not possible to infer their original timezone. Align the export timezone (for example, UTC) before comparing files from different producers. The chart displays millisecond precision.

Rust Gantt tasks have no top-level name or category: display names use `produce`, falling back to `executor #order`, and category defaults to `Normal`. Rust `order` is a sequence index, while Kotlin `order` may identify a business order; importing does not make these meanings interchangeable.

Unscheduled tasks and subtasks retain null endpoints and are not drawn. Empty or entirely unscheduled task lists show a message instead of fabricated dates. Invalid or reversed time ranges are rejected. Gantt consumption includes both current `consumption` and legacy `materials` (current keys take precedence).

The helper crate's integration tests include the **same DTO source files used by the three Tauri apps**, so JSON compatibility can be tested without a desktop toolchain:

```sh
cargo test --manifest-path framework/renderer-contract/Cargo.toml
```

Run from the repository root. Each renderer also provides `npm run build` and `npx vue-tsc --noEmit`. Gantt adds `npm test` for DTO-to-view conversion (Node.js 24).
