# Canonical Function Schema

Finkit exposes the canonical function metadata registry as a versioned JSON
contract. This is the preferred input for future binding generators, CLI help,
documentation tooling, compatibility reports, and external introspection.

## Export the full schema

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

## Schema contract

The current schema identifier is:

```text
finkit.function.v1
```

This identifier is independent from the Finkit package version. A patch or
minor Finkit release can therefore keep the same metadata contract while
adding functions. Breaking JSON shape/meaning changes require a new schema
identifier.

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

## Planned consumers

The schema is intentionally language-neutral. Recommended consumers are:

```text
FunctionRegistry
      |
      v
FunctionApiSchema
      |
      +--> finkit-schema JSON
      |       |
      |       +--> Python signatures/docs
      |       +--> TypeScript declarations
      |       +--> Java wrappers
      |       +--> C# wrappers
      |       +--> Go wrappers
      |       +--> C metadata/header generation
      |
      +--> CLI/documentation introspection
```

Language-specific generators should own only marshaling, error translation,
loader/platform packaging, and idiomatic naming. Parameter defaults and
execution capability metadata should come from this schema.

## Stability rules

1. Canonical names are uppercase and stable once published.
2. Aliases are resolved case-insensitively but remain explicit in the JSON.
3. Function ordering is deterministic.
4. Alias/canonical collisions are rejected at registry construction time.
5. A schema consumer should validate `schema_version` before code generation.
6. Experimental runtime backends must not change the public metadata contract
   unless the corresponding capability field changes semantically.
