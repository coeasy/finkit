# Formula Templates Guide

The core ships a library of **317 pre-built formula templates** across **12
categories**. The exhaustive, generated catalogue — one section per template with
its key, formula source and parameter ranges — lives in
[`docs/generated/formula-templates.md`](generated/formula-templates.md). That file
is produced from `core/src/formula/templates.rs` and is verified by CI, so it is
the only place a template list should be read from.

> This document previously claimed "67 templates in 8 categories" and documented
> 79 templates by hand, 16 of which named keys that do not exist. It is now a
> guide that links to the generated catalogue instead of duplicating it.

## Categories

| Category | Templates | What it covers |
|----------|-----------|----------------|
| `Strategy` | 99 | Multi-indicator entry/exit rules |
| `Pattern` | 45 | Candlestick and price-pattern triggers |
| `Oscillator` | 33 | RSI, KDJ, MACD, CCI, Williams %R, ROC |
| `Classic` | 27 | 通达信/同花顺 classics |
| `Volume` | 25 | Volume-price analysis, OBV, volume ratio |
| `Trend` | 21 | ADX, SAR, DMI, SuperTrend, Ichimoku |
| `TDXClassic` | 15 | 通达信经典 |
| `EMClassic` | 15 | 东方财富经典 |
| `MovingAverage` | 13 | MA/EMA crossovers, ribbons, bull/bear lines |
| `DZHMoneyFlow` | 11 | 大智慧资金流 |
| `THSSmartSelect` | 10 | 同花顺智能选股 |
| `FoxTrader` | 3 | 飞狐交易师 |

The generated catalogue is the source of truth for these numbers; the table above
is only a summary.

## Look up a template

The template *content* is the same everywhere, but the field names and the
miss-behaviour are not. Only Python flattens everything into one dictionary;
the native surfaces serialise the Rust `FormulaTemplate` struct directly, whose
source field is called `source`, not `formula`.

| Entry point | Missing key | Fields |
| --- | --- | --- |
| Rust `FormulaTemplates::get` | `None` | `name`, `description`, `category` (`TemplateCategory`), `source`, `parameters` |
| Python `formula_get_template` | raises `ValueError` | `name`, `category`, `description`, `formula`, `parameters` |
| Node `Indicators.formulaGetTemplate` | returns `null` | `name`, `description`, `category`, `source` |
| WASM `formula_get_template` | throws | `name`, `description`, `category`, `source` |
| Go / .NET / Java `ta_formula_get_template` | plain-text message, **not** JSON | JSON of the Rust struct: `name`, `description`, `category`, `source` |

Two consequences worth knowing before you write code against this:

1. **`parameters` is a Rust/Python-only field.** The Rust struct carries
   `Vec<(name, default, min, max)>`, Python exposes it as
   `{param: {default, min, max}}`, and the Go/.NET/Java JSON drops it entirely
   (`#[serde(skip)]` on the field). Node and WASM never carry it. For those
   languages read the ranges from
   [`docs/generated/formula-templates.md`](generated/formula-templates.md).
2. **`formula_list_categories` is not uniform either.** Python returns
   `[{"category", "count"}]`; Node and WASM return `[String]`; Go/.NET/Java
   return a JSON array of category names. Only Python gives you the counts.

### Rust

```rust
use finkit::formula::{FormulaTemplates, TemplateCategory};

let templates = FormulaTemplates::new();

// One template by key — `None` when the key is unknown.
let template = templates.get("ma_cross").expect("known template");
println!("{} [{}]", template.name, template.description);
println!("{}", template.source);

// Everything in one category.
for template in templates.get_by_category(&TemplateCategory::Oscillator) {
    println!("{}", template.name);
}

// Free-text search over name, description and category.
for template in templates.search("金叉") {
    println!("{}", template.name);
}

// The whole catalogue, and the category list itself.
println!("{} templates", templates.list_all().len());
println!("{} categories", FormulaTemplates::categories().len());
```

### Python

```python
import finkit as ta

# One template by key. Raises ValueError when the key is unknown.
template = ta.formula_get_template("ma_cross")
print(template["name"], template["category"])
print(template["formula"])
for param, spec in template["parameters"].items():
    print(param, spec["default"], spec["min"], spec["max"])

# Search by keyword.
for hit in ta.formula_search_templates("金叉"):
    print(hit["name"])

# Per-category counts.
for entry in ta.formula_list_categories():
    print(entry["category"], entry["count"])
```

### Node.js

