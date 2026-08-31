# symbol/parser

:us: English | :cn: [简体中文](README_ch.md)

多项式和不等式的字符串解析器。需要启用 `parser` feature flag。

## 架构

解析器分为三个阶段：

1. **词法分析器**（`Lexer`）— 将字符串转换为 token 序列
2. **语法分析器**（`Parser`）— 将 token 序列转换为表达式树
3. **语义分析** — 将表达式树转换为多项式类型

## 核心类型

| 类型 | 说明 |
|------|------|
| `Lexer` | 词法分析器，将输入字符串分词 |
| `Token` | 词法分析器产生的 token 类型 |
| `Parser` | 语法分析器，构建表达式树 |
| `Expr` | 表达式树节点 |
| `ExprKind` | 表达式种类枚举 |
| `ParseError` | 解析错误类型 |
| `ParseResult` | 解析操作的 Result 别名 |

## 使用示例

```rust
// 需要启用 `parser` feature
// use ospf_rust_math::symbol::parser::{Lexer, Parser};
```

## 许可证

MIT License
