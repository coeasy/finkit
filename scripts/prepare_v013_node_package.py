#!/usr/bin/env python3
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
path = ROOT / "ffi/node-binding/package.json"
data = json.loads(path.read_text(encoding="utf-8"))
data["files"] = ["index.js", "index.mjs", "index.d.ts"]
path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
