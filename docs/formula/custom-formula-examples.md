# 自定义公式编写与执行示例

本文对应可直接运行的示例程序：

```bash
cargo run --release --example custom_formula_gallery   -p finkit   # 9 个自定义公式示例
cargo run --release --example formula_vs_talib         -p finkit   # 与 TA-Lib C 的效率对比
cargo run --release --example formula_surface_audit    -p finkit   # 能力面审计
```

效率对比若需要第四列（TA-Lib C），需启用特性：

```bash
cargo run --release --features talib-c --example formula_vs_talib -p finkit
```

## 1. 最小可用公式

```rust
let mut engine = FormulaEngine::new();
let mut ctx = make_ctx(120);
let signal = engine.eval(r#"
    MA5  := MA(CLOSE, 5);
    MA20 := MA(CLOSE, 20);
    CROSS(MA5, MA20)
"#, &mut ctx)?;
```

`:=` 绑定命名序列，最后一个表达式是公式的值。绑定在执行后仍可从
`ctx.variables` 读回，便于主机渲染中间曲线。

## 2. 注册可复用组件

```rust
engine.register_custom_formula("ZS", &["X", "N"], "(X - MA(X, N)) / STD(X, N)")?;
engine.register_custom_formula("MEAN_REV", &["X", "N"],
    "IF(ZS(X, N) < -2, 1, IF(ZS(X, N) > 2, -1, 0))")?;
let signal = engine.eval("MEAN_REV(CLOSE, 20)", &mut ctx)?;
```

组件是**表达式级宏**，在规划前展开到规范 AST，因此与内置函数共用同一套内核与快路径。
组件可以互相调用，也可以作用在任何序列上（`MEAN_REV(VOL, 20)`）。
**内置函数不可被遮蔽**——`register_custom_formula("ZSCORE", ...)` 会返回错误，
这是有意的保护。

## 3. 一次求值取回全部通道

```rust
let multi = engine.eval_multi(r#"
    DIF  := EMA(CLOSE, 12) - EMA(CLOSE, 26);
    DEA  := EMA(DIF, 9);
    HIST := (DIF - DEA) * 2;
    HIST
"#, &mut ctx)?;
// multi.final_value 主输出；multi.outputs 是 { "DIF", "DEA", "HIST" }
```

## 4. 控制流与条件

```text
RAW     := CROSS(MA(CLOSE, 5), MA(CLOSE, 20));
CONFIRM := CLOSE > REF(CLOSE, 1) AND VOL > MA(VOL, 20);
FIRST   := RAW AND NOT(REF(RAW, 1) OR REF(RAW, 2));
IF(FIRST AND CONFIRM, 1, 0)
```

## 5. 参数化模板

同一份公式文本换参数重复执行，无需重新解析结构：

```rust
for (fast, slow) in [(5, 20), (10, 30), (3, 8)] {
    let src = format!("CROSS(MA(CLOSE, {fast}), MA(CLOSE, {slow}))");
    let signal = engine.eval(&src, &mut ctx)?;
}
```

## 6. 增量流式求值

```rust
let mut stream = FormulaStatefulStream::from_source("RSI(CLOSE, 14)", FormulaDialect::AlphaTA)?;
let mut last = f64::NAN;
// 行布局固定为 [open, high, low, close, volume, amount]，未使用的槽位保持 NaN
stream.push_values_into(&row, &mut last)?;
```

状态拥有递推历史，因此逐 bar 推送是 O(1)，不必重跑全序列。
`checkpoint()` / `restore()` 可序列化（`to_json` / `from_json`），便于持久化。

## 7. 多方言

| 方言 | 写法 | 说明 |
|---|---|---|
| AlphaTA（默认） | `MA(CLOSE, 20)` | Finkit 规范式 |
| TongDaXin | `MA(CLOSE, 20)` | 通达信 |
| TongHuaShun | `MA(CLOSE, 20)` | 同花顺 |
| EastMoney | `MA(CLOSE, 20)` | 东方财富 |
| Pine v5 | `ta.sma(close, 20)` | TradingView 子集 |

引擎统一做传输层归一化（BOM、行尾），各入口行为不会分叉。

## 8. 绘图指令

```text
BUY := CROSS(MA(CLOSE, 5), MA(CLOSE, 20));
DRAWTEXT(BUY, LOW, "B");
DRAWICON(BUY, LOW, 1);
BUY
```

绘图指令写入 `ctx.draw_commands`，绘图主机无需二次扫描。

## 9. 编译一次、多次执行（扫描循环）

```rust
let compiled = engine.compile("RSI(CLOSE, 14)")?;
for symbol in symbols {
    let ctx = make_ctx(250);
    scores.push(engine.eval_last(&compiled, &ctx)?);
}
```

> `eval_last` 在本轮修复前有一个正确性缺陷：对**等长**上下文连续求值（正是扫描形态）时，
> 递归指标（RSI/ATR）会隔次返回 `NaN`。现已修复——缓存状态只在真正的一 bar 追加下复用，
> 否则用当前序列重新播种。详见 V4 计划 §44.17。

## 能力面

审计程序 `formula_surface_audit` 从**运行时函数表**导出：

- 内置公式函数 **452** 个
- TA-Lib 公开目录 **161 项，运行时注册 161 项（100% 覆盖）**
- 5 方言 · 4 执行模式（Interpreter / Plan / Bytecode / JIT）· 自定义组件 ·
  命名通道 · DRAW 指令 · 增量流式 · 零拷贝区间求值

## 效率对比结论

同进程交错测量（10k bars，中位数）：

| 列 | 含义 | 合计 |
|---|---|---:|
| formula | `FormulaEngine::eval` | 267.44 µs |
| native | 直接调用 Rust 指标函数 | 238.08 µs |
| talib-C | TA-Lib 0.8.x C ABI | 514.02 µs |

即：公式引擎相对原生直调仅 **1.12×** 开销，相对 TA-Lib C 为 **1.92×（更快）**。
100k 档分别为 1.41× 与 1.58×。

> 跨运行的绝对微秒数在本机有 ±20% 漂移并带运行顺序伪影（V4 §44.16），
> 因此上述比值一律取自**同进程、交错轮次**的配对测量。
