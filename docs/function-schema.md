# Canonical Function and Terminal Schemas

Finkit exposes the canonical function metadata registry and declared formula-
terminal compatibility metadata as versioned JSON contracts. These are the
preferred inputs for future binding generators, CLI help, documentation tooling,
compatibility reports, and external introspection.

## Export the full function schema

After installing the `finkit-cli` Cargo package, run:

```bash
finkit-schema --compact
```

For a readable representation:

```bash
finkit-schema
```

Write the schema to a file:

```bash
finkit-schema --output dist/finkit-function-schema.json
```

## Export one function

Canonical names and compatibility aliases resolve through the same schema:

```bash
finkit-schema --function SMA
finkit-schema --function MA
finkit-schema --function BOLL --compact
```

A single-function response keeps the schema version next to the selected
function so generated SDK steps can reject incompatible metadata contracts.

## Export terminal compatibility metadata

Use the terminal schema when a CLI, binding, or documentation surface needs to
show the compatibility strength that the runtime actually declares:

```bash
finkit-schema --terminals
finkit-schema --terminals --compact
finkit-schema --terminals --output dist/finkit-terminal-schema.json
```

The terminal schema is intentionally conservative. It reports the stable
terminal id, canonical parser dialect, and compatibility level; it does not
claim that every function or drawing primitive from an external terminal is
implemented.

Current v0.1.15 declarations are:

| Terminal id | Canonical dialect | Compatibility |
| --- | --- | --- |
| `finkit` | `alpha_ta` | `native` |
| `tdx` | `tdx` | `common_subset` |
| `ths` | `ths` | `common_subset` |
| `eastmoney` | `eastmoney` | `common_subset` |
| `pine` | `pine` | `common_subset` |

The function and terminal contracts use independent schema identifiers:

```text
finkit.function.v1
finkit.formula-terminal.v1
```

Schema identifiers are independent from the Finkit package version. A patch or
minor Finkit release can therefore keep the same metadata shape while adding
functions or strengthening tested compatibility. Breaking JSON shape/meaning
changes require a new schema identifier.

## Function schema contract

Each canonical function exposes:

- `name`
- `aliases`
- `category`
- `input`
- `params` with name, value type, default, and constraint
- `outputs`
- `lookback`
- `streaming`
- `deterministic`
- `stateful`
- `effect`

### `input` values

`input` describes the series a call must supply, and the value is the contract:
a caller that passes exactly this shape gets correct numbers on every execution
path. The values are `series` (one numeric series), `hl` (high, low), `hlc`
(high, low, close), `hlcv` (high, low, close, volume), `ohlcv` (open, high,
low, close, volume) and `dynamic` (the formula expression decides). Parameters
listed in `params` always follow the series arguments, as literals.

`hl` was added because two-series indicators previously had no honest spelling
and were declared `hlc`. That told callers to pass a `close` no implementation
reads, so `ICHIMOKU_TENKAN(HIGH, LOW, CLOSE, 9)` computed a tenkan-sen over a
period of `CLOSE[0]` instead of `9`. A gate in
`core/tests/formula_registry_signature.rs` rebuilds the documented call from
this metadata and runs it through both execution paths, so the schema cannot
drift from the engine again.

### Parameters describe the operation surface

One registry describes two call surfaces, and `params` describes the wider one.
The operation and FFI entry points accept the full published list —
`finkit.bbands(real, timeperiod, nbdevup, nbdevdn, matype)` and
`finkit.stddev(close, timeperiod, nbdev)` — while a *formula* evaluates to a
single series and so takes a shorter list: `BBANDS(CLOSE, 20, 2.0)` and
`STDDEV(CLOSE, 14)`. The five functions where this applies are recorded with a
reason in the gate's `FORMULA_PARAM_SUBSET` list.

### Multi-output functions in a formula

`outputs` greater than one describes the operation surface, which returns every
leg (`BBANDS` returns `UPPERBAND`, `MIDDLEBAND`, `LOWERBAND`; `MACD` returns
`MACD`, `MACD_SIGNAL`, `MACD_HIST`). A *formula* expression evaluates to a
single series, so the same name inside a formula returns its primary leg — for
`BBANDS` that is the upper band, matching the long-standing `BOLL` convention of
`MA(CLOSE, N) + nbdev * STD(CLOSE, N)`. Use the dedicated `BOLLMID` / `BOLLDN`
names, or the multi-output operation, when you need the other legs.

The Rust source of truth remains `FunctionRegistry`. `FunctionApiSchema`
creates an owned deterministic snapshot, and `finkit-schema` serializes that
snapshot. Bindings should consume this contract instead of parsing Rust source
or maintaining a second set of defaults.

Terminal metadata is sourced from `FormulaTerminal::all()`, each terminal's
canonical dialect, and its explicit `CompatibilityLevel`. Golden fixtures under
`core/tests/fixtures/formula_compat` validate representative parser/semantic
behavior for the external subset adapters.

## Planned consumers

The schemas are intentionally language-neutral. Recommended consumers are:

```text
FunctionRegistry                 FormulaTerminal
      |                                |
      v                                v
FunctionApiSchema             terminal metadata
      |                                |
      +-------------+------------------+
                    |
                    v
             finkit-schema JSON
                    |
                    +--> Python signatures/docs
                    +--> TypeScript declarations
                    +--> Java wrappers
                    +--> C# wrappers
                    +--> Go wrappers
                    +--> C metadata/header generation
                    +--> compatibility UI/docs
```

Language-specific generators should own only marshaling, error translation,
loader/platform packaging, and idiomatic naming. Parameter defaults and
execution capability metadata should come from the canonical schema.

## Stability rules

1. Canonical function names are uppercase and stable once published.
2. Function aliases are resolved case-insensitively but remain explicit in JSON.
3. Function ordering and terminal discovery order are deterministic.
4. Alias/canonical collisions are rejected at registry construction time.
5. A consumer must validate the relevant `schema_version` before generation.
6. External terminals remain `common_subset` until stronger compatibility is
   supported and backed by golden coverage.
7. Experimental runtime backends must not change the public metadata contract
   unless the corresponding capability field changes semantically.
