# Environment and Running Examples

Each first lesson provides business input, example functions and an application launcher. Use them in a small application; repository tests are not a prerequisite. This page covers dependencies, toolchains and real backends. Regression commands are in [Verification](./verification).

## 1. Get the Source

```powershell
git clone https://github.com/fuookami/ospf.git
Set-Location ospf
```

Build source modules from their language repository; launch business examples from your own application directory. The path dependencies below assume the application is beside the ospf checkout; adjust paths for other layouts. Separate language checkouts are also usable; match the source revision used by the lesson.

::: warning Source versions are not publication guarantees
These lessons follow the current layout: the Kotlin root POM is `1.1.2`, while Rust domain crates are generally `1.1.0`. This does not establish that matching artifacts are published to Maven Central or crates.io. Do not copy a Kotlin version into Rust or mix historical Guide dependencies with new source APIs.
:::

## 2. Kotlin

Prepare JDK 25, Maven and repository settings that resolve the POM dependencies. The current root POM targets JVM 25; an older JDK cannot run these compiled classes. Confirm that `mvn -version` uses the intended JDK, not just that `java -version` does. For source integration, build/install only the modules you need and their reactor dependencies, not every business framework.

```powershell
# Working directory: ospf-kotlin
java -version
mvn -version
```

Standalone applications can use domain starters, with a separately configured solver plugin:

| Scenario | Kotlin starter artifact |
| --- | --- |
| BPP1D / BPP2D / BPP3D | `ospf-kotlin-starter-bpp1d` / `ospf-kotlin-starter-bpp2d` / `ospf-kotlin-starter-bpp3d` |
| CSP1D / CSP2D | `ospf-kotlin-starter-csp1d` / `ospf-kotlin-starter-csp2d` |
| Gantt / Network | `ospf-kotlin-starter-gantt-scheduling` / `ospf-kotlin-starter-network-scheduling` |

Their groupId is `io.github.fuookami.ospf.kotlin`. Use an artifact version whose availability you have confirmed; install the required reactor modules when integrating from source. See [Kotlin starters](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-starters).

### Kotlin Application Dependencies

The domain starter supplies business APIs, the native plugin supplies a real solver, and `kotlinx-coroutines-core` supplies `runBlocking`. Example Maven dependencies for BPP1D + Gurobi (source version; artifacts must be installed or resolvable):

```xml
<dependency>
    <groupId>io.github.fuookami.ospf.kotlin</groupId>
    <artifactId>ospf-kotlin-starter-bpp1d</artifactId>
    <version>1.1.2</version>
</dependency>
<dependency>
    <groupId>io.github.fuookami.ospf.kotlin.core.plugin</groupId>
    <artifactId>ospf-kotlin-core-plugin-gurobi</artifactId>
    <version>1.1.2</version>
</dependency>
<dependency>
    <groupId>org.jetbrains.kotlinx</groupId>
    <artifactId>kotlinx-coroutines-core</artifactId>
    <version>1.11.0</version>
</dependency>
```

A standalone project also needs Kotlin/JVM compiler setup (current source: Kotlin 2.4.20, JVM target 25). Put the example and main in application sources and launch main as a Kotlin Application in your IDE; no JUnit is needed. The Gurobi Java binding is provided-scoped in the plugin: add it explicitly to application compile/runtime dependencies and match native JNI libraries and the license. These three entries alone are not a complete native installation.

## 3. Rust

The current workspace's base crates use unstable features including `specialization`, so these source tutorials use nightly. A stable compiler supporting edition 2024 is not sufficient. The commands pin `nightly-2026-08-01`, verified against the current CSP2D example: newer nightly allocator API changes currently prevent this checkout from compiling. This is a tutorial compatibility pin, not a change to your global default toolchain. Setting `RUSTC_BOOTSTRAP` is not a solver-installation step.

```powershell
# Working directory: ospf-rust
rustup toolchain install nightly-2026-08-01
rustc +nightly-2026-08-01 --version
cargo +nightly-2026-08-01 --version
```

