# 错误处理规范

## 1. 核心原则

**整个项目统一使用返回错误（Result 模式），不抛出异常。**

所有可能失败的操作都应返回 `Result<T, C, E>` 或其变体，而不是抛出异常。
`C` 为错误码类型，`E` 为错误类型，均通过模板参数泛型化，允许各模块定义自己的领域错误码。

## 2. 错误类型体系

### 2.1 错误码（泛型化）

错误码 `C` 是一个模板参数，不限定为单一枚举。各模块按需定义自己的错误码类型：

```cpp
// ospf_base 层：基础错误码
enum class ErrorCode : std::uint32_t {
    Ok              = 0,
    IllegalArgument = 1,
    IllegalState    = 2,
    NotImplemented  = 3,
    ApplicationFailed = 4,
    ExternalFailed  = 5,
    // ...
};

// 各 framework / domain 模块：领域错误码
enum class Bpp3dErrorCode : std::uint32_t {
    InvalidDimension  = 1001,
    ItemTooLarge      = 1002,
    ContainerOverflow = 1003,
    // ...
};
```

**约束**：
- 错误码类型必须满足 `ErrorCodeTrait` concept（可比较、可输出、可构造）
- 跨模块错误码值不得冲突；建议基础层使用 `0–999`，各领域模块使用 `1000+` 按区块分配

### 2.2 错误类型

`Error<C>` 是模板化的错误基类，`C` 默认为 `ErrorCode`：

```cpp
/// 错误基类 / Error base class
template<ErrorCodeTrait C = ErrorCode>
class Error {
public:
    C           code;       // 错误码 / Error code
    std::string message;    // 错误消息 / Error message

    Error(C code, std::string message);
};

/// 带关联值的错误 / Error with associated value
template<ErrorCodeTrait C = ErrorCode, typename V = void>
class ExError : public Error<C> {
public:
    V value;    // 关联值 / Associated value

    ExError(C code, std::string message, V value);
};
```

**错误变体**（与 Kotlin 的 `Err` / `LazyErr` / `ExErr` / `LazyExErr` 对应）：

| Kotlin | C++ 说明 |
|---|---|
| `Err<C>` | `Error<C>` — 立即构造消息 |
| `LazyErr<C>` | `LazyError<C>` — 延迟消息构造，通过 `std::function<std::string()>` |
| `ExErr<C, T>` | `ExError<C, V>` — 带关联值 |
| `LazyExErr<C, T>` | `LazyExError<C, V>` — 延迟构造 + 关联值 |

### 2.3 结果类型

`Result<T, C, E>` 是核心结果类型，三个模板参数：

- `T` — 成功值类型
- `C` — 错误码类型（默认 `ErrorCode`）
- `E` — 错误类型（默认 `Error<C>`）

```cpp
template<typename T, ErrorCodeTrait C = ErrorCode, typename E = Error<C>>
class Result {
public:
    // 状态 / States
    [[nodiscard]] auto is_ok()     const -> bool;
    [[nodiscard]] auto is_failed() const -> bool;
    [[nodiscard]] auto is_fatal()  const -> bool;

    // 工厂函数 / Factory methods
    static auto ok(T value)                              -> Result;
    static auto failed(C code, std::string message)      -> Result;
    static auto failed(E error)                          -> Result;
    static auto fatal(C code, std::string message)       -> Result;
    static auto fatal(std::vector<E> errors)             -> Result;

    // 访问 / Access
    [[nodiscard]] auto unwrap()      const -> const T &;    // 仅 ok，否则 abort
    [[nodiscard]] auto error()       const -> const E &;    // 仅 failed
    [[nodiscard]] auto errors()      const -> const std::vector<E> &;  // 仅 fatal

    // 链式操作 / Combinators
    template<typename F> auto map(F &&f)         const -> Result<std::invoke_result_t<F, T>, C, E>;
    template<typename F> auto and_then(F &&f)    const -> std::invoke_result_t<F, T>;
    template<typename F> auto inspect_err(F &&f) const -> const Result &;
    template<typename F> auto map_error(F &&f)   const -> Result<T, C, std::invoke_result_t<F, E>>;

    // 期望 / Expect
    auto expect(const char *msg) const -> T;
};
```

**扩展结果类型 `ExResult<T, C, E>`**：在 `Result<T, C, E>` 基础上增加 `Warn` 状态：

