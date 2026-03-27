# BPP3D Interface Renderer

:us: English | :cn: [简体中文](README_ch.md)

This Tauri + Vue renderer loads BPP3D loading-plan JSON files and displays each bin in a Three.js scene.

## Supported Shapes

- `Cuboid`: rendered with `THREE.BoxGeometry`.
- `Cylinder`: rendered with `THREE.CylinderGeometry` when `axis` is `X`, `Y`, or `Z`.
- Missing shape fields are treated as legacy `Cuboid` data.

Cylinder items use the backend bounding-box position semantics. The mesh center is calculated from `x/y/z + boundingWidth/boundingHeight/boundingDepth / 2`, with `width/height/depth` used as fallback bounding dimensions.

The summary panel totals item volume from DTO `actualVolume` when present, so cylinder loading rate is based on real cylinder volume rather than the bounding cuboid volume.

The Tauri DTO parser accepts numeric fields as either JSON numbers or numeric strings. Shape enum aliases such as `CYLINDER` / `VERTICAL_CYLINDER` are accepted for compatibility, while BPP3D currently emits `Cylinder` / `VerticalCylinder`.

Unsupported cylinder data is still visible as a warning-colored bounding-box placeholder, logs a diagnostic message in the browser console, and shows the unsupported reason in the item detail panel:

- missing both `radius` and `diameter`

## Shape Fields

Renderer item shape fields are optional for legacy compatibility:

- `shapeType`: `Cuboid` or `Cylinder`
- `renderShapeType`: `Cuboid` or `Cylinder`
- `algorithmShapeType`: `Cuboid`, `VerticalCylinder`, `HorizontalCylinderX`, `HorizontalCylinderZ`, or `BoundingCuboid`
- `radius`
- `diameter`
- `axis`: `X`, `Y`, or `Z`
- `boundingWidth`
- `boundingHeight`
- `boundingDepth`
- `actualVolume`

The item detail panel shows actual shape type, render shape type, algorithm shape type, cylinder axis, radius, diameter, actual volume, package size, and bounding size when available.

`BoundingCuboid` is treated only as compatibility input. When a legacy item carries `shapeType = Cylinder` and `renderShapeType = Cuboid`, the renderer displays the bounding box while preserving cylinder metadata in the detail panel.

## Cylinder Coordinate Guide

Cylinder coordinates use the same bounding-box origin as cuboids:

- `x`, `y`, `z`: the minimum corner of the cylinder bounding box.
- `boundingWidth`, `boundingHeight`, `boundingDepth`: the full bounding-box size.
- `radius`: cylinder radius. `diameter` should normally be `radius * 2`.

Set the bounding dimensions by axis:

- `axis = X`: the cylinder lies along the X axis. Use `boundingWidth` as the cylinder length, and set `boundingHeight = diameter`, `boundingDepth = diameter`.
- `axis = Y`: the cylinder stands vertically. Use `boundingHeight` as the cylinder length, and set `boundingWidth = diameter`, `boundingDepth = diameter`.
- `axis = Z`: the cylinder lies along the Z axis. Use `boundingDepth` as the cylinder length, and set `boundingWidth = diameter`, `boundingHeight = diameter`.

To place a cylinder on the bin floor, set `y = 0`. For X/Z-axis horizontal cylinders, the renderer places the mesh center at `y + boundingHeight / 2`, so the cylinder bottom touches the floor when `boundingHeight = diameter`.

## Examples

- `src/examples/legacy-cuboid-loading-plan.json`: legacy cuboid-only data.
- `src/examples/cuboid-cylinder-loading-plan.json`: mixed cuboid, native X/Y/Z-axis cylinder, and solver-selected radius data.
- `src/examples/unsupported-bounding-cuboid-loading-plan.json`: compatibility `BoundingCuboid` and unsupported cylinder data.

## Commands

```bash
npm run build
npx vue-tsc --noEmit
cargo check
cargo test
```
