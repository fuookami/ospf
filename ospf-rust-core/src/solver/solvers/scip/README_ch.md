# SCIP 求解器说明

:us: [English](README.md) | :cn: 简体中文

## 前置依赖

`ospf-rust-core` 在 `scip` feature 下通过 `russcip/scip-sys` 绑定 SCIP。  
运行测试前，必须让构建系统能找到 SCIP 头文件与库文件。

## CP 边界

Gantt task-compilation component 负责构造 CP snapshot。SCIP 只为声明的精确子集消费 feature-gated
MIP-backed `ExactLowering` facade，不作为 native CP backend 暴露。optional/variable-duration
interval binding 仍为 `Unsupported`，Cumulative raw-handler 路径在独立安全合同满足前仍为
`Conditional`。

## 环境配置

### Windows（PowerShell）

```powershell
$env:SCIPOPTDIR = "C:\path\to\scip"
```

`SCIPOPTDIR` 需要指向 SCIP 安装根目录。

若 `bindgen` 报找不到 `libclang`，请安装 LLVM 并设置：

```powershell
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
```

### Linux/macOS

```bash
export SCIPOPTDIR=/path/to/scip
export LIBCLANG_PATH=/path/to/llvm/lib
```

## 最小验证命令

在 workspace 根目录执行：

```powershell
# Windows PowerShell
.\ospf-rust-core\src\solver\solvers\scip\check_non_bundled_env.ps1
.\ospf-rust-core\src\solver\solvers\scip\run_non_bundled_regression.ps1
```

```bash
# Linux/macOS
bash ./ospf-rust-core/src/solver/solvers/scip/check_non_bundled_env.sh
bash ./ospf-rust-core/src/solver/solvers/scip/run_non_bundled_regression.sh
```

或手工执行命令：

```bash
cargo test -p ospf-rust-core scip:: --features scip -- --nocapture
cargo test -p ospf-rust-framework scip_extension --features scip -- --nocapture
```

若本机未安装 SCIP，可用 bundled 模式：

```bash
cargo test -p ospf-rust-core scip:: --features scip-bundled -- --nocapture
cargo test -p ospf-rust-framework scip_extension --features scip-bundled -- --nocapture
```

## 常见报错

若出现 `scip-sys` 的 `Could not find SCIP installation`，请依次检查：

1. 当前 shell 是否设置了 `SCIPOPTDIR`。
2. 指定目录下是否包含 SCIP 头文件与库文件。
3. 测试命令是否启用了 `scip`（或 `scip-bundled`）feature。

若出现 `Unable to find libclang`，请设置 `LIBCLANG_PATH` 到 LLVM 的 `bin/lib` 目录。

## Native Callback 快速示例

```rust,ignore
use std::sync::Arc;
use ospf_rust_core::solver::solvers::SCIPSolver;
use ospf_rust_core::solvers::scip::{
    SCIPConfig, SCIPNativeControl, SCIPNativeObserver, SCIPNativeWhere
};

let observer: SCIPNativeObserver = Arc::new(|snapshot| {
    if snapshot.where_point == SCIPNativeWhere::Node {
        // 自定义逻辑
    }
    if snapshot.matches_mask_bits(0x001000000) {
        // 命中 LP 事件掩码
    }
    Ok(SCIPNativeControl::Continue)
});

let solver = SCIPSolver::with_config(
    SCIPConfig::new().add_native_observer(observer)
);
```

说明：在 framework 的 `ColumnGenerationSolver` 接口中，`UserInterrupt` 会以上层错误结果返回。

## CI 首轮排查清单

若 `.github/workflows/scip-bundled.yml` 首轮失败，按以下顺序排查：

1. `Install system deps` 步骤是否成功。
2. `Resolve LIBCLANG_PATH` 是否输出 `Resolved LIBCLANG_SO=...`。
3. `scip-sys` 日志是否显示 bundled SCIP 下载完成。
4. native callback 测试中包含 `UserInterrupt` 的错误文本在部分路径是预期行为；仅异常 panic/assert 才视为回归。
5. 若仍提示找不到 libclang，补充输出：

```bash
ldconfig -p | grep clang || true
find /usr/lib -type f -name 'libclang.so*' | head -n 20
```

## CI 工作流

1. `.github/workflows/scip-bundled.yml`  
   - 在 `ubuntu-latest` 上执行，验证 bundled SCIP 路径。
2. `.github/workflows/scip-non-bundled-self-hosted.yml`  
   - 手动触发（`workflow_dispatch`），并通过 `target_os` 选择 `linux`/`windows` 的 `self-hosted` runner。  
   - 要求 runner 预先配置 `SCIPOPTDIR`（可选配置 `LIBCLANG_PATH`）。  
   - `linux` 路径：先执行 `check_non_bundled_env.sh`，再执行 `run_non_bundled_regression.sh`。  
   - `windows` 路径：先执行 `check_non_bundled_env.ps1`，再执行 `run_non_bundled_regression.ps1`。

### 终端触发（GitHub API）

可通过 PowerShell 脚本 + Token 直接触发：

```powershell
$env:GITHUB_TOKEN = "<具备 actions write 权限的 token>"
.\ospf-rust-core\src\solver\solvers\scip\dispatch_non_bundled_ci.ps1 -TargetOS windows -Ref master -Wait
```

若存在 Linux self-hosted runner，可改为 `-TargetOS linux`。

共享 native contract 会区分真实 SCIP 执行与仅 feature 编译，并比较 report identity、bound、solution
和 residual。未请求原生执行时，缺少或无法加载 SCIP 安装只能标记为 `unsupported`；请求的 native gate
失败时必须显式失败。详见 [`docs/solver-native-matrix_ch.md`](../../../../docs/solver-native-matrix_ch.md)。
