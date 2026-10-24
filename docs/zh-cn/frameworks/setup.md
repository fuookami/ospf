# 环境与运行方式

每个第一课先给业务输入、完整示例函数和独立程序入口。你可以把它放进自己的小型应用中学习调用，不需要先运行仓库测试。此页只说明依赖、工具链和真实后端；回归命令另见[仓库验证参考](./verification)。

## 1. 准备源码

从总仓库根目录执行：

```powershell
git clone https://github.com/fuookami/ospf.git
Set-Location ospf
```

源码构建命令在对应语言仓库中执行；业务示例在你自己的应用目录启动。后文的 path 依赖示例假定应用目录与 `ospf` checkout 同级；如果目录不同，修改依赖路径。也可以单独 checkout 对应语言仓库，保持与教程链接的源码版本一致。

::: warning 版本与发布不是一回事
本组教程按当前源码布局编写：Kotlin 根 POM 为 `1.1.2`，Rust 领域 crate 多为 `1.1.0`。这不证明对应版本已发布到 Maven Central 或 crates.io。不要把 Kotlin 版本号复制到 Rust，也不要把历史 Guide 中的依赖版本与新源码混用。
:::

## 2. Kotlin

准备 JDK 25、Maven，以及能解析 POM 中依赖的 Maven 仓库配置。当前根 POM 的 JVM target 为 25，旧 JDK 无法运行这些编译产物；除 `java -version` 外，还要确认 `mvn -version` 使用了预期 JDK。从源码集成时只构建/安装需要的模块及其 reactor 依赖，不必先构建全部业务框架。

```powershell
# 工作目录：ospf-kotlin
java -version
mvn -version
```

独立业务应用可选用领域 starter，求解器插件需另行配置：

| 场景 | Kotlin starter artifact |
| --- | --- |
| BPP1D / BPP2D / BPP3D | `ospf-kotlin-starter-bpp1d` / `ospf-kotlin-starter-bpp2d` / `ospf-kotlin-starter-bpp3d` |
| CSP1D / CSP2D | `ospf-kotlin-starter-csp1d` / `ospf-kotlin-starter-csp2d` |
| Gantt / Network | `ospf-kotlin-starter-gantt-scheduling` / `ospf-kotlin-starter-network-scheduling` |

这些 starter 的 groupId 为 `io.github.fuookami.ospf.kotlin`。使用已确认可获取的版本；从源码集成时先安装所需 reactor 模块。具体依赖见 [Kotlin starters](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-starters)。

### Kotlin 应用最少接哪几部分？

领域 starter 提供业务 API；原生插件提供实际求解器；`kotlinx-coroutines-core` 提供 `runBlocking`。以 BPP1D + Gurobi 为例，Maven dependencies 如下（源码版本，仅在已安装/可解析时使用）：

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

独立项目还需 Kotlin/JVM 编译插件（当前源码 Kotlin 2.4.20、JVM target 25）。将示例函数与 `main` 放到应用源码目录，以 IDE 的 Kotlin Application 启动 `main`，不需要 JUnit。Gurobi Java 绑定在插件中是 provided 依赖，还需按插件说明显式加入应用编译/运行依赖，并匹配本机 JNI 和许可证；不要把上面三项误认为完整原生安装。

## 3. Rust

当前 workspace 的基础 crate 使用 `specialization` 等不稳定特性，因此这些源码教程使用 nightly 工具链；仅支持 edition 2024 的 stable 仍不足以编译。命令固定使用已通过当前 CSP2D 算例验证的 `nightly-2026-08-01`：较新 nightly 的 allocator API 变化目前会导致该 checkout 编译失败。这是教程的兼容性固定版本，不修改你的全局默认工具链。不要把设置 `RUSTC_BOOTSTRAP` 当作安装求解器的步骤。

```powershell
# 工作目录：ospf-rust
rustup toolchain install nightly-2026-08-01
rustc +nightly-2026-08-01 --version
cargo +nightly-2026-08-01 --version
```

示例函数放进 `src/<框架>.rs`，启动入口放进 `src/main.rs`。在你的应用目录运行 `cargo +nightly-2026-08-01 run`；这会执行应用的 `main`，不是筛选仓库测试。

