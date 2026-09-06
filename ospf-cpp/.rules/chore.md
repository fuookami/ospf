# 项目规范

## 1. 编码风格

### 1.1 注释语言
编写注释时，要中英双语。中文在前，英文在后，单行用 ` / ` 分隔，多行分行书写。

```cpp
// 基本模型 / Basic Model
///
/// 只包含变量和约束的基本模型层，不包含目标函数。
/// Basic model layer containing only variables and constraints, without objective.
```

### 1.2 版权声明
不需要添加版权声明。项目级许可统一在仓库根目录 `LICENSE` 文件中维护。

### 1.3 ReadMe 文件
英文 ReadMe：`README.md`，中文 ReadMe：`README_ch.md`，要添加超链接能互相跳转。

### 1.4 Shell 工具
PowerShell 用：`pwsh.exe`。

### 1.5 函数参数规范
超过 2 个参数时，优先使用多行参数列表。当参数具有配置性质时，使用结构体聚合参数。

```cpp
// 少量简单参数，单行
auto add_variable(const std::string &name, V value) -> Result<std::size_t>;

// 超过 2 个参数，多行
auto register_auto_variable_with_range(
    const std::string &name,
    const VariableRange<VT::value_type> &range
) -> Result<std::size_t>;

// 配置性质参数，使用结构体
auto config = ModelConfig{
    .name = "my_model",
    .tolerance = 1e-6,
    .max_iterations = 1000,
};
model.build(config);
```

### 1.6 C++ 文件排版与 include 规范

本节约束 C++ 源文件的基础排版风格。重构时应优先保持本项目既有手写风格，不使用会大幅改写 include、空行和换行的自动格式化结果作为最终形态。

#### 1.6.1 文件整体结构

文件各部分的排列顺序和间距必须遵循以下规范：

1. 对应头文件（`.cpp` 文件中）放在最开头。
2. `#include` 语句块与上方声明之间空一行。
3. 最后一个 `#include` 语句与首个顶层声明（`namespace`、`class`、`function` 等）之间空一行。
4. 文件末尾必须有且仅有一个换行符。

#### 1.6.2 include 排列

**整体顺序**：

1. 所有 `#include` 按来源分组，组间以一个空行分隔：
   - 第一组：对应头文件（仅 `.cpp` 文件）
   - 第二组：C / C++ 标准库（如 `<vector>`、`<string>`、`<memory>`）
   - 第三组：第三方库（如 `<fmt/format.h>`、`<gtest/gtest.h>`、`<nlohmann/json.hpp>`）
   - 第四组：项目内部模块（`ospf_base` → `ospf_multiarray` → `ospf_math` → `ospf_quantities` → `ospf_core` → `ospf_framework`）
2. 项目内部模块按依赖层级排列，从底层到上层：
   - `ospf_base`（基础工具，被所有模块依赖）
   - `ospf_multiarray`（多维数组）
   - `ospf_math`（数学库）
   - `ospf_quantities`（物理量）
   - `ospf_core`（优化核心）
   - `ospf_framework`（应用框架）
3. 同一模块层级内，按路径字典序排列。
4. 禁止在 `.h` / `.hpp` 文件中使用 `using namespace`。
5. `.cpp` 文件中允许在函数体内使用 `using namespace`，但禁止在全局作用域使用（`std::literals` 等字面量命名空间除外）。

**正确示例**（以 `ospf_core` 模块中的 `.cpp` 文件为例）：

```cpp
#include "ospf/core/model/mechanism.h"

#include <memory>
#include <string>
#include <vector>

#include <fmt/format.h>

#include "ospf/base/error.h"
#include "ospf/base/functional.h"
#include "ospf/math/algebra/number.h"
#include "ospf/math/symbol/monomial.h"
#include "ospf/core/variable.h"
#include "ospf/core/model/mechanism.h"
```

说明：
- 对应头文件 → 标准库 → 第三方库 → `base`（最底层）→ `math` → `core`（当前模块）

#### 1.6.3 换行、缩进与空行

**基础规则**：

- 使用 4 空格缩进，不使用 tab。
- 顶层声明之间保留一行空行。
- class 内部 public / protected / private 段落之间保留一行空行。
- 成员函数之间保留一行空行。
- struct/class 定义结束前不保留多余空行。
- 禁止保留重复 Doxygen 注释或连续重复注释块。

**函数声明与调用**：

- 1–2 个参数且语义简单时可单行书写。
- 超过 2 个参数时，按 `1.5 函数参数规范` 使用多行。
- 多行参数列表中，参数缩进一层；闭合括号与函数签名起始位置对齐。

**模板参数与 requires 子句**：

- 简短模板约束可内联写在模板参数声明处。
- 复杂或多个约束必须使用 `requires` 子句，每个约束独占一行。

```cpp
// 简短约束内联
template<std::copyable V>
auto add(const V &other) -> V;

// 复杂约束使用 requires 子句
template<typename V>
    requires std::regular<V>
          && std::convertible_to<V, double>
          && requires(V a, V b) { { a + b } -> std::same_as<V>; }
auto checked_add(const Quantity<V, Unit> &other) const -> Result<Quantity<V, Unit>>;
```

#### 1.6.4 注释与分段

- 公共 class、struct、enum、重要公共方法使用中英双语 Doxygen 注释（`///` 或 `/** */`）。
- 模块级文档使用 `/// @file`。
- 简短成员可使用单行双语注释，如 `/// 模型名称 / Model name`。
- 大段逻辑分隔允许使用 `// ============================================================================` 形式分段，并附双语说明。
- 注释应说明业务意图、加载态语义或分段边界；避免重复描述代码本身。

#### 1.6.5 文档注释覆盖要求

- 公共 class / struct 的每个公共成员必须有文档注释。
- 公共函数的参数和返回值语义不明显时，应在文档注释中说明。
- 模板类型参数语义非常规时应在文档注释中说明。

### 1.7 泛型化命名规范

对外 API 使用业务自然名表达稳定抽象，不使用迁移期技术命名。

- 泛型化后的主 class、主 struct、主服务占用自然名。
- 不使用 `V`、`Typed`、`Generic` 作为迁移痕迹型前后缀（模板参数 `V` 本身除外）。
- 需要保留的 `Flt64` 专用接口、桥接接口或兼容入口，使用 `Flt64` 后缀显式标识。
- 模板参数使用单个大写字母缩写（`V`、`U`、`T`、`I`），concept 使用全称。
- 内部变量、测试名、文档示例应尽量同步上述命名，避免保留迁移期表达。

### 1.8 C++ 惯用法

- 优先使用 RAII 管理资源，避免裸 `new` / `delete`。
- 优先使用智能指针（`std::unique_ptr` > `std::shared_ptr`），明确所有权语义。
- 优先使用 `std::optional`、`std::variant`、`std::expected`（C++23）或自定义 Result 类型表达可能失败的操作，避免异常作为常规控制流。
- 优先使用 STL 算法和范围（C++20 `std::ranges`）代替手写循环，性能敏感时除外。
- 使用 `constexpr` / `consteval` 在编译期完成尽可能多的计算。
- 使用 `[[nodiscard]]`、`[[maybe_unused]]`、`[[likely]]` / `[[unlikely]]` 等属性表达意图。
- 使用 `noexcept` 明确标记不抛异常的函数。
- 测试统一放在对应模块的 `tests/` 目录下，使用 Google Test 框架。
- 头文件使用 `#pragma once` 作为 include guard。
- 命名规范：类名 `PascalCase`，函数/变量 `snake_case`，常量 `kPascalCase`，模板参数单个大写字母或 `PascalCase`。
