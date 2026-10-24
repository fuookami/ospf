# 环境与运行方式

每个第一课先给业务输入、完整示例函数和独立程序入口。你可以把它放进自己的小型应用中学习调用，不需要先运行仓库测试。此页只说明依赖、工具链和真实后端；回归命令另见[仓库验证参考](./verification)。

## 1. 准备源码

从总仓库根目录执行：

```powershell
git clone https://github.com/fuookami/ospf.git
Set-Location ospf
```

源码构建命令在对应语言仓库中执行；业务示例在你自己的应用目录启动。后文的本地路径依赖示例假定应用目录与 `ospf` 源码目录同级；如果目录不同，修改依赖路径。也可以单独克隆对应语言仓库，保持与教程链接的源码版本一致。

::: warning 版本与发布不是一回事
本组教程按当前源码布局编写：Kotlin 根 POM 为 `1.1.2`，Rust 领域包（crate）多为 `1.1.0`。使用前需确认对应版本是否已发布到 Maven Central 或 crates.io；从源码集成时使用本地构建产物。Kotlin 与 Rust 的版本号分别管理，旧版指南中的依赖版本也可能与当前源码不同。
:::

## 2. Kotlin

准备 JDK 25、Maven，以及能解析 POM 中依赖的 Maven 仓库配置。当前根 POM 的 JVM 目标版本为 25，旧 JDK 无法运行这些编译产物；除 `java -version` 外，还要确认 `mvn -version` 使用了预期 JDK。从源码集成时只构建、安装所需模块及其依赖模块，不必先构建全部业务框架。

```powershell
# 工作目录：ospf-kotlin
java -version
mvn -version
```

独立业务应用可选用领域集成依赖（starter），求解器插件需另行配置：

| 场景 | Kotlin 集成依赖名称 |
| --- | --- |
| BPP1D / BPP2D / BPP3D | `ospf-kotlin-starter-bpp1d` / `ospf-kotlin-starter-bpp2d` / `ospf-kotlin-starter-bpp3d` |
| CSP1D / CSP2D | `ospf-kotlin-starter-csp1d` / `ospf-kotlin-starter-csp2d` |
| Gantt / Network | `ospf-kotlin-starter-gantt-scheduling` / `ospf-kotlin-starter-network-scheduling` |

这些集成依赖的 groupId 为 `io.github.fuookami.ospf.kotlin`。使用已确认可获取的版本；从源码集成时先安装所需 Maven 模块。具体依赖见 [Kotlin 集成依赖](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-starters)。

### Kotlin 应用最少接哪几部分？

领域集成依赖提供业务 API；原生插件提供实际求解器；`kotlinx-coroutines-core` 提供 `runBlocking`。以 BPP1D + Gurobi 为例，Maven 依赖如下（源码版本，仅在已安装或可解析时使用）：

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

独立项目还需 Kotlin/JVM 编译插件（当前源码使用 Kotlin 2.4.20、JVM 目标版本 25）。将示例函数与 `main` 放到应用源码目录，以 IDE 的 Kotlin Application 启动 `main`，不需要 JUnit。Gurobi Java 绑定在插件中使用 `provided` 依赖范围，因此应用还需按插件说明添加编译、运行依赖，并配置匹配的本机 JNI 库和许可证；上面的 Maven 依赖不能替代这些安装步骤。

## 3. Rust

当前工作区的基础包使用 `specialization` 等不稳定特性，因此这些源码教程使用 nightly（每日构建版）工具链；仅支持 edition 2024 的 stable（稳定版）仍不足以编译。命令固定使用已通过当前 CSP2D 算例验证的 `nightly-2026-08-01`：较新 nightly 的分配器 API 变化目前会导致当前源码编译失败。该固定版本用于教程兼容性验证，不修改你的全局默认工具链。`RUSTC_BOOTSTRAP` 不能替代求解器的安装和配置。

```powershell
# 工作目录：ospf-rust
rustup toolchain install nightly-2026-08-01
rustc +nightly-2026-08-01 --version
cargo +nightly-2026-08-01 --version
```

示例函数放进 `src/<框架>.rs`，启动入口放进 `src/main.rs`。在你的应用目录运行 `cargo +nightly-2026-08-01 run`；这会执行应用的 `main`，不是筛选仓库测试。