独立应用按需添加 `ospf-rust-framework-bpp1d` 等领域 crate。源码 checkout 可以使用 Cargo path 依赖；原生后端由 feature 显式启用，具体名称以该 crate 的 `Cargo.toml` 为准。

### 第一个独立 Rust 应用

以不需要原生库的 [CSP2D 示例](./csp2d/first-cutting)开始。在与 `ospf` checkout 同级的 `first-cutting` 目录创建以下 `Cargo.toml`，并放入该课的 `src/csp2d.rs` 与 `src/main.rs`：

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

其他场景替换领域 crate；所有示例的导入也应列为直接依赖。BPP1D 另需 Core、Framework BPP；Gantt 另需 Framework、Core 和 `time = "0.3"`；Network 另需 Core；BPP3D 另需 Framework、Core，并给领域 crate 启用 `serde`。

使用第一课的 SCIP 启动入口时，给 `ospf-rust-core` 启用 `scip-bundled`；使用 `ScipColumnGenerationSolver` 的 BPP3D/Gantt 还需给 `ospf-rust-framework` 启用 `scip`。这些 feature 应放在应用的直接依赖声明中，不是运行测试的开关。BPP2D 还需要实现该课说明的两个适配器，不能仅添加 feature 就当作已接通。

## 4. 识别实际执行方式

| 路径 | 实际执行 | 不能推断的结论 |
| --- | --- | --- |
| 默认启发式（Rust CSP1D/CSP2D 示例） | 框架真实业务流程与启发式选择 | 不证明原生优化或全局最优 |
| 原生求解（其余已接通后端的示例） | 实际模型交给 SCIP/Gurobi | 仍需核对终态、业务可行性和证明范围 |
| 仓库契约验证（可选参考页） | 几何、注册、编排、给定解或枚举等局部验证 | 不替代应用的真实求解入口 |

### Kotlin 原生后端

SCIP 需要兼容的 JSCIP Java 绑定和原生库；Gurobi 需要匹配的 Java/JNI 库及有效许可证。项目中的某些 POM 引用本地或自定义 Maven 坐标（例如 `gurobi:gurobi`），Maven Central 不一定提供。按插件 README 安装/配置，不要仅复制一个系统属性就认为环境已就绪。

原生库目录需要出现在进程的加载路径中；必要时通过应用 JVM 启动参数 `-Djava.library.path=...` 配置加载路径。教程不写入个人机器的安装目录。

若使用仓库原生回归配置，还需按[验证参考](./verification)设置 `JSCIP_HOME` 与 `SCIP_HOME`；这些环境变量不替代独立应用所需的 Java 绑定与动态库配置。

- [SCIP 插件设置](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-scip/README.md)
- [Gurobi 插件设置](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-gurobi/README.md)

### Rust 原生后端

`scip-bundled`、`scip-from-source`、`gurobi10` 等 feature 选择不同集成方式。Gurobi 仍需匹配版本的安装和许可证；bundled SCIP 也可能需要下载原生依赖。保持教程中要求的其他 feature，例如 BPP3D CSV 算例的 `serde`。

见 [Rust 后端说明](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/README.md)。不要为避开一个构建错误而随意启用所有 feature。

## 5. 运行失败时先看哪里

| 现象 | 优先检查 |
| --- | --- |
| Maven / Cargo 依赖无法解析 | 源码版本、仓库源、网络、本地绑定构件；此时尚未进入业务求解 |
| 测试为零或 skipped | 过滤器、profile、feature、运行时探测；跳过不等于通过 |
| 动态库加载失败 | 绑定版本、位数、原生库路径、传递依赖 |
| 业务输入被拒绝 | ID 唯一性、数量/尺寸、单位、库存与配置 |
| 正常终态但没有方案 | 不可行、限时无 incumbent 与技术错误需分别处理 |
| 有方案但没有最优证明 | 候选截断、启发式定价或提前终止；不要自行升级状态 |

将构建日志写到临时目录，失败后分析同一份完整日志，不要反复启动构建截取少量错误。回到[框架选型](./)开始第一课；已有成功基准后，继续[接入自己的业务](./integration)。
