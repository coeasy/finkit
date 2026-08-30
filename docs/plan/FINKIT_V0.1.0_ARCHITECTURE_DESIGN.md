# Finkit v0.1.0 Architecture Design

## Version

Current target version: v0.1.0

## Goal

Finkit v0.1.0 is the foundation release of a high-performance open financial indicator and factor computation engine.

Scope:

- Core calculation engine
- Technical indicators framework
- Python SDK
- Extensible architecture

Out of scope:

- Trading system
- Data provider
- Backtesting platform

## Architecture

```
finkit
├── core
│   ├── series
│   ├── candle
│   ├── memory
│   └── runtime
├── indicators
├── factors
├── formula
├── bindings
│   └── python
└── tests
```

## Core Engine

Recommended implementation:

- C++20 core
- SIMD optimization
- Zero-copy interface
- Streaming calculation support

## API Principle

Simple API:

```python
finkit.SMA(close, 20)
finkit.RSI(close, 14)
```

Advanced API:

```python
engine.register_indicator()
engine.calculate()
```

## Design Principles

- High performance
- Cross language
- Compatible with financial terminals
- Stable API
- Plugin extensibility