独立应用按需添加 `ospf-rust-framework-bpp1d` 等领域包。使用本地源码时可声明 Cargo 路径依赖；原生后端通过功能开关（feature）显式启用，具体名称以该包的 `Cargo.toml` 为准。

### 第一个独立 Rust 应用

以不需要原生库的 [CSP2D 示例](./csp2d/first-cutting)开始。在与 `ospf` 源码目录同级的 `first-cutting` 目录创建以下 `Cargo.toml`，并放入该课的 `src/csp2d.rs` 与 `src/main.rs`：

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

其他场景替换领域包；所有示例的导入也应列为直接依赖。BPP1D 另需 Core、Framework BPP；Gantt 另需 Framework、Core 和 `time = "0.3"`；Network 另需 Core；BPP3D 另需 Framework、Core，并给领域包启用 `serde`。

使用第一课的 SCIP 启动入口时，给 `ospf-rust-core` 启用 `scip-bundled`；使用 `ScipColumnGenerationSolver` 的 BPP3D/Gantt 还需给 `ospf-rust-framework` 启用 `scip`。这些功能开关应放在应用的直接依赖声明中。BPP2D 还需要实现该课说明的两个适配器，功能开关只选择依赖，不会自动生成适配器实现。

## 4. 识别实际执行方式

| 路径 | 实际执行 | 不能推断的结论 |
| --- | --- | --- |
| 默认启发式（Rust CSP1D/CSP2D 示例） | 框架真实业务流程与启发式选择 | 不证明原生优化或全局最优 |
| 原生求解（其余已接通后端的示例） | 实际模型交给 SCIP/Gurobi | 仍需核对求解状态、业务可行性和证明范围 |
| 仓库测试（可选参考页） | 几何、注册、求解流程、给定解或枚举等局部验证 | 不替代应用的真实求解入口 |

### Kotlin 原生后端

SCIP 需要兼容的 JSCIP Java 绑定和原生库；Gurobi 需要匹配的 Java/JNI 库及有效许可证。项目中的某些 POM 引用本地或自定义 Maven 坐标（例如 `gurobi:gurobi`），Maven Central 不一定提供。按插件 README 安装/配置，不要仅复制一个系统属性就认为环境已就绪。

原生库目录需要出现在进程的加载路径中；必要时通过应用 JVM 启动参数 `-Djava.library.path=...` 配置加载路径。教程不写入个人机器的安装目录。

若使用仓库原生回归配置，还需按[验证参考](./verification)设置 `JSCIP_HOME` 与 `SCIP_HOME`；这些环境变量不替代独立应用所需的 Java 绑定与动态库配置。

- [SCIP 插件设置](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-scip/README.md)
- [Gurobi 插件设置](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-gurobi/README.md)

### Rust 原生后端

`scip-bundled`、`scip-from-source`、`gurobi10` 等功能开关选择不同集成方式。Gurobi 仍需匹配版本的安装和许可证；使用预打包 SCIP 也可能需要下载原生依赖。保持教程中要求的其他功能开关，例如 BPP3D CSV 算例的 `serde`。

见 [Rust 后端说明](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/README.md)。不要为避开一个构建错误而随意启用所有功能开关。

## 5. 运行失败时先看哪里

| 现象 | 优先检查 |
| --- | --- |
| Maven / Cargo 依赖无法解析 | 源码版本、仓库源、网络、本地绑定构件；此时尚未进入业务求解 |
| 未运行测试或测试被跳过 | 测试筛选条件、Maven 配置、功能开关、运行时环境检查；跳过不等于通过 |
| 动态库加载失败 | 绑定版本、位数、原生库路径、传递依赖 |
| 业务输入被拒绝 | ID 唯一性、数量/尺寸、单位、库存与配置 |
| 求解正常结束但没有方案 | 区分已证明不可行、达到时限但未找到可行解，以及技术错误 |
| 有方案但没有最优证明 | 候选截断、启发式定价或提前终止；不要自行升级状态 |

将构建日志写到临时目录，失败后分析同一份完整日志，不要反复启动构建截取少量错误。回到[框架选型](./)开始第一课；已有成功基准后，继续[接入自己的业务](./integration)。
