# CSP1D Interface Renderer

English | [简体中文](README_ch.md)

A Tauri + Vue viewer for one-dimensional cutting plans. Select a JSON file exported through the Kotlin or Rust framework's renderer DTO/adapter.

## Input Compatibility

- Kotlin and Rust camelCase renderer DTOs are supported, with snake_case aliases for compatibility.
- Numeric fields accept numbers and numeric strings; null optional values are preserved.
- Product IDs, unit lengths, per-production amounts, material IDs and rest widths are retained.
- Existing `Product`, `Costar` and legacy `DefectCostar` labels and custom colors remain supported.

See the [shared renderer contract](../renderer-contract/README.md) for normalization rules and cross-language DTO tests. This viewer does not consume an arbitrary domain solve-result object.

## Development

From this directory:

```sh
npm install
npm run dev
npm run build
npx vue-tsc --noEmit
cargo check
```
