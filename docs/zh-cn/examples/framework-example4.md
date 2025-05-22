# 复杂示例 4：航班恢复问题

## 问题描述

## 业务架构

```mermaid
C4Context
  System_Boundary(应用层, "应用层") {
    System(客航应用, "客航应用")
    System(货航应用, "货航应用")
  }
  System_Boundary(领域层, "领域层") {
    System(乘客上下文, "乘客上下文")
    System(编排上下文, "编排上下文")
    System(货物上下文, "货物上下文")
    System(任务上下文, "任务上下文")
    System(规则上下文, "规则上下文")
  }

  Rel(客航应用, 乘客上下文, "", "")
  Rel(货航应用, 货物上下文, "", "")
  Rel(乘客上下文, 编排上下文, "", "")
  Rel(货物上下文, 编排上下文, "", "")
  Rel(编排上下文, 任务上下文, "", "")
  Rel(编排上下文, 规则上下文, "", "")

  UpdateLayoutConfig($c4ShapeInRow="5", $c4BoundaryInRow="1")
```

## 数学模型

### RMP

#### 编排上下文

#### 乘客上下文

#### 货物上下文

### SP

## 代码实现

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo4)
