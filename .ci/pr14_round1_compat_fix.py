from pathlib import Path

path = Path("core/src/compute.rs")
text = path.read_text(encoding="utf-8")
old = '"factor plan is stale because dependencies changed for {name}"'
new = '"stale factor plan: dependencies changed for {name}"'
if text.count(old) != 1:
    raise SystemExit(f"compute.rs: expected one stale-plan message, found {text.count(old)}")
text = text.replace(old, new, 1)
old = 'if message.contains("factor plan is stale") && message.contains("target")'
new = 'if message.contains("stale factor plan") && message.contains("target")'
if text.count(old) != 2:
    raise SystemExit(f"compute.rs: expected two new regression assertions, found {text.count(old)}")
text = text.replace(old, new)
path.write_text(text, encoding="utf-8")
