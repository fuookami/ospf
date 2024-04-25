# ospf-kotlin

[![GitHub license](https://img.shields.io/badge/license-Apache%20License%202.0-green.svg?style=flat)](http://www.apache.org/licenses/LICENSE-2.0)
[![Maven Central](https://img.shields.io/maven-central/v/io.github.fuookami.ospf.kotlin/ospf-kotlin)](https://mvnrepository.com/artifact/io.github.fuookami.ospf.kotlin/ospf-kotlin)
[![Kotlin](https://img.shields.io/badge/kotlin-1.9.22-yellow.svg?logo=kotlin)](http://kotlinlang.org)

## 介绍

ospf 是一个针对复杂的运筹优化算法中建模与编码过程的解决方案及其开发组件。ospf 旨在提供一种基于<strong><em>领域驱动设计</em></strong>（DDD）的建模方式，以便使用者能够在整个软件生命周期都能简单、高效地开发、维护数学模型、求解算法及其实现代码。更详细的介绍、设计以及文档可以参考文档页面：

文档：https://fuookami.github.io/ospf/

:us: [English](README.md) | :cn: 简体中文

## 中间值

ospf 提供了一种命名为“中间值”的概念，以实现基于 DDD 的建模方式。中间值在数学模型中用于表示运算的中间结果，它可以帮助简化模型的表示，并使得模型更易于理解和维护。中间值有以下特性：

- 指代一个被存储起来的具名的表达式
- 语义上等价于匿名的表达式
- 文法上等价于变量，拥有全局作用域以及静态生命周期

### 算术中间值

中间值最开始的设计目的是为了减少数学模型中的重复，所以最基本的算术中间值就是通过一个多项式来构建，然后使用者就可以在模型的任何地方使用该中间值替代所有同样的多项式。

$$
ExprSymbol = \sum_{i} x_{i}
$$

$$
min \quad ExprSymbol
$$

$$
s.t. \quad ExprSymbol \leq 1
$$

ospf 会在将模型翻译到具体求解器的接口时，自动将把每个算术中间值替换为具体的多项式，这个翻译过程对于使用者而言是无感知的，因此使用者并不需要知道这个算术中间值是通过什么变量通过什么运算实现的。

那么，我们就可以把数学模型的维护者划分为“中间值维护者”以及“使用中间值维护数学模型者”两个角色。中间值维护者负责定义以及实现中间值，使用中间值维护数学模型者不关注中间值的实现，只关注中间值的定义与行为，并使用这些中间值在数学模型中描述业务逻辑。

这个工程实践，和面向对象设计（OOD）中定义一个类把相同语义的变量、函数封装起来，使用者只需关注其行为，无需关注其实现，是一样的。有了这样的基础之后，我们就可以开始引入 DDD 了。

### 函数中间值

基于算术中间值的思想，ospf 同样可以把类似逻辑运算表达式等非算术表达式封装到中间值中。

$$
FuncSymbol = \bigvee_{i} x_{i} = Or(x_{1}, \\, x_{2}, \\, .. \\, , \\, x_{i})
$$

$$
s.t. \quad FuncSymbol = 1
$$

ospf 会在将模型翻译到具体求解器的接口时，自动添加每个函数中间值所需的中间变量以及约束。这个翻译过程对于使用者而言是无感知的，因此使用者并不需要知道这个函数中间值是通过什么中间变量以及约束实现的。比如上面的这个 $FuncSymbol = \bigvee_{i} x_{i}$ 就会被翻译成：

$$
s.t. \quad y = 1
$$

$$
\begin{cases}
  y \geq \frac{x_{i}}{\sup_{\leq}(x_{i})}, & \sup_{\leq}(x_{i}) > 1 \\\\\\
  y \geq x_{i}, & else
\end{cases}
$$

$$
y \leq \sum_{i} x_{i}
$$

$$
y \in \\{ 0, 1 \\}
$$

当然，你也可以根据自己的业务需求，拓展这些函数中间值。这个时候你要实现一些接口，以让 ospf 知道这个函数中间值需要添加哪些中间变量以及约束。

ospf-core 本身只维护有算术运算符以及逻辑运算符，实际上我们完全可以基于领域去设计并实现函数中间值，以作为领域工程的一部分。具体可以参考 ospf-framework 中面向特定问题的开发包。

## 组件

ospf 采用内部<strong><em>领域特定语言</em></strong>（DSL） 的形式进行设计与实现，除了部分公共组件外，其余部分均在目标宿主语言上实现。

### 公共组件

- examples: 样例，用于展示如何使用 ospf 进行建模、求解。

- framework: 面向特定问题开发包的公用组件，包含结果可视化工具。

- remote: 远程求解调度器与服务端，用于在服务器上运行求解器，并通过网络接口获取结果。

### 宿主语言实现组件

每一个 ospf 实现包含以下组件：

- <strong>utils</strong>: 工具集，包含实现 ospf dsl 所需的类与函数。
- <strong>core</strong>: 核心组件，包含建模、求解器接口、结果处理等核心功能。
  - <strong>core-plugin-XXX</strong>: 求解器插件，用于实现面向具体求解器的求解器接口。
  - <strong>core-plugin-heuristic</strong>: 元启发式算法插件，包含了许多通用的元启发式算法的实现。
- <strong>framework</strong>: 面向特定问题的框架，包含了面向特定问题的数据处理、数学模型以及求解算法的实现。所有设计与实现的都是非侵入式的，用户既可以开箱即用，也可以基于框架进行扩展，可以与其他框架或组件无缝集成。
  - <strong>framework-plugin-XXX</strong>: 框架插件，用于实现需要中间件参与的功能，比如数据持久化、异步消息通信。
  - <strong>bpp1d</strong>: 一维装箱问题开发包，包含了许多一维装箱问题的数据处理、数学模型以及求解算法的实现。
  - <strong>bpp2d</strong>: 二维装箱问题开发包，包含了许多二维装箱问题的数据处理、数学模型以及求解算法的实现。
  - <strong>bpp3d</strong>: 三维装箱问题开发包，包含了许多三维装箱问题的数据处理、数学模型以及求解算法的实现。
  - <strong>csp1d</strong>: 一维下料问题开发包，包含了许多一维下料问题的数据处理、数学模型以及求解算法的实现。
  - <strong>csp2d</strong>: 二维下料问题开发包，包含了许多二维下料问题的数据处理、数学模型以及求解算法的实现。
  - <strong>gantt-scheduling</strong>: 甘特图调度问题开发包，包含了许多甘特图调度问题的数据处理、数学模型以及求解算法的实现。可用于类似生产排程（APS）、批次生产（LSP）等调度、规划问题。
  - <strong>network-scheduling</strong>: 网络调度问题开发包，包含了许多网络调度问题的数据处理、数学模型以及求解算法的实现。可用于类似车辆调度（VRP）、设施选址（FLP）等调度、规划问题。

## 特性与进度

- ✔️：稳定版本。
- ⭕：开发完成，未稳定版本。
- ❗：正在开发，未完成版本。
- ❌：计划中，未开始。

### Core

<div style="width: auto; display: table; margin-left: auto; margin-right: auto;">
  <table style="text-align: center;">
    <thead>
      <tr>
        <th>特性</th>
        <th>C++</th>
        <th>C#</th>
        <th>Kotlin</th>
        <th>Python</th>
        <th>Rust</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td colspan=6>建模语言</td>
      </tr>
      <tr>
        <td>MILP</td>
        <td>❗</td>
        <td>❌</td>
        <td>✔️</td>
        <td>❌</td>
        <td>❗</td>
      </tr>
      <tr>
        <td>MIQCQP</td>
        <td>❌</td>
        <td>❌</td>
        <td>✔️</td>
        <td>❌</td>
        <td>❌</td>
      </tr>
      <tr>
        <td>MINLP</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
      </tr>
      <tr>
        <td colspan=6>求解器接口</td>
      </tr>
      <tr>
        <td>GUROBI</td>
        <td>❗</td>
        <td>❌</td>
        <td>✔️</td>
        <td>❌</td>
        <td>❗</td>
      </tr>
      <tr>
        <td>CPLEX</td>
        <td>❗</td>
        <td>❌</td>
        <td>✔️</td>
        <td>❌</td>
        <td>❗</td>
      </tr>
      <tr>
        <td>COPT</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
      </tr>
      <tr>
        <td>SCIP</td>
        <td>❗</td>
        <td>❌</td>
        <td>✔️</td>
        <td>❌</td>
        <td>❗</td>
      </tr>
      <tr>
        <td>COPIN-OR</td>
        <td>❗</td>
        <td>❌</td>
        <td>❗</td>
        <td>❌</td>
        <td>❗</td>
      </tr>
      <tr>
        <td>其它</td>
        <td colspan=5>计划中</td>
      </tr>
      <tr>
        <td colspan=6>元启发式算法</td>
      </tr>
      <tr>
        <td>PSO</td>
        <td>❗</td>
        <td>❌</td>
        <td>✔️</td>
        <td>❌</td>
        <td>❗</td>
      </tr>
      <tr>
        <td>GA</td>
        <td>❗</td>
        <td>❌</td>
        <td>❗</td>
        <td>❌</td>
        <td>❗</td>
      </tr>
      <tr>
        <td>其它</td>
        <td colspan=5>计划中</td>
      </tr>
    </tbody>
  </table>
</div>

### Framework

<div style="width: auto; display: table; margin-left: auto; margin-right: auto;">
  <table style="text-align: center;">
    <thead>
      <tr>
        <th>特性</th>
        <th>C++</th>
        <th>C#</th>
        <th>Kotlin</th>
        <th>Python</th>
        <th>Rust</th>
        <th>可视化</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>基础框架</td>
        <td>❗</td>
        <td>❌</td>
        <td>✔️</td>
        <td>❌</td>
        <td>❗</td>
        <td></td>
      </tr>
      <tr>
        <td>一维装箱</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
      </tr>
      <tr>
        <td>二维装箱</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
      </tr>
      <tr>
        <td>三维装箱</td>
        <td>❌</td>
        <td>❌</td>
        <td>⭕</td>
        <td>❌</td>
        <td>❌</td>
        <td>✔️</td>
      </tr>
      <tr>
        <td>一维下料</td>
        <td>❌</td>
        <td>❌</td>
        <td>⭕</td>
        <td>❌</td>
        <td>❌</td>
        <td>✔️</td>
      </tr>
      <tr>
        <td>二维下料</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
      </tr>
      <tr>
        <td>甘特图调度</td>
        <td>❌</td>
        <td>❌</td>
        <td>⭕</td>
        <td>❌</td>
        <td>❌</td>
        <td>✔️</td>
      </tr>
      <tr>
        <td>网络流调度</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
        <td>❌</td>
      </tr>
      <tr>
        <td>其它</td>
        <td colspan=6>计划中</td>
      </tr>
    </tbody>
  </table>
</div>

### Remote

<div style="width: auto; display: table; margin-left: auto; margin-right: auto;">
  <table style="text-align: center;">
    <thead>
      <tr>
        <th>特性</th>
        <td></th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>求解器服务端</td>
        <td>❗</td>
      </tr>
      <tr>
        <td>元启发式算法服务端</td>
        <td>❌</td>
      </tr>
      <tr>
        <td>调度器</td>
        <td>❗</td>
      </tr>
      <tr>
        <td>时间片轮转</td>
        <td>❌</td>
      </tr>
    </tbody>
  </table>
</div>
