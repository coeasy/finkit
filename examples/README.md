# Finkit 示例代码

本目录包含 Finkit 多语言示例代码，帮助用户快速上手。

## 目录结构

```
examples/
├── python_example.py      # Python 示例
├── nodejs_example.js      # Node.js 示例
├── java_example/
│   └── FinkitExample.java # Java 示例
├── go_example/
│   └── main.go            # Go 示例
└── README.md              # 本文件
```

## 运行示例

### Python

Use a wheel downloaded from the published GitHub Release, or build the Python
binding from source as described in `docs/installation.md`:

```bash
python -m pip install ./finkit-0.1.15-<matching-platform>.whl
python examples/python_example.py
```

### Node.js

The Node binding is currently built and packed from `ffi/node-binding`; no
public npm install path is advertised yet:

```bash
cd ffi/node-binding
npm ci
npm run build
npm test
cd ../..
node examples/nodejs_example.js
```

### Java

```bash
# 编译示例 (需要先构建 Java binding)
javac -cp dist/java/windows-x64/finkit-0.1.15.jar examples/java_example/FinkitExample.java

# 运行示例
java -cp dist/java/windows-x64/finkit-0.1.15.jar;examples/java_example FinkitExample
```

### Go

The Go binding is a nested source module. Build the native library first, then
run the example from the checkout:

```bash
cargo build -p finkit-go --release --locked
cd ffi/go-binding/go
go test ./...
cd ../../..
go run examples/go_example/main.go
```

## 示例内容

每个示例文件包含以下内容：

1. **基础指标计算**
   - SMA (简单移动平均)
   - EMA (指数移动平均)
   - RSI (相对强弱指数)
   - MACD (异同移动平均线)
   - 布林带

2. **OHLCV 数据分析**
   - ATR (平均真实波幅)
   - KDJ (随机指标)
   - ADX (平均趋向指数)
   - OBV (能量潮)
   - MFI (资金流量指数)

3. **K线形态识别**
   - 十字星
   - 锯子线
   - 吞没形态
   - 晨星/晚星

4. **交易信号生成**
   - 多指标综合分析
   - 买入/卖出信号生成

5. **完整交易分析**
   - 综合指标计算
   - 趋势判断
   - 动量分析
   - 交易建议

## 更多文档

- [快速入门指南](../docs/src/quickstart.md)
- [文档索引](../docs/README.md)
- [API 参考](../docs/api-reference.md)
- [开发指南](../docs/development.md)