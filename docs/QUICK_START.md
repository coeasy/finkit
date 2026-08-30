# Quick Start

5 分钟上手 AlphaTA（finkit）。详细安装见 [installation.md](./installation.md)。

## 1. 选择入口

| 语言 | 包名/方式 | 安装 |
|------|-----------|------|
| Rust | `finkit` | `cargo add finkit` / 或 `cargo add --path core` |
| Python | `alpha_ta`（PyO3）| `pip install alpha-ta` 或 `cd ffi/python-binding && maturin develop` |
| Node | `@alphata/core`（napi-rs）| `npm install @alphata/core` |
| Go | `github.com/coeasy/finkit/go/ta` | `go get github.com/coeasy/finkit/go/ta` |
| Java | `com.alphata`（JNI）| 编译 `.jar` 后引用 |
| .NET | `AlphaTA`（P/Invoke）| NuGet 包 |
| C/C++ | `alpha-ta-ffi` + `alpha_ta.h` | 链接动态库 |
| CLI | `alpha-ta-cli` | `cargo install --path cli` |

## 2. 第一个例子

### Rust
```rust
use alpha_ta_core::indicators;
use alpha_ta_core::streaming::{indicators::StreamingRsi, StreamingIndicator};

fn main() {
    let close = vec![44.0, 44.5, 45.0, 44.0, 43.5, 44.0, 44.5];
    let rsi = indicators::rsi(&close, 14).unwrap();   // 批量接口
    println!("{rsi:?}");

    let mut s = StreamingRsi::new(14);                 // 流式接口（O(1)/bar）
    for v in close { println!("{:?}", s.next(v)); }
}
```

### Python
```python
from alpha_ta import sma, macd

prices = [44.0, 44.5, 45.0, 44.0]
print(sma(prices, 3))
print(macd(prices, 12, 26, 9))
```

### CLI
```bash
alpha-ta-cli sma -i data.csv --period 14
alpha-ta-cli macd -i data.csv --fast 12 --slow 26 --signal 9
# 输入支持 CSV 文件或 stdin 管道，输出支持 plain / csv / json
```

## 3. 常用能力索引

- 批量指标（SMA/EMA/RSI/MACD/BOLL/ATR/KDJ……）：[参考 `core/src/indicators/`](../core/src/indicators/) 与 [API 参考](api-reference.md)。
- 流式 O(1) 指标（98 个）：[`core/src/streaming/`](../core/src/streaming/)。
- 公式引擎（TDX/THS/DZH/Pine 兼容）：见 [公式文档](formula/README.md)。
- 更多示例：`examples/` 与 `ffi/*-binding/examples/`。

## 4. 验证安装

```bash
cargo test -p finkit        # Rust 冒烟
make bench-vs-talib                # 与 TA-Lib C 对照跑分
```

需要更多？见 [BUILD_GUIDE.md](./BUILD_GUIDE.md)（构建）与 [WIKI.md](./WIKI.md)（文档索引）。