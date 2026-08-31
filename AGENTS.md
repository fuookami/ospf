# AGENTS

## 作用范围
本文件适用于当前目录及其所有子目录。

## 规则来源
请遵循 `.rules/` 下实际存在的规则文件：

- `.rules/chore.md`
- `.rules/framework-architecture.md`

## 明确优先级顺序（从高到低）
1. `.rules/chore.md`
2. `.rules/framework-architecture.md`

## 冲突处理
当规则冲突时，严格按上述优先级顺序执行。

## 语言要求
始终使用简体中文与用户对话。

## 提交信息要求
进行 `git commit` 或 `git commit --amend` 时，并且内容要具体、完整，清晰说明改动目的与关键变更点，避免过于简短或笼统的描述。
提交信息必须包含符合 Conventional Commit 风格的 Header。

## 命令行环境优先级

执行命令行操作时，优先使用 **PowerShell 7**（命令名 `pwsh`）或 **git bash**，而非 cmd.exe 或旧版 Windows PowerShell（5.x）。

- 优先级：`pwsh` > git bash > cmd.exe / Windows PowerShell 5.x
- `pwsh` 即 PowerShell 7+，跨平台二进制名统一为 `pwsh`（不同于 Windows PowerShell 5.x 的 `powershell`）
- `pwsh` 支持 UTF-8 默认编码、更好的管道对象模型、`||`/`&&` 链式操作符
- git bash 提供 Unix-like 工具链（grep、sed、awk 等），适合脚本操作
- 避免使用旧版 `powershell.exe`（Windows PowerShell 5.x），其编码和兼容性问题较多
- 当 AGENTS.md 中的示例使用 bash 语法时，在 git bash 中直接执行；若需在 pwsh 中执行，注意语法差异（如变量引用 `$env:VAR` vs `$VAR`、数组 `@()` vs `()` 等）
