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

Current v0.1.2 declarations are:

| Terminal id | Canonical dialect | Compatibility |
| --- | --- | --- |
| `finkit` | `alpha_ta` | `native` |
| `tdx` | `alpha_ta` | `common_subset` |
| `ths` | `alpha_ta` | `common_subset` |
| `eastmoney` | `alpha_ta` | `common_subset` |
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