```cpp
template<typename T, ErrorCodeTrait C = ErrorCode, typename E = Error<C>>
class ExResult : public Result<T, C, E> {
public:
    static auto warn(T value, C code, std::string message) -> ExResult;
    static auto warn(T value, E warning)                    -> ExResult;

    [[nodiscard]] auto is_warn()  const -> bool;
    [[nodiscard]] auto warning()  const -> const E &;
};
```

### 2.4 类型别名

```cpp
// 基础别名（使用默认 ErrorCode）
using VoidResult = Result<std::monostate>;
template<typename T> using Ret = Result<T>;
using Try = VoidResult;

// 扩展别名
template<typename T> using ExRet = ExResult<T>;
using ExTry = ExResult<std::monostate>;

// 领域别名示例
template<typename T> using Bpp3dResult = Result<T, Bpp3dErrorCode>;
template<typename T> using Bpp3dRet    = Result<T, Bpp3dErrorCode>;
```

## 3. 使用规范

### 3.1 函数签名

```cpp
// 正确：返回泛型 Result（默认 ErrorCode）
auto parse(const std::string &input) -> Ret<ParsedData> {
    if (!is_valid(input)) {
        return Ret<ParsedData>::failed(
            ErrorCode::IllegalArgument,
            "输入格式无效 / Invalid input format"
        );
    }
    return Ret<ParsedData>::ok(parse_data(input));
}

// 正确：返回领域错误码 Result
auto solve(const Bpp3dModel &model) -> Bpp3dResult<Solution> {
    if (model.overflow()) {
        return Bpp3dResult<Solution>::failed(
            Bpp3dErrorCode::ContainerOverflow,
            "容器溢出 / Container overflow"
        );
    }
    return Bpp3dResult<Solution>::ok(run_solver(model));
}

// 正确：返回 Try（无有意义返回值）
auto save(const Data &data) -> Try {
    return repository.save(data)
        ? Try::ok()
        : Try::failed(ErrorCode::ApplicationFailed, "保存失败 / Save failed");
}

// 错误：抛出异常
auto parse(const std::string &input) -> ParsedData {
    if (!is_valid(input)) {
        throw std::invalid_argument("Invalid input format");
    }
    return parse_data(input);
}
```

### 3.2 错误传播

使用 `and_then` 串联多个可能失败的操作，使用 `map` 转换成功值：

```cpp
auto process(const std::string &input) -> Ret<Output> {
    return validate(input)
        .and_then([](auto &&validated) { return transform(validated); })
        .and_then([](auto &&transformed) { return save(transformed); });
}
```

或使用辅助函数 `run` 顺序执行：

```cpp
auto process(const std::string &input) -> Ret<Output> {
    return run(
        [&] { return validate(input); },
        [&](auto &&validated) { return transform(validated); },
        [&](auto &&transformed) { return save(transformed); }
    );
}
```

### 3.3 错误映射

使用 `map` 转换成功值，使用 `map_error` 转换错误，使用 `inspect_err` 在失败时执行副作用：

```cpp
auto result = parse(input)
    .map([](auto &&data) { return to_string(data); })
    .inspect_err([](const auto &error) {
        spdlog::error("Parse failed: {}", error.message);
    });
```

**跨错误码类型映射**：当需要将领域错误码映射回基础错误码时：

```cpp
auto result = solve(model)
    .map_error([](const Error<Bpp3dErrorCode> &e) {
        return Error<ErrorCode>(ErrorCode::ApplicationFailed, e.message);
    });
```

### 3.4 工厂函数

使用提供的工厂函数创建结果：

```cpp
// 成功 / Success
Ret<T>::ok(value)
Try::ok()

// 失败 / Failed
Ret<T>::failed(ErrorCode::IllegalArgument, "message")
Bpp3dRet<T>::failed(Bpp3dErrorCode::ItemTooLarge, "message")

// 致命 / Fatal
Ret<T>::fatal(ErrorCode::ApplicationError, "fatal message")
Ret<T>::fatal(std::vector<Error<ErrorCode>>{error1, error2})

// 警告 / Warning
ExRet<T>::warn(value, ErrorCode::Other, "warning message")
```

### 3.5 带关联值的错误

当错误需要携带额外上下文信息时，使用 `ExError`：

