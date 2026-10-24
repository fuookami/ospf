# Gantt Scheduling Interface Renderer

English | [简体中文](README_ch.md)

A Tauri + Vue viewer for task schedules. Select a JSON file exported through the Kotlin or Rust framework's renderer DTO/adapter.

## Input Compatibility

- Kotlin's flat camelCase time fields and Rust's snake_case `actual_time` / `scheduled_time` ranges are accepted.
- Dates may be local date strings, ISO 8601 timestamps or the default `time::OffsetDateTime` serde tuple. Explicit offsets are normalized to UTC; offset-free Kotlin timestamps keep their export-local time. Use a common export timezone when comparing producers.
- Rust product, consumption and resource lists become index-keyed records. Kotlin maps and legacy `materials` remain supported.
- Tasks without actual or scheduled times are retained but not drawn. Entirely unscheduled or empty input shows a message.

See the [shared renderer contract](../renderer-contract/README.md) for normalization rules and cross-language DTO tests. This viewer does not consume an arbitrary domain solve-result object.

## Development

From this directory:

```sh
npm install
npm run dev
npm run build
npx vue-tsc --noEmit
npm test
cargo check
```
