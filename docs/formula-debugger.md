# Formula Debugger Guide

This guide explains how to use the formula engine's debugging capabilities to analyze, diagnose, and optimize your technical analysis formulas.

## Table of Contents

- [Overview](#overview)
- [Getting Started](#getting-started)
- [Debug Mode Output](#debug-mode-output)
- [Single-Step Execution](#single-step-execution)
- [Variable Watching](#variable-watching)
- [Error Localization](#error-localization)
- [Execution Trace Analysis](#execution-trace-analysis)
- [Common Debugging Scenarios](#common-debugging-scenarios)
- [Performance Profiling](#performance-profiling)
- [Best Practices](#best-practices)

## Overview

The formula engine provides a comprehensive debugging system that allows you to:

- **Single-step execution** - Execute formulas step by step and inspect intermediate results
- **Variable watching** - Monitor variable values at each execution step
- **Error localization** - Pinpoint exactly where and why a formula fails
- **Execution tracing** - Get a complete trace of all operations performed
- **Performance profiling** - Measure execution time of each formula component

### Debug API

All FFI bindings expose the debug API:

| Language | Function |
|----------|----------|
| Python | `formula_eval_debug(source, open, high, low, close, volume)` |
| Node.js | `formulaEvalDebug(source, open, high, low, close, volume)` |
| Java | `formulaEvalDebug(source, open, high, low, close, volume)` |
| Go | `FormulaEvalDebug(source, open, high, low, close, volume)` |
| .NET | `FormulaEvalDebug(source, open, high, low, close, volume)` |
| C/C++ | `ta_formula_eval_debug(...)` |

## Getting Started

### Python Example

```python
import finkit as ta

open_prices = [10.0, 10.2, 10.1, 10.5, 10.3, 10.8, 11.0, 10.9, 11.2, 11.5]
high_prices = [10.5, 10.8, 10.6, 10.9, 10.7, 11.2, 11.3, 11.1, 11.5, 11.8]
low_prices = [9.8, 10.0, 9.9, 10.2, 10.1, 10.5, 10.7, 10.6, 10.9, 11.2]
close_prices = [10.3, 10.5, 10.2, 10.6, 10.4, 10.9, 11.1, 10.8, 11.3, 11.6]
volumes = [1000.0, 1200.0, 800.0, 1500.0, 1100.0, 1800.0, 2000.0, 1600.0, 2200.0, 2500.0]

# Debug a simple MACD formula
source = """
DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
DEA := EMA(DIF, 9);
MACD := 2 * (DIF - DEA);
CROSS(DIF, DEA)
"""

debug_result = ta.formula_eval_debug(
    source, open_prices, high_prices, low_prices, close_prices, volumes
)

print("Result:", debug_result["result"])
print("Debug Info:", debug_result["debug"])
```

## Debug Mode Output

The debug mode returns a structured result with two main sections:

```python
{
    "result": {
        "__result__": [0.0, 0.0, 0.0, ...],  # Final expression result
        "DIF": [0.12, 0.15, 0.18, ...],       # Intermediate variable DIF
        "DEA": [0.10, 0.12, 0.14, ...],       # Intermediate variable DEA
        "MACD": [0.04, 0.06, 0.08, ...]       # Intermediate variable MACD
    },
    "debug": {
        "steps": [                            # Execution steps
            "Parsed AST with 15 nodes",
            "Compiled bytecode: 28 instructions",
            "Evaluated DIF := EMA(CLOSE,12) - EMA(CLOSE,26)",
            "Evaluated DEA := EMA(DIF,9)",
            "Evaluated MACD := 2 * (DIF - DEA)",
            "Evaluated CROSS(DIF, DEA)"
        ],
        "variables": {                        # Variable metadata
            "DIF": {"type": "Array", "length": 10, "computed_at": 2},
            "DEA": {"type": "Array", "length": 10, "computed_at": 3},
            "MACD": {"type": "Array", "length": 10, "computed_at": 4}
        },
        "errors": []                          # Error information (if any)
    }
}
```

### Result Section

The `result` section contains all computed variables:

| Key | Description |
|-----|-------------|
| `__result__` | The final expression result (last line of formula) |
| `<var_name>` | Any intermediate variable declared with `:=` |

### Debug Section

The `debug` section contains execution metadata:

| Key | Description |
|-----|-------------|
| `steps` | Chronological list of execution steps |
| `variables` | Metadata about each variable (type, length, computation order) |
| `errors` | Any errors encountered during execution |

## Single-Step Execution

The debug mode inherently performs step-by-step execution. Each assignment and function call is tracked:

### Step Tracking

```python
source = """
MA5 := MA(CLOSE, 5);
MA20 := MA(CLOSE, 20);
DIFF := MA5 - MA20;
CROSS(MA5, MA20)
"""

result = Indicators.formula_eval_debug(source, open, high, low, close, volume)

for i, step in enumerate(result["debug"]["steps"]):
    print(f"Step {i+1}: {step}")
```

**Example Output**:
```
Step 1: Parsed AST with 12 nodes
Step 2: Compiled bytecode: 18 instructions
Step 3: Evaluated MA5 := MA(CLOSE, 5) - computed 10 values
Step 4: Evaluated MA20 := MA(CLOSE, 20) - computed 10 values
Step 5: Evaluated DIFF := MA5 - MA20 - computed 10 values
Step 6: Evaluated CROSS(MA5, MA20) - computed 10 values
```

### Inspecting Intermediate Values

```python
# Access intermediate variable values
ma5_values = result["result"]["MA5"]
ma20_values = result["result"]["MA20"]
diff_values = result["result"]["DIFF"]

for i in range(len(ma5_values)):
    print(f"Bar {i}: MA5={ma5_values[i]:.2f}, MA20={ma20_values[i]:.2f}, DIFF={diff_values[i]:.2f}")
```

## Variable Watching

### Variable Metadata

Each variable in the `debug.variables` dictionary contains:

```python
{
    "MA5": {
        "type": "Array",
        "length": 10,
        "computed_at": 1,    # Step number when computed
        "dependencies": ["CLOSE"]
    },
    "DIFF": {
        "type": "Array", 
        "length": 10,
        "computed_at": 3,
        "dependencies": ["MA5", "MA20"]
    }
}
```

### Watching Specific Variables

```python
# Check which variables were computed
for var_name, meta in result["debug"]["variables"].items():
    print(f"{var_name}: computed at step {meta['computed_at']}, "
          f"length={meta['length']}, depends on {meta['dependencies']}")
```

### Variable Dependency Analysis

```python
def get_dependency_chain(var_name, variables):
    """Recursively trace variable dependencies."""
    var_meta = variables.get(var_name, {})
    deps = var_meta.get("dependencies", [])
    if not deps:
        return [var_name]
    
    chain = [var_name]
    for dep in deps:
        chain.extend(get_dependency_chain(dep, variables))
    return chain

# Trace how __result__ was computed
chain = get_dependency_chain("__result__", result["debug"]["variables"])
print("Dependency chain:", " -> ".join(chain))
```

## Error Localization

### Error Types

The debugger categorizes errors into four types:

| Error Type | Python Exception | Description |
|------------|------------------|-------------|
| `ParseError` | `SyntaxError` | Formula syntax is invalid |
| `RuntimeError` | `RuntimeError` | Error during formula execution |
| `InvalidParameter` | `ValueError` | Invalid function parameters |
| `InsufficientData` | `ValueError` | Not enough data for computation |

### Error Information Structure

When an error occurs, the `debug.errors` list contains detailed information:

```python
{
    "result": {...},
    "debug": {
        "steps": ["Parsed AST...", "Evaluated MA5..."],
        "variables": {...},
        "errors": [
            {
                "type": "RuntimeError",
                "message": "Division by zero in expression: CLOSE / VOLUME",
                "step": 3,
                "expression": "CLOSE / VOLUME",
                "variables": {
                    "CLOSE": [10.3, 10.5, ...],
                    "VOLUME": [0.0, 1200.0, ...]  # First value is 0!
                }
            }
        ]
    }
}
```

### Handling Errors in Code

```python
def safe_formula_eval(source, open, high, low, close, volume):
    result = Indicators.formula_eval_debug(source, open, high, low, close, volume)
    
    errors = result["debug"]["errors"]
    if errors:
        for error in errors:
            print(f"Error at step {error['step']}: {error['type']}")
            print(f"  Message: {error['message']}")
            print(f"  Expression: {error['expression']}")
            
            # Show problematic variable values
            for var_name, values in error.get("variables", {}).items():
                print(f"  {var_name}: {values[:5]}...")
        return None
    
    return result["result"]
```

### Common Error Scenarios

#### 1. Division by Zero

```python
# Problematic formula
source = "CLOSE / VOLUME"

# If VOLUME contains zeros, you'll get:
# RuntimeError: Division by zero in expression: CLOSE / VOLUME
# Step: 1, Variables: VOLUME=[0.0, 1200.0, ...]
```

#### 2. Insufficient Data

```python
# Formula requiring 30 periods but only 10 data points provided
source = "MA30 := MA(CLOSE, 30); MA30"

# Error: InsufficientData: Need at least 30 data points, got 10
```

#### 3. Invalid Parameters

```python
# Negative period value
source = "MA(CLOSE, -5)"

# Error: InvalidParameter: Period must be positive, got -5
```

#### 4. Syntax Error

```python
# Missing closing parenthesis
source = "MA(CLOSE, 5"

# Error: ParseError: Unexpected end of input, expected ')'
```

## Execution Trace Analysis

The `steps` list provides a chronological trace of formula execution:

### Understanding Steps

```
Step 1: Parsed AST with 15 nodes
        -> Formula was successfully parsed into 15 AST nodes

Step 2: Compiled bytecode: 28 instructions
        -> AST was compiled into 28 bytecode instructions

Step 3: Evaluated MA5 := MA(CLOSE, 5) - computed 10 values
        -> First assignment executed, produced 10 output values

Step 4: Evaluated MA20 := MA(CLOSE, 20) - computed 10 values
        -> Second assignment executed

Step 5: Evaluated CROSS(MA5, MA20) - computed 10 values
        -> Final expression evaluated
```

### Performance Analysis from Trace

```python
# Analyze which steps took the most time (future feature)
for step in result["debug"]["steps"]:
    if "computed" in step:
        # Extract computation info
        print(step)
```

## Common Debugging Scenarios

### Scenario 1: Formula Returns All Zeros

```python
source = "MA5 := MA(CLOSE, 5); CROSS(MA5, MA5)"  # Always false!

result = Indicators.formula_eval_debug(source, open, high, low, close, volume)

# Debug output shows:
# Step 3: Evaluated CROSS(MA5, MA5) - computed 10 values
# Result: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

# Diagnosis: CROSS(x, x) is always false
# Fix: CROSS(MA5, MA20) instead
```

### Scenario 2: Unexpected NaN Values

```python
source = "RSV := (CLOSE - LLV(LOW, 9)) / (HHV(HIGH, 9) - LLV(LOW, 9)) * 100"

result = Indicators.formula_eval_debug(source, open, high, low, close, volume)

# Check for NaN in results
for name, values in result["result"].items():
    if any(v != v for v in values):  # NaN != NaN
        print(f"NaN detected in {name}")
        # Find which bar has NaN
        for i, v in enumerate(values):
            if v != v:
                print(f"  Bar {i}: HIGH={high[i]}, LOW={low[i]}")

# Diagnosis: HHV(9) == LLV(9) causes division by zero
# Fix: Add small epsilon: / (HHV - LLV + 0.0001)
```

### Scenario 3: Signal Appears Too Late

```python
source = """
DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
DEA := EMA(DIF, 9);
CROSS(DIF, DEA)
"""

result = Indicators.formula_eval_debug(source, open, high, low, close, volume)

# Check when signal first appears
for i, signal in enumerate(result["result"]["__result__"]):
    if signal == 1.0:
        print(f"First crossover at bar {i}")
        print(f"  DIF: {result['result']['DIF'][i]:.3f}")
        print(f"  DEA: {result['result']['DEA'][i]:.3f}")
        break

# Check previous bar values
print(f"Previous bar DIF: {result['result']['DIF'][i-1]:.3f}")
print(f"Previous bar DEA: {result['result']['DEA'][i-1]:.3f}")
```

### Scenario 4: Formula Runs Slowly

```python
# Complex formula with many repeated calculations
source = """
EMA12_1 := EMA(CLOSE, 12);
EMA26_1 := EMA(CLOSE, 26);
DIF1 := EMA12_1 - EMA26_1;

EMA12_2 := EMA(CLOSE, 12);  # Repeated!
EMA26_2 := EMA(CLOSE, 26);  # Repeated!
DIF2 := EMA12_2 - EMA26_2;

DIF1 + DIF2
"""

# Use debug mode to see redundant calculations
result = Indicators.formula_eval_debug(source, open, high, low, close, volume)

for step in result["debug"]["steps"]:
    print(step)

# Diagnosis: EMA(CLOSE, 12) and EMA(CLOSE, 26) computed twice
# Fix: Reuse EMA12_1 and EMA26_1
```

## Performance Profiling

### Comparing Execution Modes

```python
import time

source = "DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA := EMA(DIF, 9); MACD := 2 * (DIF - DEA); CROSS(DIF, DEA)"

# Time AST interpretation
start = time.time()
for _ in range(100):
    Indicators.formula_eval(source, open, high, low, close, volume)
ast_time = time.time() - start

# Time bytecode compilation
start = time.time()
for _ in range(100):
    Indicators.formula_eval_bytecode(source, open, high, low, close, volume)
bytecode_time = time.time() - start

# Time optimized execution
start = time.time()
for _ in range(100):
    Indicators.formula_eval_optimized(source, open, high, low, close, volume)
optimized_time = time.time() - start

print(f"AST Interpretation: {ast_time*10:.1f}ms per execution")
print(f"Bytecode Compilation: {bytecode_time*10:.1f}ms per execution")
print(f"Optimized Execution: {optimized_time*10:.1f}ms per execution")
```

### Formula Complexity Analysis

```python
# Use debug mode to analyze formula complexity
def analyze_formula_complexity(source):
    result = Indicators.formula_eval_debug(source, open, high, low, close, volume)
    
    num_variables = len(result["debug"]["variables"])
    num_steps = len(result["debug"]["steps"])
    num_errors = len(result["debug"]["errors"])
    
    print(f"Formula Analysis:")
    print(f"  Variables: {num_variables}")
    print(f"  Execution Steps: {num_steps}")
    print(f"  Errors: {num_errors}")
    
    return result

# Analyze different formulas
analyze_formula_complexity("MA(CLOSE, 5)")
analyze_formula_complexity("DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA := EMA(DIF, 9); CROSS(DIF, DEA)")
```

## Best Practices

### 1. Always Debug During Development

```python
# During development, use debug mode
result = Indicators.formula_eval_debug(source, open, high, low, close, volume)
if result["debug"]["errors"]:
    print("Errors found:", result["debug"]["errors"])
else:
    print("Formula executed successfully")
    print("Variables:", list(result["result"].keys()))

# In production, switch to optimized mode
result = Indicators.formula_eval_optimized(source, open, high, low, close, volume)
```

### 2. Validate Formula Syntax First

```python
# Quick syntax check before expensive debug run
is_valid = Indicators.formula_validate(source)
if not is_valid:
    print("Formula has syntax errors!")
    # Use debug mode to find the error
    result = Indicators.formula_eval_debug(source, open, high, low, close, volume)
    print(result["debug"]["errors"])
```

### 3. Check Intermediate Variables

```python
result = Indicators.formula_eval_debug(source, open, high, low, close, volume)

# Verify each intermediate variable makes sense
for var_name, values in result["result"].items():
    if var_name == "__result__":
        continue
    
    min_val = min(values)
    max_val = max(values)
    print(f"{var_name}: min={min_val:.4f}, max={max_val:.4f}")
    
    # Check for anomalies
    if min_val < -1e10 or max_val > 1e10:
        print(f"  WARNING: Extreme values in {var_name}")
```

### 4. Use Bytecode for Repeated Execution

```python
# When running the same formula multiple times
if num_executions > 5:
    # Use bytecode mode
    for _ in range(num_executions):
        result = Indicators.formula_eval_bytecode(source, open, high, low, close, volume)
else:
    # Use AST mode for one-off execution
    result = Indicators.formula_eval(source, open, high, low, close, volume)
```

### 5. Monitor Error Logs in Production

```python
# Even in production, periodically run debug mode to catch issues
if random.random() < 0.01:  # 1% sampling
    result = Indicators.formula_eval_debug(source, open, high, low, close, volume)
    if result["debug"]["errors"]:
        log_error("Formula execution error", result["debug"]["errors"])
```

---

## Troubleshooting

### Q: Formula returns all zeros

**A**: Check if the final expression is a condition that's never true. Use debug mode to inspect intermediate values and verify your logic.

### Q: Getting NaN or Infinity values

**A**: Look for division by zero or operations on empty arrays. Use debug mode to find which step produces NaN.

### Q: Formula runs too slowly

**A**: Switch to bytecode or optimized mode. Also check for redundant calculations that can be eliminated.

### Q: Signal appears one bar late

**A**: This is normal for crossover detection - CROSS detects when A crosses above B, which happens after the crossover occurs. Use REF() to look back if needed.

### Q: How to handle missing data?

**A**: Ensure you have enough data points for the formula's lookback period. Use debug mode's `InsufficientData` error to determine minimum required data length.