```cpp
auto parse_range(const std::string &input) -> Ret<ValueRange> {
    auto [min, max] = parse_pair(input);
    if (min > max) {
        return Ret<ValueRange>::failed(
            ExError<ErrorCode, std::pair<double, double>>(
                ErrorCode::IllegalArgument,
                "最小值大于最大值 / min exceeds max",
                std::make_pair(min, max)
            )
        );
    }
    return Ret<ValueRange>::ok(ValueRange(min, max));
}
```

### 3.6 延迟消息构造

当错误消息构造成本较高时，使用 `LazyError` 避免不必要的格式化：

```cpp
return Ret<T>::failed(
    LazyError<ErrorCode>(
        ErrorCode::ApplicationFailed,
        [&] { return fmt::format("处理失败：{} / Processing failed: {}", detail); }
    )
);
```

### 3.7 与 std::expected 的互操作

如果编译器支持 C++23 `std::expected`，可提供互操作转换：

```cpp
template<typename T, ErrorCodeTrait C, typename E>
auto to_expected(const Result<T, C, E> &result) -> std::expected<T, E>;

template<typename T, typename E, ErrorCodeTrait C = ErrorCode>
auto from_expected(const std::expected<T, E> &exp) -> Result<T, C>;
```

## 4. 禁止的模式

### 4.1 禁止抛出异常

```cpp
// 禁止
throw std::invalid_argument("...");
throw std::runtime_error("...");
throw std::logic_error("...");
throw std::out_of_range("...");
```

### 4.2 禁止忽略错误

```cpp
// 禁止：Result 未检查即丢弃
save(data);  // 警告：返回的 Try 被忽略

// 正确：显式处理或向上传播
auto _ = save(data);                    // 显式忽略
save(data).expect("save must succeed"); // 测试中可接受
[[maybe_unused]] auto r = save(data);   // 显式标注
```

### 4.3 禁止在 Result 处理中抛异常

```cpp
// 禁止
auto result = some_operation();
if (result.is_failed()) {
    throw std::runtime_error(result.error().message);
}
```

## 5. 允许的例外情况

### 5.1 测试代码

测试代码中可以使用异常来：
- 模拟失败场景
- 断言预期行为
- 测试 stub 实现

```cpp
// 测试中允许
auto method() -> Type override {
    throw std::runtime_error("stub");
}
```

### 5.2 外部库交互

与不支持 Result 模式的外部库交互时，可以在边界处捕获异常并转换为 Result：

```cpp
auto external_call() -> Ret<Response> {
    try {
        return Ret<Response>::ok(external_library.do_something());
    } catch (const ExternalException &e) {
        return Ret<Response>::failed(
            ErrorCode::ExternalFailed,
            fmt::format("外部调用失败 / External call failed: {}", e.what())
        );
    }
}
```

### 5.3 不可恢复的系统级错误

以下场景因系统约束保留 `throw` / `abort`，不属于迁移范围：

- **内存分配失败**：`std::bad_alloc` 在 OOM 时无法通过返回值恢复。
- **构造函数中的不变量违反**：当对象根本无法构造时，允许在构造函数中抛异常。
- **STL / 第三方库契约违反**：如 `std::vector::at()` 越界，属于编程错误而非业务错误。

## 6. ErrorCodeTrait 定义

```cpp
/// 错误码 trait / Error code trait
template<typename C>
concept ErrorCodeTrait = std::is_enum_v<C>
    && requires(C code) {
        { static_cast<std::underlying_type_t<C>>(code) } -> std::convertible_to<std::uint32_t>;
    };
```

## 7. ErrorCode 扩展

当现有错误码不足以表达错误类型时：

1. 首先检查是否可以复用基础 `ErrorCode`
2. 在相应模块定义领域错误码枚举（满足 `ErrorCodeTrait`）
3. 为该模块定义 `Result` / `Ret` 别名，绑定领域错误码
4. 确保错误码值不与现有模块冲突
5. 在模块 README 中记录错误码分配范围

## 8. 错误消息规范

- 使用简洁明了的中英双语消息
- 中文在前，英文在后，用 ` / ` 分隔
- 包含足够的上下文信息（如参数值、状态）
- 避免暴露内部实现细节
- 格式：`"操作失败：原因 / Operation failed: reason"`
- 使用 `fmt::format` 构建动态消息，避免字符串拼接
