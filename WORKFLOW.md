# AzaLoop WORKFLOW.md
#
# This file controls AzaLoop's runtime behavior for this repository.
# Settings here take priority over .aza/config.json.
# See https://github.com/azaloop/azaloop for full documentation.

# ── Parallelism ──────────────────────────────────────────────────────────────
max_parallel: 1            # Max concurrent stories (1–3)

# ── Approval ─────────────────────────────────────────────────────────────────
auto_approve: true         # true = skip human APPROVAL_GATE in full_auto mode

# ── Skip Patterns ────────────────────────────────────────────────────────────
# skip_patterns:
#   - "S-5*"               # Skip all stories matching S-5xx
#   - "WATCH-*"            # Skip watch-mode stories

# ── Model Overrides ──────────────────────────────────────────────────────────
# model_overrides:
#   default: "claude-3-5-sonnet-20241022"
#   "S-870": "gpt-4o"      # Use different model for specific story

# ── Loop Config ──────────────────────────────────────────────────────────────
# loop:
#   max_iterations: 100
#   max_retries_per_story: 3
#   full_auto: true
#   auto_commit: true
