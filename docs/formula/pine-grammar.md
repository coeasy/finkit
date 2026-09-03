# Pine Script v5 文法规范

本文档描述 Finkit Pine 方言支持的 **Pine Script v5 语法子集**，EBNF 由 `core/src/formula/pine/grammar.pest` 导出。

实现文件：

- 文法：`core/src/formula/pine/grammar.pest`
- 解析器：`core/src/formula/pine/parser.rs`
- AST 映射：`core/src/formula/pine/ast_mapper.rs`

---

## 概述

Finkit Pine 方言是 TradingView Pine Script v5 的**有意裁剪子集**，目标：

1. 解析常见 `indicator()` 脚本
2. 将 `ta.*` / `math.*` 内置函数映射到 Finkit 公式函数
3. 支持 bar-by-bar series 求值语义（`core/src/formula/pine/runtime.rs`）

**不支持**完整 Pine 生态（策略、库、UDT、完整绘图对象等）。详见 [兼容矩阵](../generated/pine-compatibility.md)。

---

## EBNF 文法

符号约定：

| 符号 | 含义 |
|------|------|
| `?` | 可选（0 或 1 次） |
| `*` | 重复（0 或更多） |
| `+` | 重复（1 或更多） |
| `\|` | 选择 |

### 顶层结构

```ebnf
program          = program_item*
program_item     = version_annotation | declaration | statement
version_annotation = "//" "@version=" version_num
version_num      = DIGIT+
```

**示例：**

```pine
//@version=5
indicator("My Indicator")
```

### 声明

```ebnf
declaration      = indicator_decl | study_decl | function_decl | var_decl | input_decl

indicator_decl   = "indicator" "(" string ("," indicator_arg)* ")"
study_decl       = "study" "(" string ("," indicator_arg)* ")"
indicator_arg    = identifier "=" expression

var_decl         = var_kw type_qualifier? identifier "=" expression
var_kw           = "var" | "varip"

input_decl       = "input" "(" expression ("," input_arg)* ")"
                 | "input." input_type "(" expression ("," input_arg)* ")"
input_type       = "int" | "float" | "bool" | "string" | "color" | "source"
input_arg        = string | identifier "=" expression

function_decl    = identifier "(" param_list? ")" "=>" (expression | block)
param_list       = identifier ("," identifier)*
```

### 语句

```ebnf
block            = statement (NEWLINE+ statement)*
statement        = assignment | if_stmt | for_stmt | while_stmt
                 | plot_call | hline_call | fill_call
                 | function_call_stmt | expression

assignment       = identifier assign_op expression
assign_op        = ":=" | "="

if_stmt          = "if" expression NEWLINE+ block ("else" NEWLINE+ block)?
for_stmt         = "for" identifier "=" expression "to" expression ("by" expression)? NEWLINE+ block
while_stmt       = "while" expression NEWLINE+ block

plot_call        = "plot" "(" plot_args ")"
hline_call       = "hline" "(" hline_args ")"
fill_call        = "fill" "(" fill_args ")"
```

### 表达式

```ebnf
expression       = ternary
ternary          = logical_or ("?" expression ":" expression)?
logical_or       = logical_and ("or" logical_and)*
logical_and      = comparison ("and" comparison)*
comparison       = addition (comp_op addition)?
comp_op          = "==" | "!=" | ">=" | "<=" | ">" | "<"
addition         = multiplication (add_op multiplication)*
add_op           = "+" | "-"
multiplication   = unary (mul_op unary)*
mul_op           = "*" | "/" | "%"
unary            = "not" unary | "-" unary | postfix
postfix          = primary (index_access)*
index_access     = "[" expression "]"
```

### 基本元素

```ebnf
primary          = function_call | number | string | na_literal
                 | variable | "(" expression ")"

function_call    = qualified_call | simple_call
qualified_call   = namespace "." identifier "(" arg_list? ")"
simple_call      = identifier "(" arg_list? ")"
namespace        = "ta" | "math" | "request" | "color" | "str"
                 | "array" | "line" | "label" | "box"
arg_list         = expression ("," arg_item)*
arg_item         = identifier "=" expression | expression

na_literal       = "na"
variable         = identifier | builtin_var
builtin_var      = "open" | "high" | "low" | "close" | "volume" | "time"
                 | "hl2" | "hlc3" | "ohlc4"

type_qualifier   = "series" | "simple" | "const" | "input"
identifier       = ALPHA (ALPHANUM | "_")*   (* 非关键字 *)
number           = DIGIT+ ("." DIGIT+)?
string           = "\"" (!"\"" ANY)* "\""
```

### 注释

```ebnf
line_comment     = "//" (!"\n" ANY)*
block_comment    = "/*" (!"*/" ANY)* "*/"
```

---

## 支持的语法子集

### ✅ 已支持

| 类别 | 语法 |
|------|------|
| 版本注解 | `//@version=5` |
| 指标声明 | `indicator()`, `study()` |
| 输入 | `input()`, `input.int()` 等 |
| 赋值 | `x = expr`, `x := expr` |
| 控制流 | `if/else`, `for`, `while` |
| 绘图语句 | `plot()`, `hline()`, `fill()`（解析级） |
| 运算符 | 算术、比较、逻辑、`? :` 三元 |
| 历史引用 | `close[1]`（索引访问语法） |
| 内置变量 | OHLCV、`hl2`/`hlc3`/`ohlc4` |
| 命名空间调用 | `ta.*`, `math.*`, `request.security` |
| 用户函数 | `fn(x) => expr`（映射为占位调用） |
| 注释 | `//` 行注释, `/* */` 块注释 |

### ⚠️ 部分支持

| 类别 | 限制 |
|------|------|
| `ta.*` 内置 | 仅映射表中的函数（见 builtin_table.rs） |
| `var`/`varip` | 可解析；跨 bar 持久化语义不完整 |
| Series 历史 `[n]` | 语法可解析；运行时覆盖有限 |
| `plot` 样式参数 | `color=`, `style=` 等作为表达式解析，不执行 |
| `request.security` | 映射为 SECURITY；不重绘语义未实现 |
| `color.*` 命名空间 | 文法预留；颜色运算未完整实现 |

### ❌ 不支持

| 类别 | 示例 |
|------|------|
| 策略入口 | `strategy()` |
| 警报 | `alertcondition()` |
| 自定义类型 | `type Point`, UDT 方法 |
| 库 | `import` / `library` |
| 绘图对象 | `line.new()`, `label.new()`, `box.new()` |
| 数组运行时 | `array.new()`, `array.push()` |
| 元信息变量 | `syminfo.tickerid`, `timeframe.period` |
| Repaint 语义 | `barmerge.lookahead_*`, 安全函数重绘 |

---

## 与 Finkit 公式引擎的关系

Pine 脚本经以下管线进入 Finkit 执行：

```
Pine 源码 → parse_pine() → PineAst
         → map_pine_to_Finkit() → AstNode
         → FormulaExecutor / BytecodeVM
```

方言选择通过 `parse_formula_with_dialect(source, FormulaDialect::Pine)` 入口。

---

## 相关文档

- [Finkit 公式文法（TDX 方言）](./grammar.md)
- [Pine 兼容矩阵](../generated/pine-compatibility.md)
- [Pine → Finkit 迁移指南](../migration/pine-to-alphata.md)
