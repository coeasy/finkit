---
name: Trial feedback
about: Report anything that made the trial harder than it should have been
title: "[trial] "
labels: ["trial-feedback"]
---

<!--
This template is for the trial release. Two things matter most:

1. Which exact version and install path you used. "It did not work" is not
   actionable; "0.2.0, wheel from the v0.2.0 Release, Windows x86_64" is.
2. A snippet that reproduces it. If the snippet needs market data, say which
symbol/timeframe; do not paste proprietary data. For a v0.2.0 candidate, also
say whether you used a source checkout or a locally built artifact.
-->

## What I was trying to do

<!-- One or two sentences. The goal, not the symptom. -->

## What happened instead

<!-- Include the full error text, not a paraphrase of it. -->

## Version and install path

| | |
| --- | --- |
| Finkit version | <!-- e.g. 0.2.0 --> |
| Install path | <!-- e.g. wheel from the GitHub v0.1.15 Release / built from the v0.2.0 workspace / unpacked crate candidate --> |
| Language binding | <!-- Rust / Python / CLI / Node / Java / C / Go / .NET / WASM / none (docs only) --> |
| OS and architecture | <!-- e.g. Windows 11 x86_64, Ubuntu 22.04 x86_64, macOS 14 arm64 --> |
| Runtime | <!-- e.g. CPython 3.12.4, rustc 1.85.0, Node 20 --> |

## Reproduction

```python
# smallest snippet that shows the problem
```

<!--
If the input is a formula or a Pine script, paste it verbatim. If the problem
only appears with a particular data shape, describe the shape (length, leading
NaN run, whether the series is monotonic).
-->

## What I expected

<!--
For a numeric problem, say what number you expected and where that number comes
from (a TA-Lib result, a terminal's own output, a hand calculation). A claimed
mismatch without a reference value cannot be checked.
-->

## Anything else

<!--
Optional: workarounds you found, whether it blocked you completely, and which
part of the docs you were reading when you hit it.
-->