```javascript
const { Indicators } = require('finkit');

// Returns the template object, or `null` when the key is unknown.
const template = Indicators.formulaGetTemplate("ma_cross");
if (template) {
  console.log(template.name, template.source); // note: `source`, not `formula`
}

// Both of these return arrays of plain strings.
console.log(Indicators.formulaSearchTemplates("金叉").map((t) => t.name));
console.log(Indicators.formulaListCategories());
```

### WebAssembly

```javascript
import init, { formula_get_template, formula_search_templates } from './finkit_wasm.js';

await init();
const template = formula_get_template("ma_cross"); // throws when unknown
console.log(template.name, template.source);       // `source`, not `formula`
console.log(formula_search_templates("金叉").map((t) => t.name));
```

### Coverage by language

Template lookup is **not** part of the C ABI — it is only exported by the
higher-level bindings. The table records exactly what each language exposes
today; "—" means the language has no template lookup and callers should read the
generated catalogue (or keep their own copy of the key → formula mapping).

| Operation | Python | Node.js | Go | .NET (native) | Java | WASM | C | iOS | Android |
|---|---|---|---|---|---|---|---|---|---|
| Get one | `formula_get_template` | `formulaGetTemplate` | `ta_formula_get_template` | `ta_formula_get_template` | `Indicators.formulaGetTemplate` | `formula_get_template` | — | — | — |
| Search | `formula_search_templates` | `formulaSearchTemplates` | `ta_formula_search_templates` | `ta_formula_search_templates` | `Indicators.formulaSearchTemplates` | `formula_search_templates` | — | — | — |
| Categories | `formula_list_categories` | `formulaListCategories` | `ta_formula_list_categories` | `ta_formula_list_categories` | `Indicators.formulaListCategories` | `formula_list_categories` | — | — | — |

Notes on the native (non-managed) rows:

- The Go, .NET and Java rows return a NUL-terminated JSON document. Go and .NET
  release it with `ta_free_string`; Java copies it into a `String` before
  returning.
- **A missing key is not an error document there.** `ta_formula_get_template`
  writes the plain text `template '<name>' not found` into the returned string.
  Go's `FormulaGetTemplate` wrapper detects this (`json.Valid` fails) and
  converts it into a Go `error`; a .NET caller P/Invoking the symbol directly
  has to make the same check itself.
- The .NET column names the raw `ta_*` symbol — the C# wrapper in
  `ffi/dotnet-binding/src/Finkit/Indicators.cs` does not yet surface a
  `FormulaGetTemplate` method, so a .NET caller must P/Invoke the symbol
  directly. This is a known coverage gap, not a broken link: template keys are
  stable strings and `docs/generated/formula-templates.md` lists them all.
- C, iOS and Android deliberately expose no template lookup. Their formula
  surface is the `ta_formula_*_contract_json` family; to use a template from
  those languages, read its `formula` source from
  [`docs/generated/formula-templates.md`](generated/formula-templates.md) and
  pass it to the ordinary eval entry point.

See [`docs/language-bindings.md`](language-bindings.md) for the full
per-language export list.

## Evaluate a template

A template is a convenience wrapper: its `formula` field is ordinary AlphaTA
source, so any evaluation entry point accepts it. Two things are worth knowing:

1. **Parameters are not substituted for you.** `eval_template` applies the
   template's declared *defaults*. To use other values, read `parameters`, build
   a `ParamValues` map and call `eval_with_params`.
2. **`eval_template` is a tree-backend entry point.** Under
   `FormulaExecutionMode::Plan` it returns `FormulaError::BackendUnsupported`
   instead of silently walking the tree — evaluate the template's `formula`
   string with `eval` instead. See
   [`docs/formula-runtime-contract.md`](formula-runtime-contract.md).

```rust
let templates = FormulaTemplates::new();
let template = templates.get("ma_cross").expect("known template");

let mut engine = FormulaEngine::new();
let values = engine.eval_template("ma_cross", &mut ctx)?; // defaults applied
let custom = engine.eval_with_params(&template.source, &mut ctx, &params)?;
```

## Register a custom template

`FormulaEngine::register_custom_formula` handles user-defined *components*;
templates are a fixed catalogue. To add one, extend `init_builtin_templates()` in
`core/src/formula/templates.rs` and re-run:

```bash
python scripts/gen_ssot_docs.py --generate
```

CI's `python scripts/gen_ssot_docs.py --check` fails until the generated
catalogue is regenerated, so a new template cannot ship undocumented.

## Related documentation

- [Generated template catalogue](generated/formula-templates.md) — all 317 templates
- [Formula language reference](formula.md)
- [Formula runtime contract](formula-runtime-contract.md)
- [Function schema](function-schema.md)
