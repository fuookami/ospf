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

- 指代一个被存储起来的具名的多项式
- 语义上等价于匿名的多项式
- 文法上等价于变量，拥有全局作用域以及静态生命周期
- 可以通过一个匿名的多项式构造

<!-- 添加样例 -->

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

### Core

### Framework

### Remote
