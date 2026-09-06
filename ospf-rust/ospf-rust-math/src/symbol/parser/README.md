# symbol/parser

:us: English | :cn: [简体中文](README_ch.md)

String parser for polynomials and inequalities. Requires the `parser` feature flag.

## Architecture

The parser operates in three stages:

1. **Lexer** (`Lexer`) — converts input string to token sequence
2. **Parser** (`Parser`) — converts token sequence to expression tree
3. **Semantic analysis** — converts expression tree to polynomial types

## Key Types

| Type | Description |
|------|-------------|
| `Lexer` | Lexical analyzer, tokenizes input strings |
| `Token` | Token types produced by the lexer |
| `Parser` | Syntax parser, builds expression trees |
| `Expr` | Expression tree node |
| `ExprKind` | Expression kind enum |
| `ParseError` | Parse error type |
| `ParseResult` | Result alias for parse operations |

## Usage

```rust
// Requires `parser` feature
// use ospf_rust_math::symbol::parser::{Lexer, Parser};
```

## License

MIT License