Place the example function in `src/<framework>.rs` and its launcher in `src/main.rs`. Run `cargo +nightly-2026-08-01 run` from the application directory: it executes your main, not repository test filters.

Standalone applications add a domain crate such as `ospf-rust-framework-bpp1d`. A source checkout can use Cargo path dependencies. Native backends require explicit features; consult that crate's `Cargo.toml` for the actual names.

### Your First Standalone Rust Application

Start with the [CSP2D example](./csp2d/first-cutting), which needs no native runtime. Create `first-cutting` beside the `ospf` checkout, add this `Cargo.toml`, and use the lesson sources as `src/csp2d.rs` and `src/main.rs`:

```toml
[package]
name = "first-cutting"
version = "0.1.0"
edition = "2024"

[dependencies]
ospf-rust-framework-csp2d = { path = "../ospf/ospf-rust/ospf-rust-framework-csp2d" }
ospf-rust-base = { path = "../ospf/ospf-rust/ospf-rust-base" }
ospf-rust-quantities = { path = "../ospf/ospf-rust/ospf-rust-quantities" }
```

For other scenarios replace the domain crate and declare imported crates as direct dependencies. BPP1D also needs Core and Framework BPP; Gantt needs Framework, Core and `time = "0.3"`; Network needs Core; BPP3D needs Framework, Core and domain feature `serde`.

For SCIP launchers enable `scip-bundled` on `ospf-rust-core`. BPP3D/Gantt using `ScipColumnGenerationSolver` also need `scip` on `ospf-rust-framework`. Set these in application dependency declarations, not test filters. BPP2D still requires the two adapters described by its lesson; features alone do not implement them.

## 4. Identify the Actual Execution Path

| Path | Actual execution | Does not establish |
| --- | --- | --- |
| Default heuristic (Rust CSP1D/CSP2D examples) | Real domain flow with heuristic selection | Native optimization or global optimality |
| Native solve (other examples with real adapters connected) | Actual models sent to SCIP/Gurobi | Delivery or proof scope without report/domain checks |
| Optional repository verification | Geometry, registration, orchestration, witness or oracle checks | A real application solve |

### Kotlin Native Backends

SCIP needs compatible JSCIP Java bindings and native libraries. Gurobi needs matching Java/JNI libraries and a valid license. Some POMs use local/custom Maven coordinates, such as `gurobi:gurobi`, that Maven Central may not supply. Follow the plugin README rather than only setting an enablement property.

Native libraries must be on the process loader path. Configure the application JVM with `-Djava.library.path=...` if necessary; lessons do not hard-code a developer's installation directory.

Repository native verification may additionally require `JSCIP_HOME` and `SCIP_HOME`; these do not replace the Java binding and loader configuration of a standalone application.

- [SCIP plugin setup](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-scip/README.md)
- [Gurobi plugin setup](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-gurobi/README.md)

### Rust Native Backends

Features such as `scip-bundled`, `scip-from-source` and `gurobi10` select different integrations. Gurobi still needs a matching installation and license; bundled SCIP may also download native dependencies. Preserve additional required features, such as `serde` for BPP3D CSV cases.

See [Rust backend setup](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/README.md). Do not enable every feature to work around a single build failure.

## 5. Troubleshooting

| Symptom | Check first |
| --- | --- |
| Maven / Cargo cannot resolve a dependency | Revision, registry, network and local binding artifacts; business solving has not started |
| Zero or skipped tests | Filter, profile, feature and runtime probes; a skip is not a pass |
| Native library loading fails | Binding version, architecture, loader paths and transitive native dependencies |
| Business input is rejected | Unique IDs, counts/dimensions, units, inventory and configuration |
| Normal termination without a plan | Distinguish infeasibility, limits without an incumbent and technical errors |
| A solution without an optimality proof | Candidate truncation, heuristic pricing or early termination; do not upgrade the status |

Write build logs to a temporary directory and inspect the same complete log after a failure instead of rerunning the build to capture a few lines. Return to [framework selection](./) to start a lesson; after a successful baseline, continue to [business integration](./integration).
