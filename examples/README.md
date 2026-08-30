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

```bash
# 安装 Finkit
pip install finkit

# 运行示例
python examples/python_example.py
```

### Node.js

```bash
# 安装 Finkit
npm install finkit

# 运行示例
node examples/nodejs_example.js
```

### Java

```bash
# 编译示例 (需要先构建 Java binding)
javac -cp dist/java/windows-x64/finkit-0.1.0.jar examples/java_example/FinkitExample.java

# 运行示例
java -cp dist/java/windows-x64/finkit-0.1.0.jar;examples/java_example FinkitExample
```

### Go

```bash
# 安装 Finkit
go get github.com/coeasy/finkit/go/ta

# 运行示例
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

- [快速入门指南](../docs/QUICK_START.md)
- [完整 Wiki](../docs/WIKI.md)
- [API 参考](../docs/api-reference-zh.md)
- [构建指南](../docs/BUILD_GUIDE.md)